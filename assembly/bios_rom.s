    .include "bios_macros.s"
   
.section .data
.align 4
newline: .ascii "\n\0"
hex_chars: .ascii "0123456789ABCDEF"
str_0x: .ascii "0x\0"

   
    .section .text
    .globl bios_clear
    .globl bios_putc
    .globl bios_puth
    .globl bios_putw
    .globl bios_putu
    .globl bios_puti
    .globl bios_putx
    .globl bios_puts
    .globl bios_clin
    .globl bios_getc
    .globl bios_memcmp
    .globl bios_memcpy
    .globl bios_strcmp
    .globl bios_strncmp
    .globl bios_atoi
    .globl bios_rsec
    .globl bios_wsec
    .globl bios_init
    .globl bios_find
    .globl bios_load
    .globl bios_ls
    .globl bios_render
    .globl _start

# MMIO console
.equ MMIO_CONSOLE, 0xFF000000

# Disk controller MMIO
.equ MMIO_DISK, 0xFF00000C

# GPU MMIO
.equ MMIO_GPU, 0xFF000024

.equ main, 0x11000



_start:
    la sp, stack_top
    jal main

# --------------------------------------------------
# bios_clear()
# Clears console
# Clobbers: t0
# --------------------------------------------------
bios_clear:
    la t0, MMIO_CONSOLE
    li t1, 12
    sb t1, 0(t0)
    ret

# --------------------------------------------------
# bios_putc(char c)
# a0 = character (low byte)
# Clobbers: t0
# Returns: ret (uses ra)
# --------------------------------------------------
bios_putc:
    li   t0, MMIO_CONSOLE
    sb   a0, 0(t0)
    ret

# --------------------------------------------------
# bios_puth(uint16)
# a0 = packed 2 chars (low half is first char)
# Clobbers: t0
# --------------------------------------------------
bios_puth:
    li   t0, MMIO_CONSOLE
    sh   a0, 0(t0)
    ret

# --------------------------------------------------
# bios_putw(uint32)
# a0 = packed 4 chars (LSB first)
# Clobbers: t0
# --------------------------------------------------
bios_putw:
    li   t0, MMIO_CONSOLE
    sw   a0, 0(t0)
    ret
    
# --------------------------------------------------
# bios_puts(char* str)
# prints null-terminated string
# Arg: a0 = pointer to string
# Clobbers: t0, t1
# Returns: ret (ra preserved)
# --------------------------------------------------
bios_puts:
    PUSH ra
    PUSH s0
    
    mv s0, a0

1:  lb   a0, 0(s0)          # load byte from string
    beq  a0, zero, 2f       # if null terminator, done
    jal bios_putc           # print character
    addi s0, s0, 1          # move pointer to next char
    j    1b
2:  POP s0
    POP ra
    ret

# --------------------------------------------------
# bios_putu(uint32 value)
# prints unsigned integer in decimal (no stack, no nested calls)
# Arg: a0 contains value
# Clobbers: t0,t1,t2,t3,t5
# Uses digit_buf (bss) as temporary buffer
# --------------------------------------------------
bios_putu:
    PUSH ra
    li   t0, 10        # divisor
    li   t1, 0         # digit count

1:  remu t2, a0, t0
    divu a0, a0, t0
    addi t2, t2, '0'
    PUSH t2
    addi t1, t1, 1
    bnez a0, 1b

2:  POP a0
    PUSH t1
    jal bios_putc
    POP t1
    addi t1, t1, -1
    bnez t1, 2b

    POP ra
    ret

# --------------------------------------------------
# bios_puti(int32 value)
# prints signed integer in decimal
# Arg: a0 contains value
# Clobbers: 
# --------------------------------------------------

bios_puti:
    PUSH ra
    bgez a0, bios_puti_pos
    PUSH a0
    li a0, '-'
    jal bios_putc
    POP a0
    neg a0, a0
bios_puti_pos:
    jal bios_putu
    POP ra
    ret

