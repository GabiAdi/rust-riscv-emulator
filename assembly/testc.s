.equ MMIO_CONSOLE, 0xFFFFFFFFFFFFFF00
.equ MMIO_KEYBOARD, 0xFFFFFFFFFFFFFF08

.section .data

.section .text

.global _start

_start:
    li    sp, 0x10000

    la    t0, trap_handler
    csrw  mtvec, t0

    li    t0, (1<<11)
    csrw  mie, t0

    csrsi mstatus, 0x8 

wait_loop:
    nop
    j     wait_loop

.align 4
trap_handler:
    addi  sp, sp, -16
    sd    t0, 0(sp)
    sd    t1, 8(sp)

    la    t0, MMIO_KEYBOARD
    lbu   t1, 0(t0)

    la    t0, MMIO_CONSOLE
    sb    t1, 0(t0)

    la    t0, MMIO_KEYBOARD
    sb    zero, 0(t0)

    ld    t0, 0(sp)
    ld    t1, 8(sp)

    addi  sp, sp, 16

    mret

.section .bss 
