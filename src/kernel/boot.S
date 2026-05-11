# AIOS Boot Assembly
# Multiboot2 header + 32-bit to 64-bit mode transition
#
# Model: claude-sonnet-4-6
# Tool: claude-code
# Prompt: Add multiboot2 boot assembly with 32-to-64-bit mode transition.
#
# Multiboot2 bootloaders (GRUB) enter here in 32-bit protected mode with:
#   eax = 0x36D76289 (multiboot2 magic)
#   ebx = physical address of multiboot2 info structure

.set MULTIBOOT2_MAGIC,    0xE85250D6
.set MULTIBOOT2_ARCH,     0          # i386 protected mode
.set MULTIBOOT2_LENGTH,   multiboot2_header_end - multiboot2_header_start
.set MULTIBOOT2_CHECKSUM, -(MULTIBOOT2_MAGIC + MULTIBOOT2_ARCH + MULTIBOOT2_LENGTH)

# ── Multiboot2 header (must be within first 8192 bytes of ELF) ────────────────
.section .multiboot2, "a"
.align 8
multiboot2_header_start:
    .long MULTIBOOT2_MAGIC
    .long MULTIBOOT2_ARCH
    .long MULTIBOOT2_LENGTH
    .long MULTIBOOT2_CHECKSUM
    # End tag
    .short 0    # type
    .short 0    # flags
    .long  8    # size
multiboot2_header_end:

# ── Temporary GDT for the transition to 64-bit long mode ─────────────────────
.section .rodata
.align 8
gdt64:
    .quad 0x0000000000000000    # null descriptor
    .quad 0x00AF9A000000FFFF    # 64-bit code: L=1, P=1, S=1, exec/read
    .quad 0x00CF92000000FFFF    # 64-bit data: G=1, P=1, S=1, read/write
gdt64_end:

gdt64_ptr:
    .short gdt64_end - gdt64 - 1
    .long  gdt64

# ── BSS: page tables and boot stack ───────────────────────────────────────────
.section .bss
.align 4096
.global p4_table
p4_table:   .skip 4096   # PML4
.global p3_table
p3_table:   .skip 4096   # PDPT
.global p2_table
p2_table:   .skip 4096   # PD (2 MiB pages)

.align 16
boot_stack_bottom:
    .skip 65536           # 64 KiB boot stack
.global boot_stack_top
boot_stack_top:

# ── 32-bit entry point ────────────────────────────────────────────────────────
.section .text.boot
.code32
.global _start
_start:
    cli
    # esi = mbi pointer (saved across BSS clear)
    movl %ebx, %esi

    cmpl $0x36D76289, %eax
    jne  boot_halt

    # Zero BSS inline without using the stack.
    # (page tables and boot_stack are in BSS, so no stack calls here)
    movl $_bss_start, %edi
    movl $_bss_end,   %ecx
    subl %edi, %ecx         # byte count
    xorl %eax, %eax
    rep  stosb               # zero BSS (page tables + stack → zeroed)

    # Now set up the boot stack (BSS is zeroed so stack contents are zero,
    # but ESP is a register — still valid)
    movl $boot_stack_top, %esp

    # Set up identity-mapped page tables (2 MiB pages, first 1 GiB)
    call setup_page_tables

    # Enable PAE
    movl %cr4, %eax
    orl  $(1 << 5), %eax
    movl %eax, %cr4

    # Load PML4
    movl $p4_table, %eax
    movl %eax, %cr3

    # Enable long mode (EFER.LME)
    movl $0xC0000080, %ecx
    rdmsr
    orl  $(1 << 8), %eax
    wrmsr

    # Enable paging + protected mode
    movl %cr0, %eax
    orl  $(1 << 31 | 1 << 0), %eax
    movl %eax, %cr0

    # Load GDT and far-jump into 64-bit mode
    lgdt gdt64_ptr
    movl %esi, %edi         # move mbi ptr to edi (first arg register in 64-bit ABI)
    ljmp $0x08, $long_mode_entry

boot_halt:
    hlt
    jmp boot_halt

# P4[0]→p3, P3[0]→p2, P2[0..512] = identity 2 MiB huge pages
# Uses stack (called AFTER BSS is zeroed and stack is set up)
setup_page_tables:
    movl $p3_table, %eax
    orl  $0x3, %eax
    movl %eax, p4_table

    movl $p2_table, %eax
    orl  $0x3, %eax
    movl %eax, p3_table

    movl $0, %ecx
fill_p2:
    movl %ecx, %eax
    shll $21, %eax
    orl  $0x83, %eax        # PRESENT | WRITE | HUGE
    movl %eax, p2_table(, %ecx, 8)
    incl %ecx
    cmpl $512, %ecx
    jne  fill_p2
    ret

# ── 64-bit entry ──────────────────────────────────────────────────────────────
.section .text
.code64
.global long_mode_entry
long_mode_entry:
    movw $0x10, %ax
    movw %ax, %ss
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %fs
    movw %ax, %gs

    # Stack pointer (RIP-relative to avoid absolute 32-bit reloc)
    lea  boot_stack_top(%rip), %rsp

    # edi = mbi pointer from 32-bit entry; zero-extend to rdi
    movl %edi, %edi
    callq kernel_main

hang:
    hlt
    jmp hang
