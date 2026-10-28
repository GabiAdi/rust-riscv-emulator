#![no_std]
#![no_main]

mod allocator;
mod drivers;
mod trap;

extern crate alloc;

use allocator::LinkedListAllocator;
use core::{panic::PanicInfo, u64};
use drivers::console::ConsoleWriter;

use crate::drivers::{
    disk::Disk,
    ext2::{Ext2fs, File},
};

const MMIO_CONSOLE: u64 = 0xFFFFFFFFFFFFFF00;
const MMIO_DISK: u64 = 0xFFFFFFFFFFFFFF10;
const MMIO_KEYBOARD: u64 = 0xFFFFFFFFFFFFFF30;
const SECTOR_SIZE: u32 = 512;

const RAM_END: usize = 0x8000000;
const STACK_SIZE: usize = 0x1000000;
const HEAP_END: usize = RAM_END - STACK_SIZE;

#[global_allocator]
static ALLOCATOR: LinkedListAllocator = LinkedListAllocator::new_uninit();

#[unsafe(link_section = ".text.start")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "mv   s0, a0",
        "mv   s1, a1",
        "lla  s2, _start",          // runtime base (link addr is 0)
        "li   sp, {ram_end}",       // physical RAM end, not base-relative

        "lla  t0, __rela_start",
        "lla  t1, __rela_end",
        "li   t3, 3",
        "1:",
        "bgeu t0, t1, 3f",
        "ld   t2, 8(t0)",           // r_info
        "bne  t2, t3, 2f",
        "ld   t4, 0(t0)",           // r_offset
        "ld   t5, 16(t0)",          // r_addend
        "add  t4, t4, s2",
        "add  t5, t5, s2",
        "sd   t5, 0(t4)",
        "2:",
        "addi t0, t0, 24",
        "j    1b",
        "3:",

        // zero .bss
        "lla  t0, _bss_start",
        "lla  t1, _bss_end",
        "4:",
        "bgeu t0, t1, 5f",
        "sd   zero, 0(t0)",
        "addi t0, t0, 8",
        "j    4b",
        "5:",

        "mv   a0, s0",              // hartid
        "mv   a1, s1",              // dtb
        "mv   a2, s2",              // base
        "lla  a3, _heap_start",     // runtime heap start
        "tail main",
        ram_end = const RAM_END,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn main(_hartid: usize, _dtb: usize, _base: usize, heap_start: usize) -> ! {
    let _heap_end = HEAP_END;
    unsafe {
        ALLOCATOR.init(heap_start, HEAP_END);
    }
    let _ = heap_start;

    let console = ConsoleWriter::new(MMIO_CONSOLE).unwrap();

    console.print_str("Testing...\n");

    let disk = Disk::new(MMIO_DISK, SECTOR_SIZE).unwrap();
    let ext2fs = Ext2fs::new(disk).unwrap();
    let mut file = File::from_path("dir_test/top10.txt", ext2fs).unwrap();
    let data = file.read_all().unwrap();

    for c in data {
        unsafe {
            console.print_char(c);
        }
    }

    ebreak();
    loop {}
}

#[unsafe(naked)]
extern "C" fn ebreak() {
    core::arch::naked_asm!("ebreak");
}

#[unsafe(naked)]
extern "C" fn ecall() {
    core::arch::naked_asm!("ecall");
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
