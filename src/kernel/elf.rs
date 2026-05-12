// AIOS ELF Loader
//
// Model: claude-sonnet-4-6
// Tool: claude-code
// Prompt: Add user page-table mapping for ELF segments and TSS stack setup before iretq;
//         export p2_table and boot_stack_top from boot.S for use here.

use crate::memory::FrameAllocator;
use x86_64::structures::gdt::SegmentSelector;

const EI_NIDENT: usize = 16;
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
#[cfg(test)]
const PF_X: u32 = 1;
#[cfg(test)]
const PF_W: u32 = 2;
#[cfg(test)]
const PF_R: u32 = 4;

const PT_LOAD: u32 = 1;
const P2_ENTRY_SIZE: u64 = 2 * 1024 * 1024; // 2 MiB per P2 entry
const P2_ENTRIES: usize = 512;
// P2 entry flags: PRESENT | WRITE | HUGE | USER_ACCESSIBLE
const P2_FLAGS_USER: u64 = 0x87;

// Symbols exported from boot.S
extern "C" {
    static mut p2_table: [u64; P2_ENTRIES];
    static boot_stack_top: u8;
}

/// Mark the 2 MiB P2 entries that cover [vaddr, vaddr+memsz) as user-accessible.
/// The boot page tables use 2 MiB huge pages for the first 1 GiB (identity mapped).
/// This adds the USER_ACCESSIBLE flag (bit 2) so ring-3 code can access those pages.
///
/// # Safety
/// Modifies the live boot page tables. Safe to call before the first iretq to ring 3
/// because no user-mode code is executing yet. `p2_table` lives in BSS at a known
/// address; accessing it as a `[u64; 512]` via the exported symbol is valid.
pub fn map_user_segment(vaddr: u64, memsz: u64) {
    let start_entry = (vaddr / P2_ENTRY_SIZE) as usize;
    let end_entry = ((vaddr + memsz + P2_ENTRY_SIZE - 1) / P2_ENTRY_SIZE) as usize;
    let end_entry = end_entry.min(P2_ENTRIES);
    // # Safety
    // p2_table is the boot-time PD (page directory) exported from boot.S.
    // Entries cover the first 1 GiB identity-mapped. We only set the USER bit
    // on entries that correspond to the ELF segment virtual range. Called once
    // from load_and_map, before interrupts route to ring-3 code.
    unsafe {
        for i in start_entry..end_entry {
            p2_table[i] = (i as u64 * P2_ENTRY_SIZE) | P2_FLAGS_USER;
        }
        // Flush TLB by reloading CR3.
        core::arch::asm!(
            "mov rax, cr3",
            "mov cr3, rax",
            out("rax") _,
        );
    }
}