# --------------------------------------------------
# bios_putx(uint32 value)
# prints unsigned integer in hexadecimal
# Arg: a0 contains value
# Clobbers: t0,t1,t2
# --------------------------------------------------
bios_putx:
    PUSH ra
    PUSH s0

    mv   s0, a0         # save value, a0 will be clobbered by bios_puts

    la   a0, str_0x
    jal  bios_puts      # print "0x" prefix

    li   t1, 8          # 8 nibbles to print

1:  addi t1, t1, -1     # count down from 7 to 0
    slli t0, t1, 2      # shift amount = nibble_index * 4
    srl  t0, s0, t0     # shift value right to bring nibble to bottom
    andi t0, t0, 0xF    # mask lowest nibble

    la   t2, hex_chars
    add  t2, t2, t0
    lbu  a0, 0(t2)      # look up ASCII character

    PUSH t1             # save counter across bios_putc
    jal  bios_putc      # print the character
    POP  t1

    bnez t1, 1b         # loop until all 8 nibbles printed

    POP  s0
    POP  ra
    ret

# --------------------------------------------------
# bios_clear_interrupt()
# Clears the interrupt in the BIOS console
# Clobbers: t0, t1
# --------------------------------------------------
bios_clin:
    li   t0, MMIO_CONSOLE
    li   t1, 0x00000000
    sw   t1, 4(t0)      # write to interrupt clear register
    
    ret

# --------------------------------------------------
# bios_getc()
# Reads a character from the console input register
# Clobbers: t0
# Returns: a0 = character read (low byte)
# --------------------------------------------------
bios_getc:
    li   t0, MMIO_CONSOLE
    lb   a0, 8(t0)      # read from input register
    ret

# --------------------------------------------------
# bios_memcmp(uint32* ptr1, uint32* ptr2, uint32 length)
# Compares two memory regions word by word
# Args: a0 = ptr1, a1 = ptr2, a2 = length
# Clobbers: t0, t1, t2
# Returns: a0 = 0 if equal, nonzero if different
# --------------------------------------------------
bios_memcmp:
    PUSH ra
    
    li t0, 0
memcmp_loop:
    lbu t1, 0(a0)
    lbu t2, 0(a1)
    bne t1, t2, memcmp_diff
    addi a0, a0, 1
    addi a1, a1, 1
    addi t0, t0, 1
    blt t0, a2, memcmp_loop
    li a0, 0
    POP ra
    ret
memcmp_diff:
    li a0, 1
    POP ra
    ret
    
# --------------------------------------------------
# bios_memcpy(uint32* dest, uint32* src, uint32 length)
# Copies memory from src to dest word by word
# Args: a0 = dest, a1 = src, a2 = length
# Clobbers: t0, t1
# --------------------------------------------------
bios_memcpy:
    PUSH ra
    beqz a2, memcpy_done
memcpy_loop:
    lbu t0, 0(a1)
    sb t0, 0(a0)
    addi a0, a0, 1
    addi a1, a1, 1
    addi a2, a2, -1
    bnez a2, memcpy_loop
memcpy_done:
    POP ra
    ret

# --------------------------------------------------
# bios_strcmp(char* str1, char* str2)
# Compares two null-terminated strings byte by byte
# Args: a0 = str1, a1 = str2
# Clobbers: t0, t1
# Returns: a0 = 0 if equal, nonzero if different
# --------------------------------------------------
bios_strcmp:
    PUSH ra

strcmp_loop:
    lbu t0, 0(a0)
    lbu t1, 0(a1)
    bne t0, t1, strcmp_diff
    beqz t0, strcmp_equal      # both are null terminator, strings are equal
    addi a0, a0, 1
    addi a1, a1, 1
    j strcmp_loop

strcmp_equal:
    li a0, 0
    POP ra
    ret

strcmp_diff:
    sub a0, t0, t1             # return difference like standard strcmp
    POP ra
    ret
    
