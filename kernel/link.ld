ENTRY(_start)

_ram_size   = 0x8000000;
_stack_size = 0x1000000;

SECTIONS {
    . = 0;
    _image_start = .;

    .text : {
        KEEP(*(.text.start))
        . = ALIGN(4);
        *(.text.trap)
        *(.text .text.*)
    }
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    .data.rel.ro : { *(.data.rel.ro .data.rel.ro.*) }
    .got : { *(.got .got.*) }
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    .rela.dyn : {
        __rela_start = .;
        *(.rela.dyn .rela.*)
        __rela_end = .;
    }
    .dynsym  : { *(.dynsym) }
    .dynstr  : { *(.dynstr) }
    .dynamic : { *(.dynamic) }

    . = ALIGN(16);
    _image_end = .;

    .bss (NOLOAD) : ALIGN(16) {
        _bss_start = .;
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(8);
        _bss_end = .;
    }

    . = ALIGN(16);
    _heap_start = .;

    /DISCARD/ : { *(.eh_frame*) *(.comment) }
}
