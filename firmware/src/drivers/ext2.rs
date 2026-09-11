use alloc::vec::Vec;

use crate::drivers::disk::Disk;

pub struct Ext2fs {
    disk: Disk,
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

fn read_hword(data: &Vec<u8>, offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn read_word(data: &Vec<u8>, offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn read_qword(data: &Vec<u8>, offset: usize) -> u128 {
    u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap())
}

impl Ext2fs {
    pub fn new(disk: Disk) -> Option<Ext2fs> {
        if read_hword(&disk.read_sectors(2, 1), 56) != 0xef53 {
            return None;
        }

        let ext2fs = Ext2fs { disk };

        Some(ext2fs)
    }

    pub fn get_superblock(&self) -> Superblock {
        let data = &self.disk.read_sectors(2, 2);

        Superblock {
            s_inodes_count: read_word(data, 0),
            s_blocks_count: read_word(data, 4),
            s_r_blocks_count: read_word(data, 8),
            s_free_blocks_count: read_word(data, 12),
            s_free_inodes_count: read_word(data, 16),
            s_first_data_block: read_word(data, 20),
            s_log_size: read_word(data, 24),
            s_log_frag_size: read_word(data, 28),
            s_blocks_per_group: read_word(data, 32),
            s_frags_per_group: read_word(data, 36),
            s_inodes_per_group: read_word(data, 40),
            s_mtime: read_word(data, 44),
            s_wtime: read_word(data, 48),
            s_mnt_count: read_hword(data, 52),
            s_max_mnt_count: read_hword(data, 54),
            s_magic: read_hword(data, 56),
            s_state: read_hword(data, 58),
            s_errors: read_hword(data, 60),
            s_minor_rev_level: read_hword(data, 62),
            s_lastcheck: read_word(data, 64),
            s_checkinterval: read_word(data, 68),
            s_creator_os: read_word(data, 72),
            s_rev_level: read_word(data, 76),
            s_def_resuid: read_hword(data, 80),
            s_def_resgid: read_hword(data, 82),
            s_first_ino: read_word(data, 84),
            s_inode_size: read_hword(data, 88),
            s_block_group_nr: read_hword(data, 90),
            s_feature_compat: read_word(data, 92),
            s_feature_incompat: read_word(data, 96),
            s_feature_ro_compat: read_word(data, 100),
            s_uuid: read_qword(data, 104),
            s_volume_name: read_qword(data, 120),
            s_last_mounted: data[136..200]
                .try_into()
                .expect("Slice with incorrect length"),
            s_algo_bitmap: read_word(data, 200),
            s_prealloc_blocks: data[204],
            s_prealloc_dir_blocks: data[205],
            s_journal_uuid: read_qword(data, 208),
            s_journal_inum: read_word(data, 224),
            s_journal_dev: read_word(data, 228),
            s_last_orphan: read_word(data, 232),
            s_hash_seed: read_qword(data, 236),
            s_def_hash_version: data[252],
            s_default_mount_options: read_word(data, 256),
            s_first_meta_bg: read_word(data, 260),
        }
    }
}