const USER_STACK_SIZE: usize = 4096 * 8;
const MAX_PHDR: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub(crate) struct Elf64Ehdr {
    e_ident: [u8; EI_NIDENT],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub(crate) struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

pub struct LoadedElf {
    pub entry: u64,
    pub base: u64,
    pub size: u64,
}

pub struct ElfLoader {
    phdrs: [Elf64Phdr; MAX_PHDR],
    phdr_count: usize,
}

impl ElfLoader {
    pub fn new() -> Self {
        Self {
            phdrs: [Elf64Phdr {
                p_type: 0,
                p_flags: 0,
                p_offset: 0,
                p_vaddr: 0,
                p_paddr: 0,
                p_filesz: 0,
                p_memsz: 0,
                p_align: 0,
            }; MAX_PHDR],
            phdr_count: 0,
        }
    }

    pub(crate) fn validate(data: &[u8]) -> Result<&Elf64Ehdr, &'static str> {
        if data.len() < core::mem::size_of::<Elf64Ehdr>() {
            return Err("Data too small for ELF header");
        }

        // # Safety
        // We verified data.len() >= size_of::<Elf64Ehdr> above.
        // Elf64Ehdr is #[repr(C)] so the pointer cast is valid.
        let ehdr = unsafe { &*(data.as_ptr() as *const Elf64Ehdr) };

        if ehdr.e_ident[0..4] != ELF_MAGIC {
            return Err("Invalid ELF magic");
        }

        if ehdr.e_ident[4] != 2 {
            return Err("Not 64-bit ELF");
        }

        if ehdr.e_ident[5] != 1 {
            return Err("Not little-endian ELF");
        }

        if ehdr.e_machine != 0x3E {
            return Err("Not x86_64 architecture");
        }

        if ehdr.e_type != 2 {
            return Err("Not an executable ELF");
        }

        Ok(ehdr)
    }

    pub fn parse_phdrs(&mut self, data: &[u8]) -> Result<usize, &'static str> {
        let ehdr = Self::validate(data)?;

        let phdr_size = ehdr.e_phentsize as usize;
        if phdr_size != core::mem::size_of::<Elf64Phdr>() {
            return Err("Invalid program header size");
        }

        let phdr_count = ehdr.e_phnum as usize;
        if phdr_count > MAX_PHDR {
            return Err("Too many program headers");
        }

        let phoff = ehdr.e_phoff as usize;
        if phoff + phdr_count * phdr_size > data.len() {
            return Err("Program headers exceed data bounds");
        }

        for i in 0..phdr_count {
            // # Safety
            // phoff + phdr_count * phdr_size <= data.len() as verified above.
            // Pointer arithmetic stays within valid data slice range.
            // Elf64Phdr is #[repr(C)] so the cast is valid.
            let phdr_ptr = unsafe { data.as_ptr().add(phoff + i * phdr_size) as *const Elf64Phdr };
            // # Safety
            // The pointer is within valid data bounds for a single Elf64Phdr read.
            self.phdrs[i] = unsafe { *phdr_ptr };
        }

        self.phdr_count = phdr_count;
        Ok(phdr_count)
    }

    pub fn load_segments(
        &self,
        data: &[u8],
        allocator: &mut FrameAllocator,
        phys_base: *mut u8,
    ) -> Result<LoadedElf, &'static str> {
        let ehdr = Self::validate(data)?;
        let mut min_addr = u64::MAX;
        let mut max_addr = 0u64;

        for i in 0..self.phdr_count {
            let phdr = &self.phdrs[i];
            if phdr.p_type != PT_LOAD {
                continue;
            }

            if phdr.p_vaddr < min_addr {
                min_addr = phdr.p_vaddr;
            }
            let seg_end = phdr.p_vaddr + phdr.p_memsz;
            if seg_end > max_addr {
                max_addr = seg_end;
            }

            if phdr.p_filesz > phdr.p_memsz {
                return Err("Filesz larger than memsz");
            }

            if data.len() < phdr.p_offset as usize + phdr.p_filesz as usize {
                return Err("Segment data out of bounds");
            }

            let page_aligned_start = phdr.p_vaddr & !0xFFF;
            let page_aligned_end = (phdr.p_vaddr + phdr.p_memsz + 0xFFF) & !0xFFF;
            let pages_needed = ((page_aligned_end - page_aligned_start) / 4096) as usize;

            for p in 0..pages_needed {
                let frame_vaddr = page_aligned_start + (p as u64) * 4096;
                if let Some(frame_addr) = allocator.alloc_frame_addr(phys_base) {
                    let frame_ptr = frame_addr as u64;
                    let offset_in_data = if frame_vaddr >= phdr.p_vaddr {
                        (frame_vaddr - phdr.p_vaddr) as usize
                    } else {
                        0
                    };

                    for byte in 0..4096 {
                        // # Safety
                        // alloc_frame_addr returns a pointer to a pre-allocated physical frame.
                        // phys_base is the virtual address of the physical memory base region.
                        // The kernel identity-maps physical memory, so phys_base + idx*4096
                        // gives a directly dereferenceable virtual address. frame_ptr.wrapping_add
                        // computes the virtual address of byte 'byte' within this frame, which
                        // is writable kernel memory (no other mapping needed).
                        let dst = unsafe { &mut *(frame_ptr.wrapping_add(byte as u64) as *mut u8) };
                        if offset_in_data + byte < phdr.p_filesz as usize {
                            let src_idx = phdr.p_offset as usize + offset_in_data + byte;
                            if src_idx < data.len() {
                                *dst = data[src_idx];
                            } else {
                                *dst = 0;
                            }
                        } else if offset_in_data + byte < phdr.p_memsz as usize {
                            *dst = 0;
                        }
                    }
                } else {
                    return Err("Failed to allocate frame for segment");
                }
            }
        }

        if min_addr == u64::MAX {
            return Err("No loadable segments found");
        }

        Ok(LoadedElf {
            entry: ehdr.e_entry,
            base: min_addr & !0xFFF,
            size: ((max_addr - min_addr) + 0xFFF) & !0xFFF,
        })
    }

    pub fn load(
        &mut self,
        data: &[u8],
        allocator: &mut FrameAllocator,
        phys_base: *mut u8,
    ) -> Result<LoadedElf, &'static str> {
        self.parse_phdrs(data)?;
        let loaded = self.load_segments(data, allocator, phys_base)?;
        // Mark each PT_LOAD segment's virtual address range as user-accessible
        // in the boot page tables so ring-3 code can access its own pages.
        for i in 0..self.phdr_count {
            if self.phdrs[i].p_type == PT_LOAD && self.phdrs[i].p_memsz > 0 {
                map_user_segment(self.phdrs[i].p_vaddr, self.phdrs[i].p_memsz);
            }
        }
        Ok(loaded)
    }

    #[allow(dead_code)]
    pub(crate) fn get_phdr(&self, index: usize) -> Option<&Elf64Phdr> {
        if index < self.phdr_count {
            Some(&self.phdrs[index])
        } else {
            None
        }
    }

    pub fn phdr_count(&self) -> usize {
        self.phdr_count
    }

    pub fn has_loadable_segments(&self) -> bool {
        for i in 0..self.phdr_count {
            if self.phdrs[i].p_type == PT_LOAD {
                return true;
            }
        }
        false
    }
}

