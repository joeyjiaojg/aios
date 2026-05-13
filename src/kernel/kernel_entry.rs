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

    crate::vmm::init();
    println!("[aios] VMM initialized");

    crate::interrupts::init();
    crate::interrupts::init_idt();
    println!("[aios] Interrupts initialized");

    crate::process::init();
    crate::task::init_scheduler();
    println!("[aios] Scheduler initialized");

    crate::syscalls::init();

    // Parse multiboot2 modules and initialize ramdisk
    println!("[aios] Parsing multiboot2 modules...");
    crate::multiboot2::parse_modules(mbi_ptr as *const u8);
    crate::ramdisk::init_from_modules();
    crate::ramdisk::list_files();

    crate::interrupts::enable_interrupts();
    println!("[aios] Starting shell");

    crate::shell::run_shell();

    loop {
        crate::task::run_scheduler();
        crate::interrupts::enable_interrupts();
        // # Safety
        // HLT in the idle loop is safe; interrupts are enabled above.
        unsafe { core::arch::asm!("hlt") }
    }
}
