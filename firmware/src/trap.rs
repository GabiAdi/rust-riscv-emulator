// trap.rs
use core::arch::global_asm;

use crate::{
    console::{print_char, print_str, print_u64},
    csr, ebreak,
};

global_asm!(
    r#"
.section .text.trap
.align 4
.global _trap_entry
_trap_entry:
    nop
    addi sp, sp, -256
    sd x1,  0*8(sp)
    sd x5,  1*8(sp)
    sd x6,  2*8(sp)
    sd x7,  3*8(sp)
    sd x8,  4*8(sp)
    sd x9,  5*8(sp)
    sd x10, 6*8(sp)
    sd x11, 7*8(sp)
    sd x12, 8*8(sp)
    sd x13, 9*8(sp)
    sd x14, 10*8(sp)
    sd x15, 11*8(sp)
    sd x16, 12*8(sp)
    sd x17, 13*8(sp)
    sd x18, 14*8(sp)
    sd x19, 15*8(sp)
    sd x20, 16*8(sp)
    sd x21, 17*8(sp)
    sd x22, 18*8(sp)
    sd x23, 19*8(sp)
    sd x24, 20*8(sp)
    sd x25, 21*8(sp)
    sd x26, 22*8(sp)
    sd x27, 23*8(sp)
    sd x28, 24*8(sp)
    sd x29, 25*8(sp)
    sd x30, 26*8(sp)
    sd x31, 27*8(sp)

    call rust_trap_dispatch

    ld x1,  0*8(sp)
    ld x5,  1*8(sp)
    ld x6,  2*8(sp)
    ld x7,  3*8(sp)
    ld x8,  4*8(sp)
    ld x9,  5*8(sp)
    ld x10, 6*8(sp)
    ld x11, 7*8(sp)
    ld x12, 8*8(sp)
    ld x13, 9*8(sp)
    ld x14, 10*8(sp)
    ld x15, 11*8(sp)
    ld x16, 12*8(sp)
    ld x17, 13*8(sp)
    ld x18, 14*8(sp)
    ld x19, 15*8(sp)
    ld x20, 16*8(sp)
    ld x21, 17*8(sp)
    ld x22, 18*8(sp)
    ld x23, 19*8(sp)
    ld x24, 20*8(sp)
    ld x25, 21*8(sp)
    ld x26, 22*8(sp)
    ld x27, 23*8(sp)
    ld x28, 24*8(sp)
    ld x29, 25*8(sp)
    ld x30, 26*8(sp)
    ld x31, 27*8(sp)
    addi sp, sp, 256
    mret
"#
);

#[inline(always)]
unsafe fn read_mcause() -> u64 {
    unsafe {
        let v: u64;
        core::arch::asm!("csrr {}, mcause", out(reg) v);
        v
    }
}

#[unsafe(no_mangle)]
extern "C" fn rust_trap_dispatch() {
    unsafe {
        let cause = read_mcause();
        let is_interrupt = (cause >> 63) & 1 == 1;
        let code = cause & 0x7fff_ffff_ffff_ffff;

        match (is_interrupt, code) {
            // (true, 11) => keyboard_isr(),
            (true, 11) => timer_external_isr(), // machine external interrupt
            (true, 7) => timer_isr(),           // machine timer interrupt
            (false, 3) => ebreak_handler(),
            (false, _) => exception_handler(code),
            _ => {}
        }
    }
}

fn ebreak_handler() {
    return;
}

fn timer_external_isr() {
    const MMIO_TIMER_COUNT: *mut u8 = 0xFFFFFFFFFFFFFF08 as *mut u8;
    const MMIO_TIMER_ENABLE: *mut u8 = 0xFFFFFFFFFFFFFF10 as *mut u8;

    unsafe {
        MMIO_TIMER_ENABLE.write_volatile(0 as u8);
        MMIO_TIMER_COUNT.write_volatile(0);
    }

    let cycles = unsafe { csr::read_mcycle() };
    print_str("Cycles: ");
    print_u64(cycles);
    print_char(b'\n');

    // - 12 for _start
}

fn keyboard_isr() {
    const MMIO_CONSOLE: *mut u8 = 0xFFFFFFFFFFFFFF00 as *mut u8;
    const MMIO_KEYBOARD: *mut u8 = 0xFFFFFFFFFFFFFF08 as *mut u8;
    unsafe {
        let c = MMIO_KEYBOARD.read_volatile();
        MMIO_CONSOLE.write_volatile(c);
        MMIO_KEYBOARD.write_volatile(0);
    }
}

fn timer_isr() {}

fn exception_handler(_code: u64) {
    print_str("Exception: ");
    print_u64(_code);
    print_str("\n");
    panic!("Illegal instruction! ");
    // ebreak();
}
