.equ MMIO_CONSOLE, 0xFFFFFFFFFFFFFF00

.section .data

.section .text

.global _start

_start:
  la sp, 0x10000

  c.li t0, 1
  .option norvc
  addi t1, t0, 100      # straddle #1
  .option rvc
  c.li t0, 2
  .option norvc
  addi t2, t0, 200      # straddle #2 (pc phase may differ due to prior norvc block)
  .option rvc
  c.li t0, 3
  .option norvc
  addi t3, t0, 300      # straddle #3
  .option rvc

  li a0, 101
  bne t1, a0, fail
  li a0, 202
  bne t2, a0, fail
  li a0, 303
  bne t3, a0, fail
  j pass


pass:
  li s0, 1
  ebreak

fail:
  li s0, 2
  ebreak
.section .bss 
