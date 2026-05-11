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
// The code:
//   mov rax, 1          ; sys_write
//   mov rdi, 1          ; fd=stdout
//   lea rsi, [rel msg]  ; buf
//   mov rdx, 22         ; len
//   int 0x80
//   mov rax, 60         ; sys_exit
//   xor rdi, rdi        ; status=0
//   int 0x80
//   msg: db "Hello from userspace!", 10
//
// Entry point: 0x400078  (0x400000 base + 0x78 header offset)
// p_vaddr:     0x400000
// p_filesz:    0x78 + code+data = 0xb0  (176 bytes total)
// p_memsz:     0x1000  (one page, BSS zero-fill)

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
    // p_filesz = 0xB0  (176 bytes: 64 ELF hdr + 56 phdr + 56 code+data)
    0xB0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // p_memsz  = 0x1000  (one 4KiB page so BSS extends to page boundary)
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // p_align  = 0x1000
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // ── Code (at file offset 0x78 = entry point vaddr 0x400078) ─────────
    // mov rax, 1
    0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00,
    // mov rdi, 1
    0x48, 0xC7, 0xC7, 0x01, 0x00, 0x00, 0x00,
    // lea rsi, [rip + msg_offset]   ; msg is 14 bytes ahead (7+7+2+2+2 = 22 instr bytes → label)
    // instruction: 48 8D 35 <rel32>
    // After this instruction (at offset 0x78+7+7+7=0x78+21=0x8D), rip=0x40008D+7=0x400094
    // msg is at 0x400096 (0x78 + 7+7+7+2+2 = 0x78+25 = 0x91... let's compute carefully)
    // Layout from 0x400078:
    //  +0  mov rax,1   7 bytes → 0x40007F
    //  +7  mov rdi,1   7 bytes → 0x400086
    //  +14 lea rsi,... 7 bytes → 0x40008D  (rip after = 0x40008D+7=0x400094? no, rip after = next instr)
    //      Actually rip after lea = 0x400085... let me recalc:
    //      0x400078 + 7 = 0x40007F  (after mov rax,1)
    //      0x40007F + 7 = 0x400086  (after mov rdi,1)
    //      0x400086 + 7 = 0x40008D  (after lea rsi, — rip=0x40008D when lea executes, but rip used = next instr addr)
    //      Wait: RIP-relative lea uses the address of the NEXT instruction as the base.
    //      lea rsi, [rip+disp32] where rip = 0x40008D
    //      +0x40008D + 7 = 0x400094  (after mov rdx,22)? no — lea is 7 bytes so next=0x40008D
    //      lea at 0x400086, 7 bytes: next instr at 0x40008D  → rip=0x40008D after decode
    //      +0x40008D + disp = msg_vaddr
    //      msg_vaddr = 0x400078 + 7+7+7+7+2+2 = 0x400078 + 32 = 0x400098? let's just count:
    //  Instruction sequence from 0x400078:
    //  [0]  48 C7 C0 01 00 00 00   mov rax,1     (7)  → ends at 0x40007F
    //  [7]  48 C7 C7 01 00 00 00   mov rdi,1     (7)  → ends at 0x400086
    //  [14] 48 8D 35 XX XX XX XX   lea rsi,[rip+?] (7) → ends at 0x40008D, rip_after=0x40008D
    //  [21] 48 C7 C2 16 00 00 00   mov rdx,22    (7)  → ends at 0x400094
    //  [28] CD 80                  int 0x80      (2)  → ends at 0x400096
    //  [30] 48 C7 C0 3C 00 00 00   mov rax,60    (7)  → ends at 0x40009D
    //  [37] 48 31 FF               xor rdi,rdi   (3)  → ends at 0x4000A0
    //  [40] CD 80                  int 0x80      (2)  → ends at 0x4000A2
    //  [42] 48 65 6C 6C 6F 20 66 72 6F 6D 20 75 73 65 72 73 70 61 63 65 21 0A  (22 bytes msg)
    //  msg vaddr = 0x400078 + 42 = 0x4000A2
    //  rip after lea = 0x40008D
    //  disp = 0x4000A2 - 0x40008D = 0x15  → disp32 = 0x15000000 (LE)
    // Wait that's not right: 0x4000A2 - 0x40008D = 0x15.  LE bytes: 15 00 00 00
    0x48, 0x8D, 0x35, 0x15, 0x00, 0x00, 0x00, // lea rsi, [rip+0x15]  → msg
    // mov rdx, 22
    0x48, 0xC7, 0xC2, 0x16, 0x00, 0x00, 0x00,
    // int 0x80
    0xCD, 0x80,
    // mov rax, 60  (sys_exit)
    0x48, 0xC7, 0xC0, 0x3C, 0x00, 0x00, 0x00,
    // xor rdi, rdi
    0x48, 0x31, 0xFF,
    // int 0x80
    0xCD, 0x80,
    // ── Message string at vaddr 0x4000A2 ────────────────────────────────
    b'H', b'e', b'l', b'l', b'o', b' ', b'f', b'r', b'o', b'm', b' ',
    b'u', b's', b'e', b'r', b's', b'p', b'a', b'c', b'e', b'!', b'\n',
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
        assert_eq!(filesz, 0xB0);
    }

    #[test]
    fn test_user_init_elf_total_size() {
        assert_eq!(USER_INIT_ELF.len(), 0xB0); // exactly filesz bytes
    }

    #[test]
    fn test_user_init_elf_message_present() {
        let msg = b"Hello from userspace!\n";
        // Message starts at offset 0x2A (= 0xB0 - 22)
        let msg_start = USER_INIT_ELF.len() - 22;
        assert_eq!(&USER_INIT_ELF[msg_start..], msg);
    }
}
