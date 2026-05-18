// AIOS Kernel Entry Point
//
// Model: claude-sonnet-4-6
// Tool: claude-code
// Prompt: Fix multiboot2 kernel_main signature and early serial init for boot debugging.

#[no_mangle]
pub extern "C" fn kernel_main(mbi_ptr: u64) -> ! {
    // Initialize serial first so we can see output before anything else runs.
    crate::serial::init();
    println!("[aios] kernel_main entered, mbi={:#x}", mbi_ptr);

    crate::gdt::init();
    println!("[aios] GDT initialized");

    // Enable SSE in CR4 and CR0 so user-mode code can use XMM registers.
    // CR4.OSFXSR (bit 9)    - required for SSE/SSE2 in 64-bit mode
    // CR4.OSXMMEXCPT (bit 10) - enable #XF (#SIMD) exception
    // Also clear CR0.EM (bit 2) and CR0.TS (bit 3) to allow FPU/SSE access.
    // Without CR4.OSFXSR, any SSE instruction (e.g. xorps) in user mode generates #UD.
    // # Safety: writing CR0/CR4 is safe here; no other code runs concurrently at boot.
    unsafe {
        core::arch::asm!(
            // Clear CR0.EM (bit 2) and CR0.TS (bit 3), set CR0.MP (bit 1)
            "mov rax, cr0",
            "and rax, ~(1 << 2)",  // clear EM
            "and rax, ~(1 << 3)",  // clear TS
            "or  rax, (1 << 1)",   // set MP
            "mov cr0, rax",
            // Set CR4.OSFXSR (bit 9) and CR4.OSXMMEXCPT (bit 10)
            "mov rax, cr4",
            "or rax, (3 << 9)",    // bits 9 and 10
            "mov cr4, rax",
            out("rax") _,
        );
    }

    // Enable CR4.FSGSBASE (bit 16) if supported, so user-mode code can use
    // wrfsbase/rdfsbase for TLS base updates (used by musl-built static binaries).
    // Check CPUID.7.0:EBX[0] before setting the bit.
    // # Safety: reading CPUID and writing CR4 are safe in ring-0 during single-threaded init.
    unsafe {
        let cpuid7_ebx: u32;
        core::arch::asm!(
            "push rbx",
            "mov eax, 7",
            "xor ecx, ecx",
            "cpuid",
            "mov {0:e}, ebx",
            "pop rbx",
            out(reg) cpuid7_ebx,
            out("eax") _,
            out("ecx") _,
            out("edx") _,
        );
        if cpuid7_ebx & 1 != 0 {
            core::arch::asm!(
                "mov rax, cr4",
                "or rax, 0x10000",
                "mov cr4, rax",
                out("rax") _,
            );
        }
    }

    crate::vmm::init();
    println!("[aios] VMM initialized");

    crate::interrupts::init();
    crate::interrupts::init_idt();
    println!("[aios] Interrupts initialized");

    crate::process::init();
    crate::task::init_scheduler();
    println!("[aios] Scheduler initialized");

    crate::syscalls::init();
    crate::interrupts::init_syscall();

    // Parse multiboot2 modules and initialize ramdisk
    println!("[aios] Parsing multiboot2 modules...");
    unsafe { crate::multiboot2::parse_modules(mbi_ptr as *const u8) };
    crate::ramdisk::init_from_modules();
    crate::ramdisk::list_files();

    crate::interrupts::enable_interrupts();

    // Attempt to launch /bin/sh as the default interactive shell.
    // If /bin/sh is present, exec_cmd does `iretq` into ring 3 and control
    // returns here only on error.  On successful exec the process exit path
    // fires PROCESS_EXITED and jmps to shell_prompt_loop_entry().
    // If /bin/sh is absent, run_shell() falls back to the built-in shell.
    if crate::ramdisk::lookup_file("/bin/sh").is_some() {
        println!("[aios] /bin/sh found — exec-ing external shell");
    } else {
        println!("[aios] /bin/sh not found — starting built-in shell");
    }

    crate::shell::run_shell();

    loop {
        crate::task::run_scheduler();
        crate::interrupts::enable_interrupts();
        // # Safety
        // HLT in the idle loop is safe; interrupts are enabled above.
        unsafe { core::arch::asm!("hlt") }
    }
}
