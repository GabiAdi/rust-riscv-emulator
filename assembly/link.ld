SECTIONS {
    . = 0x0;
    .text : {
        *(.text.start)
        *(.text*)
    }
    .data : {
        *(.data*)
    }
    .bss : {
        *(.bss*)
    }
}

/* _rom_end = .; */
/* _ram_start = _rom_end; */
/* _ram_size = 0x500000; */
/* _stack_top = _ram_start + _ram_size; */
