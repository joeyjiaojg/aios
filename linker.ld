/* AIOS Linker Script for x86_64 multiboot2 kernel */

ENTRY(_start)

SECTIONS {
    /* Load at 1 MiB physical address */
    . = 1M;

    /* Multiboot2 header must be within first 8192 bytes of ELF */
    .multiboot2 : ALIGN(8) {
        KEEP(*(.multiboot2))
    }

    .text : ALIGN(4K) {
        *(.text.boot)
        *(.text .text.*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K) {
        *(.data .data.*)
    }

    .bss : ALIGN(4K) {
        _bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        _bss_end = .;
    }

    /* Discard unnecessary sections */
    /DISCARD/ : {
        *(.comment)
        *(.eh_frame)
        *(.note.GNU-stack)
    }
}
