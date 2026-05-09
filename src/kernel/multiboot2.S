# AIOS Multiboot2 Header
# This must be within the first 8192 bytes of the kernel
.section .multiboot2, "a"

.align 8
multiboot2_header:
    .long 0xE85250D6          # magic number
    .long 0                    # architecture: 0 = protected mode
    .long 16                   # header length
    .long 0x100000000 - (0xE85250D6 + 0 + 16)  # checksum

    # End tag
    .short 0                   # type: 0 = end
    .short 0                   # flags: 0
    .long 8                    # size: 8 (minimum)