# --------------------------------------------------
# bios_strncmp(char* str1, char* str2, uint32 n)
# Compares only the first n bytes
# a0 = str1, a1 = str2, a2 = n
# Returns: a0 = 0 if equal
# --------------------------------------------------
bios_strncmp:
    PUSH ra
    li t2, 0

strncmp_loop:
    bge  t2, a2, strncmp_equal  # checked n bytes, all matched
    lbu  t0, 0(a0)
    lbu  t1, 0(a1)
    bne  t0, t1, strncmp_diff
    beqz t0, strncmp_equal      # both null, equal
    addi a0, a0, 1
    addi a1, a1, 1
    addi t2, t2, 1
    j    strncmp_loop

strncmp_equal:
    li  a0, 0
    POP ra
    ret

strncmp_diff:
    sub a0, t0, t1
    POP ra
    ret

# --------------------------------------------------
# bios_atoi(char* str)
# Parses a decimal integer string, handles negative with '-'
# a0 = pointer to string
# Returns: a0 = integer value
# --------------------------------------------------
bios_atoi:
    PUSH ra
    PUSH s0; PUSH s1; PUSH s2

    mv   s0, a0
    li   s1, 0
    li   s2, 1

    lbu  t0, 0(s0)
    li   t1, '-'
    bne  t0, t1, atoi_loop
    li   s2, -1
    addi s0, s0, 1

atoi_loop:
    lbu  t0, 0(s0)
    beqz t0, atoi_done
    li   t1, '0'
    blt  t0, t1, atoi_done
    li   t1, '9'
    bgt  t0, t1, atoi_done
    addi t0, t0, -48
    li   t1, 10
    mul  s1, s1, t1
    add  s1, s1, t0
    addi s0, s0, 1
    j    atoi_loop

atoi_done:
    mul  a0, s1, s2
    POP s2; POP s1; POP s0
    POP ra
    ret

# ----------------------------------------------
# bios_read_sector(uint32 sector_num, uint32 buffer_addr)
# Reads a 512-byte sector from disk into memory
# Arg: a0 = sector number, a1 = buffer address
# Clobbers: t0, t1, t2
# --------------------------------------------------
bios_rsec:
    PUSH ra
    
    li t0, MMIO_DISK
    
    sw a0, 0(t0)                    # write sector number to disk controller
    sw a1, 8(t0)                    # write buffer address to disk
    li t1, 1
    sw t1, 16(t0)                   # start read operation
    
wait_for_disk_read:
    lw t1, 20(t0)                   # read status register
    li t2, 1
    beq t1, t2, wait_for_disk_read  # wait until disk is ready
    
    li t2, 2
    beq t1, t2, handle_read_error        # if error bit is set, handle error
    
    POP ra
    ret
    
handle_read_error:
    ebreak
    POP ra
    ret
    
# --------------------------------------------------
# bios_write_sector(uint32 sector_num, uint32 buffer_addr)
# Writes a 512-byte sector from memory to disk
# Arg: a0 = sector number, a1 = buffer address
# Clobbers: t0, t1, t2
# --------------------------------------------------
bios_wsec:
    PUSH ra
    
    li t0, MMIO_DISK
    
    sw a0, 0(t0)                    # write sector number to disk controller
    sw a1, 8(t0)                    # write buffer address to disk
    li t1, 2
    sw t1, 16(t0)                   # start write operation
   
wait_for_disk_write:
    lw t1, 20(t0)                   # read status register
    li t2, 1
    beq t1, t2, wait_for_disk_write # wait until disk is ready
    
    li t2, 2
    beq t1, t2, handle_write_error   # if error bit is set
    
    POP ra
    ret

handle_write_error:
    ebreak
    POP ra
    ret
    
