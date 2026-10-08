use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    MMIO_CONSOLE,
    drivers::{
        console::{self, ConsoleWriter},
        disk::Disk,
    },
};

pub struct Ext2fs {
    disk: Disk,
    block_size: u32,
    inode_size: u16,
    inodes_per_group: u32,
    block_group_count: u32,
}

pub struct Superblock {
    pub s_inodes_count: u32,
    pub s_blocks_count: u32,
    pub s_r_blocks_count: u32,
    pub s_free_blocks_count: u32,
    pub s_free_inodes_count: u32,
    pub s_first_data_block: u32,
    pub s_log_size: u32,
    pub s_log_frag_size: u32,
    pub s_blocks_per_group: u32,
    pub s_frags_per_group: u32,
    pub s_inodes_per_group: u32,
    pub s_mtime: u32,
    pub s_wtime: u32,
    pub s_mnt_count: u16,
    pub s_max_mnt_count: u16,
    pub s_magic: u16,
    pub s_state: u16,
    pub s_errors: u16,
    pub s_minor_rev_level: u16,
    pub s_lastcheck: u32,
    pub s_checkinterval: u32,
    pub s_creator_os: u32,
    pub s_rev_level: u32,
    pub s_def_resuid: u16,
    pub s_def_resgid: u16,
    pub s_first_ino: u32,
    pub s_inode_size: u16,
    pub s_block_group_nr: u16,
    pub s_feature_compat: u32,
    pub s_feature_incompat: u32,
    pub s_feature_ro_compat: u32,
    pub s_uuid: u128,
    pub s_volume_name: u128,
    pub s_last_mounted: [u8; 64],
    pub s_algo_bitmap: u32,
    pub s_prealloc_blocks: u8,
    pub s_prealloc_dir_blocks: u8,
    pub s_journal_uuid: u128,
    pub s_journal_inum: u32,
    pub s_journal_dev: u32,
    pub s_last_orphan: u32,
    pub s_hash_seed: u128,
    pub s_def_hash_version: u8,
    pub s_default_mount_options: u32,
    pub s_first_meta_bg: u32,
}

pub struct BlockGroupDescriptor {
    pub bg_block_bitmap: u32,
    pub bg_inode_bitmap: u32,
    pub bg_inode_table: u32,
    pub bg_free_blocks_count: u16,
    pub bg_free_inodes_count: u16,
    pub bg_used_dirs_count: u16,
}

pub struct InodeBlockPointers {
    pub direct_pointers: [u32; 12],
    pub indirect_pointer: u32,
    pub double_indirect_pointer: u32,
    pub triple_indirect_pointer: u32,
}

pub struct Inode {
    pub i_mode: u16,
    pub i_uid: u16,
    pub i_size: u32,
    pub i_atime: u32,
    pub i_ctime: u32,
    pub i_mtime: u32,
    pub i_dtime: u32,
    pub i_gid: u16,
    pub i_links_count: u16,
    pub i_blocks: u32,
    pub i_flags: u32,
    pub i_osd1: u32,
    pub i_block: InodeBlockPointers,
    pub i_generation: u32,
    pub i_file_acl: u32,
    pub i_dir_acl: u32,
    pub i_faddr: u32,
    pub i_osd2: [u8; 12],
}

pub struct Directory {
    pub inode: u32,
    pub rec_len: u16,
    pub name_len: u8,
    pub file_type: u8,
    pub name: String,
}

pub struct File {
    pub directory: Directory,
    pub ext2fs: Ext2fs,
    pub inode: Inode,
    pub current_block: u32,
    pub bytes_read: u64,
}

pub struct PointerBlock {
    pub pointers: [u32; 256],
}

