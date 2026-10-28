#![no_std]
#![no_main]

mod allocator;
mod drivers;
mod trap;

#[macro_use]
extern crate alloc;

use core::{panic::PanicInfo, u64};

use alloc::{
    string::String,
    vec::{self, Vec},
};

use crate::{
    allocator::LinkedListAllocator,
    drivers::{
        console::ConsoleWriter,
        disk::{Disk, DiskRegister},
        ext2::{Directory, Ext2fs, File},
    },
    trap::take_key,
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
const MMIO_KEYBOARD: u64 = 0xFFFFFFFFFFFFFF30;
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

    // loop {
    //     match take_key() {
    //         Some(v) => unsafe {
    //             console.print_char(v);
    //         },
    //         None => {}
    //     }
    // }

    let mut file = File::from_path("kernel", ext2fs).unwrap();
    // let mut file = File::from_path("test.txt", ext2fs).unwrap();
    // file.seek(20);
    // let data = file.read_delim('\n' as u8).unwrap();
    let data = file.read_all().unwrap();
    // let data = file.read_bytes(6000).unwrap();
    // let data2 = file.read_all().unwrap();
    // let data2 = file.read_bytes(43).unwrap();

    // for i in 0..data.len() {
    //     unsafe {
    //         console.print_char(data[i as usize]);
    //     }
    // }
    // for i in 0..data2.len() {
    //     unsafe {
    //         console.print_char(data2[i as usize]);
    //     }
    // }

    // console.print_str("\nDone!\n");

    const KERNEL_EXTRA: usize = 1 << 20;
    let raw = vec![0u8; data.len() + KERNEL_EXTRA + 4096];
    let base = (raw.as_ptr() as usize + 4095) & !4095;
    unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), base as *mut u8, data.len()) };

    ConsoleWriter::print_fmt(format_args!(
        "Size: {}, Pointer addr: {}\n\n",
        data.len(),
        data.as_ptr() as usize
    ));
    unsafe { jump_to(base, 0, 0) };

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

    // ConsoleWriter::print_fmt(format_args!("\n"));

    ebreak();
    loop {}
}

unsafe fn jump_to(addr: usize, a0: usize, a1: usize) -> ! {
    unsafe {
        core::arch::asm!("fence.i", "jr {0}", in(reg) addr, in("a0") a0, in("a1") a1, options(noreturn))
    };
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
