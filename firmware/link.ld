ENTRY(_start)

_ram_start  = 0x0;
_ram_size   = 0x5000000;
_ram_end    = _ram_start + _ram_size;
_stack_size = 0x100000;

SECTIONS {
    . = _ram_start;
    .text : {
        *(.text.start)
        . = ALIGN(4);
        *(.text.trap)
        *(.text*)
    }
    .rodata : {
        *(.rodata*)
    }
    .data : {
        *(.data*)
    }
    .bss : {
        _bss_start = .;
        *(.bss*)
        . = ALIGN(8);
        _bss_end = .;
    }
    . = ALIGN(16);
    _heap_start = .;
    . = _ram_end - _stack_size;
    _heap_end = .;
}
_stack_top = _ram_end;
