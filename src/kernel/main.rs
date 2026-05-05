// AIOS Kernel Entry Point
//
// Model: MiniMax M2.5 Free
// Tool: opencode
// Prompt: Create x86_64 kernel entry point with boot info parsing,
//         serial port initialization, GDT/IDT/VMM/allocator setup, and kernel main loop.

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate boot_info;

use core::panic::PanicInfo;
use x86_64::VirtAddr;

mod serial;
mod vga;

/// Kernel entry point called by bootloader
#[no_mangle]
pub extern "C" fn _start(boot_info: &'static boot_info::BootInfo) -> ! {
    // Initialize serial port for early debug output
    serial::init();

    println!("AIOS - AI-Generated Operating System");
    println!("Booting on x86_64...");

    // Initialize GDT
    println!("[INIT] Setting up GDT...");
    crate::kernel::gdt::init();

    // Initialize interrupts
    println!("[INIT] Setting up interrupts...");
    crate::kernel::interrupts::init();

    // Initialize PIC
    println!("[INIT] Initializing PIC...");
    crate::kernel::pic::init();

    // Initialize memory manager
    println!("[INIT] Setting up physical memory manager...");
    crate::kernel::memory::init(
        boot_info.memory_map.entries as *mut u8,
        boot_info.memory_map.len(),
        0x100000, // TODO: Calculate from actual memory map
    );

    // Initialize virtual memory manager
    println!("[INIT] Setting up virtual memory manager...");
    crate::kernel::vmm::init();

    // Initialize kernel heap
    println!("[INIT] Initializing kernel heap...");
    unsafe {
        crate::kernel::allocator::init();
    }

    println!("[INIT] Kernel initialization complete.");
    println!("[INIT] Memory map entries: {}", boot_info.memory_map.len());

    // Print VMM memory map
    if let Some(vmm) = crate::kernel::vmm::VMM.lock().as_ref() {
        vmm.print_memory_map();
    }

    // Kernel main loop - halt until next interrupt
    loop {
        x86_64::instructions::hlt();
    }
}

/// Global panic handler
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[PANIC] {}", info);
    loop {
        x86_64::instructions::hlt();
    }
}