# --------------------------------------------------
# bios_init_fat()
# Reads the FAT filesystem parameters from the boot sector and stores them in .data
# Clobbers: t0, t1, t2, t3, t4
# --------------------------------------------------
bios_init:
    PUSH ra
    PUSH s0; PUSH s1; PUSH s2; PUSH s3; PUSH s4
    
    li a0, 0
    la a1, bios_scratch
    jal ra, bios_rsec  
    
    la t0, bios_scratch
    
    # 1. Reserved Sectors (FAT Start)
    lhu s0, 0x0E(t0)   
    la t1, b_fat_start
    sw s0, 0(t1)       # Store FAT Start
    
    # 2. Calculate Root Start
    lbu t1, 0x10(t0)   # FAT Count
    lhu t2, 0x16(t0)   # Sectors per FAT
    mul t3, t1, t2     # Total FAT size
    add s1, s0, t3     # s1 = Root Start (s0 was FAT start)
    la t1, b_root_start
    sw s1, 0(t1)
    
    # 3. Calculate Data Start
    lbu s2, 0x11(t0)
    lbu t2, 0x12(t0)
    slli t2, t2, 8
    or s2, s2, t2
    
    la t1, b_root_entries
    sw s2, 0(t1)
    
    slli t2, s2, 5     # entry count * 32
    srli t2, t2, 9     # / 512
    add s3, s1, t2     # s3 = Data Start (s1 was Root start)
    la t1, b_data_start
    sw s3, 0(t1)
    
    # 4. Sectors per Cluster
    lbu s4, 0x0D(t0)   
    la t1, b_sec_per_clus
    sw s4, 0(t1)
    
    POP s4; POP s3; POP s2; POP s1; POP s0
    POP ra
    ret

# --------------------------------------------------
# bios_find_file(char* filename)
# Searches the root directory for a file with the given name and returns its starting cluster
# Arg: a0 = pointer to 11-byte filename (padded with spaces)
# Clobbers: t0, t1, t2
# Returns: a0 = starting cluster of file, or 0 if not found, a1 = file size
# --------------------------------------------------
bios_find:
    PUSH ra
    PUSH s0; PUSH s1; PUSH s2; PUSH s3; PUSH s4

    mv   s3, a0              # s3 = target filename
    lw   s4, b_root_start    # s4 = current root sector
    lw   s0, b_root_entries  # s0 = remaining entries to check

find_next_sector:
    mv   a0, s4
    la   a1, bios_scratch
    jal  ra, bios_rsec

    li   s1, 0               # s1 = entries checked this sector
    la   s2, bios_scratch    # s2 = pointer to current entry

check_entry:
    lbu  t0, 0(s2)
    beqz t0, find_failed     # 0x00 = end of directory

    mv   a0, s2
    mv   a1, s3
    li   a2, 11
    jal  ra, bios_memcmp

    beqz a0, find_success

    addi s2, s2, 32          # advance to next entry
    addi s1, s1, 1           # entries checked this sector
    addi s0, s0, -1          # total entries remaining
    beqz s0, find_failed

    li   t0, 16
    blt  s1, t0, check_entry # still entries left in this sector

    addi s4, s4, 1           # next root sector
    j    find_next_sector

find_success:
    lw   a0, 26(s2)
    li t0, 0xFFFF
    and a0, a0, t0      # first cluster (lhu broken — use lw+mask)
    lw   a1, 28(s2)          # file size
    j    find_done

find_failed:
    li   a0, 0
    li   a1, 0

find_done:
    POP s4; POP s3; POP s2; POP s1; POP s0
    POP ra
    ret
    
# --------------------------------------------------
# bios_read_file(uint32 start_cluster, uint32* buffer)
# Reads the entire file starting from the given cluster into the buffer
# Arg: a0 = starting cluster, a1 = buffer address
# Clobbers: t0, t1, t2, t3, t4, t5
# --------------------------------------------------
bios_load:
    PUSH ra
    PUSH s0; PUSH s1; PUSH s2; PUSH s3

    mv   s0, a0              # s0 = current cluster
    mv   s1, a1              # s1 = buffer pointer