fn read_hword(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn read_word(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn read_qword(data: &[u8], offset: usize) -> u128 {
    u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap())
}

impl InodeBlockPointers {
    fn new(block_pointers: [u8; 60]) -> InodeBlockPointers {
        let mut direct_pointers: [u32; 12] = [0; 12];
        for i in 0..12 {
            direct_pointers[i] = read_word(&block_pointers, 4 * i);
        }
        InodeBlockPointers {
            direct_pointers,
            indirect_pointer: read_word(&block_pointers, 12 * 4),
            double_indirect_pointer: read_word(&block_pointers, 13 * 4),
            triple_indirect_pointer: read_word(&block_pointers, 14 * 4),
        }
    }
}

impl Inode {
    fn from_block(block: Vec<u8>, offset: usize) -> Inode {
        Inode {
            i_mode: read_hword(&block, 0 + offset),
            i_uid: read_hword(&block, 2 + offset),
            i_size: read_word(&block, 4 + offset),
            i_atime: read_word(&block, 8 + offset),
            i_ctime: read_word(&block, 12 + offset),
            i_mtime: read_word(&block, 16 + offset),
            i_dtime: read_word(&block, 20 + offset),
            i_gid: read_hword(&block, 24 + offset),
            i_links_count: read_hword(&block, 26 + offset),
            i_blocks: read_word(&block, 28 + offset),
            i_flags: read_word(&block, 32 + offset),
            i_osd1: read_word(&block, 36 + offset),
            i_block: InodeBlockPointers::new(
                block[(40 + offset)..(100 + offset)]
                    .try_into()
                    .expect("Slice with incorrect length"),
            ),
            i_generation: read_word(&block, 100 + offset),
            i_file_acl: read_word(&block, 104 + offset),
            i_dir_acl: read_word(&block, 108 + offset),
            i_faddr: read_word(&block, 112 + offset),
            i_osd2: block[(116 + offset)..(128 + offset)]
                .try_into()
                .expect("Slice with incorrect length"),
        }
    }
}

impl BlockGroupDescriptor {
    fn from_block(block: Vec<u8>, offset: usize) -> BlockGroupDescriptor {
        BlockGroupDescriptor {
            bg_block_bitmap: read_word(&block, 0 + offset),
            bg_inode_bitmap: read_word(&block, 4 + offset),
            bg_inode_table: read_word(&block, 8 + offset),
            bg_free_blocks_count: read_hword(&block, 12 + offset),
            bg_free_inodes_count: read_hword(&block, 14 + offset),
            bg_used_dirs_count: read_hword(&block, 16 + offset),
        }
    }
}

impl Superblock {
    pub fn from_block(block: Vec<u8>) -> Superblock {
        Superblock {
            s_inodes_count: read_word(&block, 0),
            s_blocks_count: read_word(&block, 4),
            s_r_blocks_count: read_word(&block, 8),
            s_free_blocks_count: read_word(&block, 12),
            s_free_inodes_count: read_word(&block, 16),
            s_first_data_block: read_word(&block, 20),
            s_log_size: read_word(&block, 24),
            s_log_frag_size: read_word(&block, 28),
            s_blocks_per_group: read_word(&block, 32),
            s_frags_per_group: read_word(&block, 36),
            s_inodes_per_group: read_word(&block, 40),
            s_mtime: read_word(&block, 44),
            s_wtime: read_word(&block, 48),
            s_mnt_count: read_hword(&block, 52),
            s_max_mnt_count: read_hword(&block, 54),
            s_magic: read_hword(&block, 56),
            s_state: read_hword(&block, 58),
            s_errors: read_hword(&block, 60),
            s_minor_rev_level: read_hword(&block, 62),
            s_lastcheck: read_word(&block, 64),
            s_checkinterval: read_word(&block, 68),
            s_creator_os: read_word(&block, 72),
            s_rev_level: read_word(&block, 76),
            s_def_resuid: read_hword(&block, 80),
            s_def_resgid: read_hword(&block, 82),
            s_first_ino: read_word(&block, 84),
            s_inode_size: read_hword(&block, 88),
            s_block_group_nr: read_hword(&block, 90),
            s_feature_compat: read_word(&block, 92),
            s_feature_incompat: read_word(&block, 96),
            s_feature_ro_compat: read_word(&block, 100),
            s_uuid: read_qword(&block, 104),
            s_volume_name: read_qword(&block, 120),
            s_last_mounted: block[136..200]
                .try_into()
                .expect("Slice with incorrect length"),
            s_algo_bitmap: read_word(&block, 200),
            s_prealloc_blocks: block[204],
            s_prealloc_dir_blocks: block[205],
            s_journal_uuid: read_qword(&block, 208),
            s_journal_inum: read_word(&block, 224),
            s_journal_dev: read_word(&block, 228),
            s_last_orphan: read_word(&block, 232),
            s_hash_seed: read_qword(&block, 236),
            s_def_hash_version: block[252],
            s_default_mount_options: read_word(&block, 256),
            s_first_meta_bg: read_word(&block, 260),
        }
    }
}

impl PointerBlock {
    pub fn from_block(block: Vec<u8>) -> PointerBlock {
        assert_eq!(block.len(), 1024);
        let mut pointers: [u32; 256] = [0; 256];

        for i in 0..256 {
            pointers[i] = read_word(&block, i * 4);
        }

        PointerBlock { pointers }
    }
}

impl File {
    pub fn from_directory(directory: Directory, ext2fs: Ext2fs) -> Option<File> {
        let inode = ext2fs.get_inode(directory.inode);

        if inode.i_mode & 0x8000 != 0x8000 {
            return None;
        }

        Some(File {
            directory,
            ext2fs,
            inode,
            current_block: 0,
            bytes_read: 0,
        })
    }

    pub fn read_bytes(&mut self, bytes: u64) -> Option<Vec<u8>> {
        // let mut buffer: Vec<u8> = Vec::new();
        // let remaining_bytes = bytes;
        // buffer.resize(bytes as usize, 0);
        //
        // while let block = Some(self.next_block())
        //     && remaining_bytes != 0
        // {
        //
        // }
        //
        // Some(buffer)
        todo!()
    }

    pub fn read_all(&mut self) -> Option<Vec<u8>> {
        let mut buffer: Vec<u8> = Vec::new();
        let mut remaining_bytes = self.inode.i_size;
        // buffer.resize(self.inode.i_size as usize, 0);
        buffer.reserve(self.inode.i_size as usize);

        while remaining_bytes != 0 {
            let to_copy = if remaining_bytes < 1024 {
                remaining_bytes
            } else {
                1024
            };

            let block = self.next_block()?;
            self.current_block += 1;

            for i in 0..to_copy {
                buffer.push(block[i as usize]);
            }

            remaining_bytes -= to_copy;
        }

        Some(buffer)
    }

    fn next_block(&mut self) -> Option<Vec<u8>> {
        // 0..12 Direct
        // 12..268 Indirect
        // 268..65804 Double
        // 65804..16843020 Triple
        let block_number = match self.current_block {
            0..12 => self.inode.i_block.direct_pointers[self.current_block as usize],
            12..268 => {
                let direct_pointers_block = PointerBlock::from_block(
                    self.ext2fs
                        .get_blocks(self.inode.i_block.indirect_pointer, 1),
                );
                let direct_index = self.current_block as usize - 12;
                direct_pointers_block.pointers[direct_index]
            }
            268..65804 => {
                let indirect_pointers_block = PointerBlock::from_block(
                    self.ext2fs
                        .get_blocks(self.inode.i_block.double_indirect_pointer, 1),
                );
                let indirect_index = (self.current_block as usize - 268) / 256;
                let direct_pointers_block = PointerBlock::from_block(
                    self.ext2fs
                        .get_blocks(indirect_pointers_block.pointers[indirect_index], 1),
                );
                let direct_index = (self.current_block as usize - 12) % 256;
                direct_pointers_block.pointers[direct_index]
            }
            65804..16843020 => {
                let double_indirect_pointers_block = PointerBlock::from_block(
                    self.ext2fs
                        .get_blocks(self.inode.i_block.triple_indirect_pointer, 1),
                );
                let double_indirect_index = (self.current_block as usize - 65804) / 65536;
                let indirect_pointers_block = PointerBlock::from_block(self.ext2fs.get_blocks(
                    double_indirect_pointers_block.pointers[double_indirect_index],
                    1,
                ));
                let indirect_index = ((self.current_block as usize - 65804) / 256) % 256;
                let direct_pointers_block = PointerBlock::from_block(
                    self.ext2fs
                        .get_blocks(indirect_pointers_block.pointers[indirect_index], 1),
                );
                let direct_index = (self.current_block as usize - 12) % 256;
                direct_pointers_block.pointers[direct_index]
            }
            _ => {
                return None;
            }
        };
        Some(self.ext2fs.get_blocks(block_number, 1))
    }
}

impl Directory {
    pub fn from_block(block: &Vec<u8>, offset: usize) -> Option<Directory> {
        if offset + 8 > 1024 {
            return None;
        }

        let inode = read_word(block, 0 + offset);
        let rec_len = read_hword(block, 4 + offset);
        let name_len = block[6 + offset];
        let file_type = block[7 + offset];

        if rec_len == 0 || name_len == 0 {
            return None;
        }

        let name_start = offset + 8;
        let name = String::from_utf8_lossy(&block[name_start..(name_start + name_len as usize)])
            .into_owned();

        Some(Directory {
            inode,
            rec_len,
            name_len,
            file_type,
            name,
        })
    }

    pub fn vec_from_block(block: &Vec<u8>) -> Vec<Directory> {
        let mut offset: usize = 0;
        let mut directories: Vec<Directory> = Vec::new();

        while let Some(directory) = Self::from_block(block, offset) {
            offset += directory.rec_len as usize;
            directories.push(directory);
        }

        directories
    }
}

impl Ext2fs {
    pub fn new(disk: Disk) -> Option<Ext2fs> {
        if read_hword(&disk.read_sectors(2, 1), 56) != 0xef53 {
            return None;
        }

        let mut ext2fs = Ext2fs {
            disk,
            block_size: 0,
            inode_size: 0,
            inodes_per_group: 0,
            block_group_count: 0,
        };

        let superblock = Superblock::from_block(ext2fs.get_blocks(1, 1));
        ext2fs.block_size = 1024 << superblock.s_log_size;
        ext2fs.inode_size = superblock.s_inode_size;
        ext2fs.inodes_per_group = superblock.s_inodes_per_group;

        let calculated_block_group_count =
            superblock.s_blocks_count / superblock.s_blocks_per_group;
        ext2fs.block_group_count = if calculated_block_group_count
            == superblock.s_inodes_count / superblock.s_inodes_per_group
        {
            calculated_block_group_count
        } else {
            return None;
        };

        Some(ext2fs)
    }

    pub fn get_blocks(&self, block_number: u32, block_count: u32) -> Vec<u8> {
        self.disk
            .read_sectors(block_number as u64 * 2, block_count * 2)
    }

    fn get_block_group_descriptor(&self, block_group_number: u32) -> BlockGroupDescriptor {
        let block = self.get_blocks(block_group_number * 32 / 1024 + 2, 1);

        BlockGroupDescriptor::from_block(block, (block_group_number as usize * 32) % 1024)
    }

    pub fn get_inode(&self, inode_number: u32) -> Inode {
        let block_group =
            self.get_block_group_descriptor((inode_number - 1) / self.inodes_per_group);

        let inode_index = (inode_number - 1) % self.inodes_per_group;

        let block_number =
            block_group.bg_inode_table + (inode_index * self.inode_size as u32) / self.block_size;
        let inode_offset = (inode_index * self.inode_size as u32) % self.block_size;

        Inode::from_block(self.get_blocks(block_number, 1), inode_offset as usize)
    }
}