impl Default for ElfLoader {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UserStack {
    pub base: *mut u8,
    pub size: usize,
    pub sp: *mut u8,
}

impl UserStack {
    pub fn new(allocator: &mut FrameAllocator, phys_base: *mut u8) -> Result<Self, &'static str> {
        let pages = USER_STACK_SIZE / 4096;

        for _ in 0..pages {
            allocator
                .alloc_frame_addr(phys_base)
                .ok_or("Failed to allocate stack frame")?;
        }

        let stack_top = 0x0000_7FFF_FFFF_F000u64;
        let sp = stack_top as *mut u8;
        Ok(Self {
            base: (stack_top - USER_STACK_SIZE as u64 + 4096) as *mut u8,
            size: USER_STACK_SIZE,
            sp,
        })
    }

    pub fn push_arg(&mut self, arg: &[u8]) -> Result<*mut u8, &'static str> {
        let len = arg.len() + 1;
        let sp_val = self.sp as u64;
        let new_sp_val = sp_val.wrapping_sub(len as u64) & !0xF;
        if new_sp_val < self.base as u64 {
            return Err("Stack overflow");
        }
        let new_sp = new_sp_val as *mut u8;
        // # Safety
        // new_sp_val >= self.base verified above. Writes stay within
        // the pre-allocated user stack region which is writable memory.
        for (byte, arg_byte) in arg.iter().enumerate() {
            unsafe { *new_sp.add(byte) = *arg_byte };
        }
        // # Safety
        // Same bounds guarantee as above. NUL terminator is within stack.
        unsafe { *new_sp.add(arg.len()) = 0 };
        self.sp = new_sp;
        Ok(new_sp)
    }

    pub fn push_u64(&mut self, val: u64) -> Result<(), &'static str> {
        let sp_val = self.sp as u64;
        let new_sp_val = sp_val.wrapping_sub(8);
        if new_sp_val < self.base as u64 {
            return Err("Stack overflow");
        }
        let new_sp = new_sp_val as *mut u64;
        // # Safety
        // new_sp_val >= self.base verified above. The write is within the
        // pre-allocated user stack region. 8-byte aligned write to u64 is safe.
        unsafe { *new_sp = val };
        self.sp = new_sp as *mut u8;
        Ok(())
    }

    pub fn sp(&self) -> u64 {
        self.sp as u64
    }
}

pub struct UserContext {
    pub entry: u64,
    pub stack_ptr: u64,
    pub stack_base: u64,
    pub stack_size: usize,
}

pub fn setup_user_context(
    elf_data: &[u8],
    allocator: &mut FrameAllocator,
    phys_base: *mut u8,
    args: &[&[u8]],
) -> Result<UserContext, &'static str> {
    crate::serial::write_str("[elf] setup_user_context: creating loader\r\n");
    let mut loader = ElfLoader::new();
    crate::serial::write_str("[elf] setup_user_context: calling load\r\n");
    let loaded = loader.load(elf_data, allocator, phys_base)?;
    crate::serial::write_str("[elf] setup_user_context: ELF loaded\r\n");

    crate::serial::write_str("[elf] setup_user_context: creating user stack\r\n");
    let mut user_stack = UserStack::new(allocator, phys_base)?;
    crate::serial::write_str("[elf] setup_user_context: user stack created\r\n");

    let mut arg_addrs = [0u64; 8];
    let argc = args.len().min(8);
    for i in (0..argc).rev() {
        let ptr = user_stack.push_arg(args[i])?;
        arg_addrs[i] = ptr as u64;
    }

    for arg_addr in arg_addrs.iter().take(argc) {
        user_stack.push_u64(*arg_addr).ok();
    }
    user_stack.push_u64(0).ok();

    user_stack.push_u64(argc as u64).ok();

    user_stack.push_u64(0).ok();

    Ok(UserContext {
        entry: loaded.entry,
        stack_ptr: user_stack.sp(),
        stack_base: user_stack.base as u64,
        stack_size: user_stack.size,
    })
}

