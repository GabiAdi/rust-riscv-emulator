#[inline(always)]
pub unsafe fn write_mtvec(addr: u64) {
    core::arch::asm!("csrw mtvec, {}", in(reg) addr);
}
#[inline(always)]
pub unsafe fn read_mcause() -> u64 {
    let v: u64;
    core::arch::asm!("csrr {}, mcause", out(reg) v);
    v
}
#[inline(always)]
pub unsafe fn read_mepc() -> u64 {
    let v: u64;
    core::arch::asm!("csrr {}, mepc", out(reg) v);
    v
}
#[inline(always)]
pub unsafe fn write_mepc(v: u64) {
    core::arch::asm!("csrw mepc, {}", in(reg) v);
}
#[inline(always)]
pub unsafe fn set_mstatus_mie() {
    // mstatus.MIE is bit 3
    core::arch::asm!("csrsi mstatus, 0x8");
}
#[inline(always)]
pub unsafe fn write_mie(v: u64) {
    core::arch::asm!("csrw mie, {}", in(reg) v);
}

#[unsafe(no_mangle)]
pub extern "C" fn trap_handler() {
    unsafe {
        let cause = read_mcause();
        let is_interrupt = (cause >> 63) & 1 == 1;
        let code = cause & 0x7fff_ffff_ffff_ffff;

        if is_interrupt {
            match code {
                7 => { /* machine timer interrupt — clear/reload your timer here */ }
                11 => { /* machine external interrupt — service your PLIC/IRQ here */ }
                _ => {}
            }
        } else {
            // synchronous exception: illegal instr, ebreak, load/store fault, etc.
            print_str("EXCEPTION cause=");
            print_u64(code);
            print_str(" mepc=");
            print_u64(read_mepc());
            print_str("\n");
            // if it's a trap you want to skip past (e.g. ebreak), bump mepc by 4:
            // write_mepc(read_mepc() + 4);
        }
    }
}
