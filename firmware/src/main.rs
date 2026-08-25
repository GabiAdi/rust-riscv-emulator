#![no_std]
#![no_main]

mod allocator;
mod console;
mod csr;
mod trap;

extern crate alloc;

use core::{panic::PanicInfo, u64};

use crate::{
    allocator::LinkedListAllocator,
    console::{ConsoleWriter, print_char, print_str, print_u64},
};

unsafe extern "C" {
    fn _trap_entry();
    static _heap_start: u8;
    static _heap_end: u8;
    static _stack_top: u8;
}

#[global_allocator]
static ALLOCATOR: LinkedListAllocator = LinkedListAllocator::new_uninit();

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
        "li t0, 1000000000",
        "li t1, 0xFFFFFFFFFFFFFF08",
        "sd t0, 0(t1)",
        "li t0, 1",
        "sb t0, 8(t1)",
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

    // use alloc::vec::Vec;
    // let mut v: Vec<u64> = Vec::with_capacity(4);
    //
    // v.push(42);
    // v.push(53);
    // v.push(67);
    // v.push(43);
    // v.push(55);
    //
    // for n in v {
    //     console::print_str("Element of vector: ");
    //     console::print_u64(n);
    //     console::print_char(b'\n');
    // }

    test_vector();

    ebreak();
    loop {}
}

fn test_vector() {
    use alloc::vec::Vec;
    let mut v: Vec<u64> = Vec::new();

    for i in 0..10000 {
        v.push(i);
        if v[i as usize] % 100 == 0 {
            console::print_str("Element of vector: ");
            console::print_u64(v[i as usize]);
            console::print_char(b'\n');
        }
    }

    print_str("Done allocation\n");

    for n in v {
        if n % 100 == 0 {
            console::print_str("Element of vector: ");
            console::print_u64(n);
            console::print_char(b'\n');
        }
    }
}

#[unsafe(naked)]
extern "C" fn ebreak() {
    core::arch::naked_asm!("ebreak");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    console::print_str("PANIC AT: ");
    if let Some(location) = info.location() {
        // console::print_str(location.file());
        // console::print_str(":");
        // console::print_u64(location.line() as u64);
        // console::print_str("\n");
    }
    if let Some(msg) = info.message().as_str() {
        console::print_str(msg);
    }
    console::print_str("\n");
    unsafe {
        core::arch::asm!(".4byte 0x00100073");
    }

    // let mut w = ConsoleWriter;
    // let _ = core::fmt::write(&mut w, format_args!("PANIC: {}\n", info));
    // unsafe { core::arch::asm!("ebreak") }

    loop {}
}