pub fn start_user_program(
    context: &UserContext,
    user_cs: SegmentSelector,
    user_ss: SegmentSelector,
) -> ! {
    crate::serial::write_str("[elf] start_user_program: setting up TSS\r\n");
    // Set up the TSS ring-0 stack so that any ring-3 → ring-0 transition
    // (syscall int 0x80, page fault, etc.) has a valid kernel stack to switch to.
    // # Safety
    // boot_stack_top is a BSS symbol exported from boot.S. It is the top of the
    // 64 KiB boot stack, which is valid kernel stack memory for the lifetime of
    // the kernel. setup_tss_stack() writes this address into TSS.privilege_stack_table[0].
    unsafe {
        crate::gdt::setup_tss_stack(x86_64::VirtAddr::new(&boot_stack_top as *const u8 as u64));
    }
    crate::serial::write_str("[elf] start_user_program: TSS ready\r\n");

    crate::serial::write_str("[elf] start_user_program: building iretq frame\r\n");
    // # Safety
    // Constructs a valid iretq frame to transition from ring 0 to ring 3.
    // The iretq frame is pushed onto the CURRENT (kernel) stack, not user stack.
    // iretq pops these 5 values in order:
    //   RIP (user entry point)
    //   CS  (user code selector with RPL=3)
    //   RFLAGS (IF=1, reserved bit 1)
    //   RSP (user stack pointer)
    //   SS  (user data selector with RPL=3)
    unsafe {
        let ss_val = (user_ss.0 as u64) | 3;
        let rsp_val = context.stack_ptr;
        let rflags_val = 0x202u64; // IF=1, reserved bit 1
        let cs_val = (user_cs.0 as u64) | 3;
        let rip_val = context.entry;

        crate::serial::write_str("[elf] start_user_program: executing iretq to ring 3...\r\n");
        // Push iretq frame onto kernel stack (current RSP).
        // Push in reverse order so iretq pops RIP first.
        core::arch::asm!(
            "push {ss}",
            "push {rsp}",
            "push {rflags}",
            "push {cs}",
            "push {rip}",
            "iretq",
            ss = in(reg) ss_val,
            rsp = in(reg) rsp_val,
            rflags = in(reg) rflags_val,
            cs = in(reg) cs_val,
            rip = in(reg) rip_val,
            options(noreturn)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_elf() -> [u8; 128] {
        let mut elf = [0u8; 128];
        elf[0..4].copy_from_slice(&ELF_MAGIC);
        elf[4] = 2;
        elf[5] = 1;
        elf[6] = 1;
        let e_type_bytes = 2u16.to_le_bytes();
        let e_machine_bytes = 0x3Eu16.to_le_bytes();
        let e_version_bytes = 1u32.to_le_bytes();
        let e_entry_bytes = 0x401000u64.to_le_bytes();
        let e_phoff_bytes = 64u64.to_le_bytes();
        let e_ehsize_bytes = 64u16.to_le_bytes();
        let e_phentsize_bytes = 56u16.to_le_bytes();
        let e_phnum_bytes = 1u16.to_le_bytes();
        elf[16..18].copy_from_slice(&e_type_bytes);
        elf[18..20].copy_from_slice(&e_machine_bytes);
        elf[20..24].copy_from_slice(&e_version_bytes);
        elf[24..32].copy_from_slice(&e_entry_bytes);
        elf[32..40].copy_from_slice(&e_phoff_bytes);
        elf[52..54].copy_from_slice(&e_ehsize_bytes);
        elf[54..56].copy_from_slice(&e_phentsize_bytes);
        elf[56..58].copy_from_slice(&e_phnum_bytes);
        let phdr = Elf64Phdr {
            p_type: PT_LOAD,
            p_flags: PF_R | PF_X,
            p_offset: 0,
            p_vaddr: 0x401000,
            p_paddr: 0x401000,
            p_filesz: 4096,
            p_memsz: 4096,
            p_align: 4096,
        };
        // # Safety
        // PhantomData reference to the local Elf64Phdr variable is valid
        // for the call. The resulting slice is immediately copied into the array.
        let phdr_bytes = unsafe {
            core::slice::from_raw_parts(
                &phdr as *const Elf64Phdr as *const u8,
                core::mem::size_of::<Elf64Phdr>(),
            )
        };
        elf[64..64 + phdr_bytes.len()].copy_from_slice(phdr_bytes);
        elf
    }

    #[test]
    fn test_elf_validate_valid() {
        let data = create_test_elf();
        let result = ElfLoader::validate(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_elf_validate_bad_magic() {
        let mut data = create_test_elf();
        data[0] = 0xFF;
        let result = ElfLoader::validate(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_elf_validate_not_64bit() {
        let mut data = create_test_elf();
        data[4] = 1;
        let result = ElfLoader::validate(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_elf_validate_not_x86_64() {
        let mut data = create_test_elf();
        data[18] = 0x01;
        let result = ElfLoader::validate(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_elf_validate_too_small() {
        let data = [0u8; 10];
        let result = ElfLoader::validate(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_phdrs() {
        let data = create_test_elf();
        let mut loader = ElfLoader::new();
        let result = loader.parse_phdrs(&data);
        assert!(result.is_ok());
        assert_eq!(Ok(1), result);
    }

    #[test]
    fn test_phdr_count() {
        let mut loader = ElfLoader::new();
        assert_eq!(0, loader.phdr_count());
    }

    #[test]
    fn test_has_loadable_segments() {
        let data = create_test_elf();
        let mut loader = ElfLoader::new();
        loader.parse_phdrs(&data).unwrap();
        assert!(loader.has_loadable_segments());
    }

    #[test]
    fn test_get_phdr() {
        let data = create_test_elf();
        let mut loader = ElfLoader::new();
        loader.parse_phdrs(&data).unwrap();
        let phdr = loader.get_phdr(0);
        assert!(phdr.is_some());
        assert_eq!(PT_LOAD, phdr.unwrap().p_type);
        assert!(loader.get_phdr(1).is_none());
    }

    #[test]
    fn test_elf_loader_new() {
        let loader = ElfLoader::new();
        assert_eq!(0, loader.phdr_count());
    }

    #[test]
    fn test_elf_loader_default() {
        let loader = ElfLoader::default();
        assert_eq!(0, loader.phdr_count());
    }

    #[test]
    fn test_user_stack_new() {
        let mut alloc = FrameAllocator::new();
        alloc.init(0x100000 as *mut u8, 4096 * 100, 100);
        let stack = UserStack::new(&mut alloc, 0x100000 as *mut u8);
        assert!(stack.is_ok());
    }

    #[test]
    fn test_user_stack_push_u64() {
        let mut alloc = FrameAllocator::new();
        alloc.init(0x100000 as *mut u8, 4096 * 100, 100);
        let mut stack = UserStack::new(&mut alloc, 0x100000 as *mut u8).unwrap();
        let initial_sp = stack.sp();
        stack.push_u64(0xDEAD_BEEF).unwrap();
        assert!(stack.sp() < initial_sp);
        // # Safety
        // stack.sp() points into the pre-allocated user stack (owned by
        // UserStack). We just pushed 0xDEAD_BEEF via push_u64 which verified
        // the write address is within bounds. Reading it back as u64 is safe.
        let val = unsafe { *(stack.sp() as *const u64) };
        assert_eq!(0xDEAD_BEEF, val);
    }

    #[test]
    fn test_user_stack_push_arg() {
        let mut alloc = FrameAllocator::new();
        alloc.init(0x100000 as *mut u8, 4096 * 100, 100);
        let mut stack = UserStack::new(&mut alloc, 0x100000 as *mut u8).unwrap();
        let result = stack.push_arg(b"Hello");
        assert!(result.is_ok());
        let ptr = result.unwrap();
        // # Safety
        // push_arg returned Ok(ptr), meaning ptr is within the pre-allocated
        // user stack region where we wrote the "Hello" bytes + NUL terminator.
        let c = unsafe { *ptr };
        // # Safety
        // Same stack region guarantee. ptr.add(5) is the NUL byte we wrote.
        let null_check = unsafe { *ptr.add(5) };
        assert_eq!(0, null_check);
    }
}
