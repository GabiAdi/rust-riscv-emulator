#![no_std]
#![no_main]

mod allocator;
mod drivers;
mod trap;

extern crate alloc;

use core::{panic::PanicInfo, u64};

use alloc::{string::String, vec::Vec};

use crate::{
    allocator::LinkedListAllocator,
    drivers::{
        console::ConsoleWriter,
        disk::Disk,
        ext2::{Directory, Ext2fs, File},
    },
};

unsafe extern "C" {
    fn _trap_entry();
    static _heap_start: u8;
    static _heap_end: u8;
    static _stack_top: u8;
}

#[global_allocator]
static ALLOCATOR: LinkedListAllocator = LinkedListAllocator::new_uninit();

const MMIO_CONSOLE: u64 = 0xFFFFFFFFFFFFFF00;
const MMIO_DISK: u64 = 0xFFFFFFFFFFFFFF10;
const SECTOR_SIZE: u32 = 512;

#[unsafe(link_section = ".text.start")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "la sp, _stack_top",
        "la t0, {trap}",
        "csrw mtvec, t0",
        "li t0, (1 << 11)",
        "csrw mie, t0",
        "csrsi mstatus, 0x8",
        "j main",
        trap = sym _trap_entry,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> ! {
    unsafe {
        let heap_start = &_heap_start as *const u8 as usize;
        let heap_end = &_heap_end as *const u8 as usize;
        ALLOCATOR.init(heap_start, heap_end);
    }

    let mut console = ConsoleWriter::new(MMIO_CONSOLE).unwrap();
    let disk = Disk::new(MMIO_DISK, SECTOR_SIZE).unwrap();
    let ext2fs = Ext2fs::new(disk).unwrap();

    // let superblock = Superblock::from_block(ext2fs.get_blocks(1, 1));
    let root_inode = ext2fs.get_inode(2);

    let data = ext2fs.get_blocks(root_inode.i_block.direct_pointers[0], 1);

    let directories: Vec<Directory> = Directory::vec_from_block(&data);

    let mut test_dir: Option<Directory> = None;

    for dir in directories {
        let name = &dir.name.clone();
        if dir.name == "1m.txt" {
            test_dir = Some(dir);
        }
        console.print_str(name);
        console.print_str("\n");
    }

    let mut file = File::from_directory(test_dir.unwrap(), ext2fs).unwrap();
    file.seek(20);
    let data = file.read_delim('\n' as u8).unwrap();
    // let data = file.read_all().unwrap();
    // let data = file.read_bytes(6000).unwrap();
    // let data2 = file.read_all().unwrap();
    // let data2 = file.read_bytes(43).unwrap();

    for i in 0..data.len() {
        unsafe {
            console.print_char(data[i as usize]);
        }
    }
    // for i in 0..data2.len() {
    //     unsafe {
    //         console.print_char(data2[i as usize]);
    //     }
    // }

    console.print_str("\nDone!\n");
    ConsoleWriter::print_fmt(format_args!("Size1: {}, Size2: {}\n\n", data.len(), 0));

    // let inode = ext2fs.get_inode(inode_number);
    // let block = ext2fs.get_blocks(inode.i_block.direct_pointers[0], 1);
    // for i in 0..inode.i_size {
    //     unsafe {
    //         console.print_char(block[i as usize]);
    //     }
    // }

    // for c in data {
    //     unsafe {
    //         console.print_char(c);
    //     }
    // }

    ConsoleWriter::print_fmt(format_args!("\n"));

    ebreak();
    loop {}
}

fn test_vector(mut console: ConsoleWriter) {
    use alloc::vec::Vec;
    let mut v: Vec<u64> = Vec::new();

    for i in 0..10000 {
        v.push(i);
        if v[i as usize] % 100 == 0 {
            core::fmt::write(
                &mut console,
                format_args!("Element of vector: {}\n", v[i as usize]),
            );
            // console.print_str(format_args!("Element of vector: {}", v[i as usize]));
            // console.print_str("Element of vector: ");
            // console.print_u64(v[i as usize]);
            // console.print_char(b'\n');
        }
    }

    console.print_str("Done allocation\n");

    for n in v {
        if n % 100 == 0 {
            core::fmt::write(&mut console, format_args!("Element of vector: {}\n", n));
            // console::print_str("Element of vector: ");
            // console::print_u64(n);
            // console::print_char(b'\n');
        }
    }
}

#[unsafe(naked)]
extern "C" fn ebreak() {
    core::arch::naked_asm!("ebreak");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let console = unsafe { ConsoleWriter::new_unchecked(MMIO_CONSOLE) };
    console.print_str("PANIC AT: ");
    if let Some(location) = info.location() {
        console.print_str(location.file());
        console.print_str(":");
        console.print_u64(location.line() as u64);
        console.print_str("\n");
    }
    if let Some(msg) = info.message().as_str() {
        console.print_str(msg);
    }
    console.print_str("\n");

    unsafe {
        core::arch::asm!(".4byte 0x00100073");
    }

    loop {}
}
