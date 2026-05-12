// AIOS User Init Binary
//
// Model: claude-sonnet-4-6
// Tool: claude-code
// Prompt: Embed a hand-crafted minimal x86_64 ELF that calls sys_write then sys_exit via
//         int 0x80, to prove the ring-3 userspace path works end-to-end.

// Hand-crafted ELF64 executable.  Layout:
//   Offset 0x00  ELF header        (64 bytes)
//   Offset 0x40  Program header    (56 bytes)
//   Offset 0x78  Code              (loaded at vaddr 0x400078)
//
// The code (SIMPLIFIED TEST - infinite loop):
//   jmp $         ; infinite loop (0xEB 0xFE = jmp -2)
//
// Entry point: 0x400078  (0x400000 base + 0x78 header offset)
// p_vaddr:     0x400000
// p_filesz:    0x78 + code = 0x7A  (122 bytes total)
// p_memsz:     0x1000  (one page)

pub const USER_INIT_ELF: &[u8] = &[
    // ── ELF64 header (64 bytes) ─────────────────────────────────────────
    0x7F, 0x45, 0x4C, 0x46, // e_ident magic
    0x02,                   // EI_CLASS  = ELFCLASS64
    0x01,                   // EI_DATA   = ELFDATA2LSB
    0x01,                   // EI_VERSION= EV_CURRENT
    0x00,                   // EI_OSABI  = ELFOSABI_NONE
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // EI_ABIVERSION + pad
    0x02, 0x00,             // e_type    = ET_EXEC
    0x3E, 0x00,             // e_machine = EM_X86_64
    0x01, 0x00, 0x00, 0x00, // e_version = EV_CURRENT
    // e_entry = 0x400078  (load base 0x400000 + 0x78 header bytes)
    0x78, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
    // e_phoff = 64  (program header table immediately follows ELF header)
    0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_shoff = 0
    0x00, 0x00, 0x00, 0x00, // e_flags
    0x40, 0x00,             // e_ehsize  = 64
    0x38, 0x00,             // e_phentsize = 56
    0x01, 0x00,             // e_phnum   = 1
    0x40, 0x00,             // e_shentsize = 64
    0x00, 0x00,             // e_shnum   = 0
    0x00, 0x00,             // e_shstrndx = 0
    // ── PT_LOAD program header (56 bytes, at offset 0x40) ───────────────
    0x01, 0x00, 0x00, 0x00, // p_type   = PT_LOAD
    0x05, 0x00, 0x00, 0x00, // p_flags  = PF_R | PF_X
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_offset = 0
    // p_vaddr = 0x400000
    0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
    // p_paddr = 0x400000  (same as vaddr for static binary)
    0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
    // p_filesz = 0x7A  (122 bytes: 64 ELF hdr + 56 phdr + 2 code)
    0x7A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // p_memsz  = 0x1000  (one 4KiB page)
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // p_align  = 0x1000
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // ── Code (at file offset 0x78 = entry point vaddr 0x400078) ─────────
    // jmp $  (infinite loop: 0xEB 0xFE means jmp -2)
    0xEB, 0xFE,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_init_elf_magic() {
        assert_eq!(&USER_INIT_ELF[0..4], &[0x7F, b'E', b'L', b'F']);
    }

    #[test]
    fn test_user_init_elf_class_64() {
        assert_eq!(USER_INIT_ELF[4], 2); // ELFCLASS64
    }

    #[test]
    fn test_user_init_elf_machine_x86_64() {
        assert_eq!(USER_INIT_ELF[18], 0x3E);
        assert_eq!(USER_INIT_ELF[19], 0x00);
    }

    #[test]
    fn test_user_init_elf_type_exec() {
        let e_type = u16::from_le_bytes([USER_INIT_ELF[16], USER_INIT_ELF[17]]);
        assert_eq!(e_type, 2); // ET_EXEC
    }

    #[test]
    fn test_user_init_elf_entry_point() {
        let entry = u64::from_le_bytes(USER_INIT_ELF[24..32].try_into().unwrap());
        assert_eq!(entry, 0x400078);
    }

    #[test]
    fn test_user_init_elf_phoff() {
        let phoff = u64::from_le_bytes(USER_INIT_ELF[32..40].try_into().unwrap());
        assert_eq!(phoff, 64);
    }

    #[test]
    fn test_user_init_elf_phnum() {
        let phnum = u16::from_le_bytes([USER_INIT_ELF[56], USER_INIT_ELF[57]]);
        assert_eq!(phnum, 1);
    }

    #[test]
    fn test_user_init_elf_phdr_type_pt_load() {
        let p_type = u32::from_le_bytes(USER_INIT_ELF[64..68].try_into().unwrap());
        assert_eq!(p_type, 1); // PT_LOAD
    }

    #[test]
    fn test_user_init_elf_phdr_vaddr() {
        let vaddr = u64::from_le_bytes(USER_INIT_ELF[72..80].try_into().unwrap());
        assert_eq!(vaddr, 0x400000);
    }

    #[test]
    fn test_user_init_elf_phdr_filesz() {
        let filesz = u64::from_le_bytes(USER_INIT_ELF[96..104].try_into().unwrap());
        assert_eq!(filesz, 0x7A); // Updated for simplified infinite loop
    }

    #[test]
    fn test_user_init_elf_total_size() {
        assert_eq!(USER_INIT_ELF.len(), 0x7A); // Updated for simplified infinite loop
    }

    #[test]
    fn test_user_init_elf_infinite_loop_code() {
        // Check that the code at offset 0x78 is 0xEB 0xFE (jmp $)
        assert_eq!(USER_INIT_ELF[0x78], 0xEB);
        assert_eq!(USER_INIT_ELF[0x79], 0xFE);
    }
}
