// AIOS Kernel Entry Point
//
// Model: opencode
// Tool: opencode
// Prompt: Create x86_64 kernel entry point with boot info parsing,
//         serial port initialization, and kernel main loop.

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate boot_info;

use core::panic::PanicInfo;

mod serial;
mod vga;

/// Kernel entry point called by bootloader
#[no_mangle]
pub extern "C" fn _start(boot_info: &'static boot_info::BootInfo) -> ! {
    // Initialize serial port for early debug output
    serial::init();

    println!("AIOS - AI-Generated Operating System");
    println!("Booting on x86_64...");
    println!("Memory map entries: {}", boot_info.memory_map.len());

    // TODO: Initialize memory manager
    // TODO: Initialize interrupt controller
    // TODO: Initialize scheduler

    println!("Kernel initialization complete. Halting.");

    // Halt the CPU until next interrupt
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