load_loop:
    # 1. Convert cluster to LBA
    lw   t2, b_data_start
    addi t3, s0, -2
    lw   s2, b_sec_per_clus  # s2 = sectors per cluster
    mul  t3, t3, s2
    add  s3, t2, t3          # s3 = starting LBA of this cluster

read_entire_cluster:
    mv   a0, s3
    mv   a1, s1
    jal  ra, bios_rsec

    addi s3, s3, 1
    addi s1, s1, 512
    addi s2, s2, -1
    bnez s2, read_entire_cluster

    # 2. Get next cluster from FAT
    slli t2, s0, 1           # byte offset in FAT
    srli t3, t2, 9           # FAT sector index
    andi s3, t2, 511         # byte within sector (saved in s3, survives bios_rsec)

    lw   t4, b_fat_start
    add  a0, t3, t4
    la   a1, bios_scratch
    jal  ra, bios_rsec

    la   t3, bios_scratch
    add  t3, t3, s3
    lhu   s0, 0(t3) # next cluster

    li   t2, 0xFFF8
    bltu s0, t2, load_loop

    POP s3; POP s2; POP s1; POP s0
    POP ra
    ret
    
# --------------------------------------------------
# bios_ls(void (*callback)(char* name, uint32 size))
# Iterates all root directory entries and calls callback for each file
# Arg: a0 = callback function pointer
# Clobbers: t0, t1, t2
# Callback receives: a0 = pointer to 11-byte name, a1 = file size
# --------------------------------------------------
bios_ls:
    PUSH ra
    PUSH s0; PUSH s1; PUSH s2; PUSH s3; PUSH s4

    mv   s3, a0              # s3 = callback pointer
    lw   s4, b_root_start    # s4 = current root sector
    lw   s0, b_root_entries  # s0 = remaining entries to check

ls_next_sector:
    mv   a0, s4
    la   a1, bios_scratch
    jal  ra, bios_rsec

    li   s1, 0               # s1 = entries checked this sector
    la   s2, bios_scratch    # s2 = pointer to current entry

ls_check_entry:
    lbu  t0, 0(s2)
    beqz t0, ls_done         # 0x00 = end of directory

    li   t1, 0xE5
    beq  t0, t1, ls_skip     # 0xE5 = deleted entry, skip

    lbu  t0, 11(s2)          # attributes byte
    li   t1, 0x08
    beq  t0, t1, ls_skip     # volume label, skip
    li   t1, 0x0F
    beq  t0, t1, ls_skip     # LFN entry, skip

    # valid entry — call the callback
    mv   a0, s2              # a0 = pointer to entry (first 11 bytes are name)
    lw   a1, 28(s2)          # a1 = file size
    jalr ra, s3, 0           # call callback(name, size)

ls_skip:
    addi s2, s2, 32          # next entry
    addi s1, s1, 1
    addi s0, s0, -1
    beqz s0, ls_done

    li   t0, 16
    blt  s1, t0, ls_check_entry

    addi s4, s4, 1           # next root sector
    j    ls_next_sector

ls_done:
    POP s4; POP s3; POP s2; POP s1; POP s0
    POP ra
    ret
    
# --------------------------------------------------
# bios_render(uint32* buffer, uint32 size)
# Renders the buffer to the screen via GPU
# Arg: a0 = buffer pointer, a1 = buffer size
# Clobbers: t0, t1, t2
# --------------------------------------------------
bios_render:
    PUSH ra
    la t0, MMIO_GPU
    sw a0, 0x00(t0)
    sw a1, 0x04(t0)
    li t1, 1
    sw t1, 0x08(t0)
    POP ra
    ret

    .section .bss
    .org 0x2000
    .align 4
b_fat_start:    .word 0
b_root_start:   .word 0
b_data_start:   .word 0
b_sec_per_clus: .word 0
b_root_entries: .word 0
    
digit_buf:
    .space 12

.align 4
stack_bottom:
    .space 0x1000
stack_top:

.align 4
bios_scratch:
    .space 0x1000
    