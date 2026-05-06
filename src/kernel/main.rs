// AIOS Kernel Entry Point
//
// Model: opencode
// Tool: opencode
// Prompt: Create x86_64 kernel entry point stub.

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use crate::{serial, gdt, interrupts, pic, memory, vmm, allocator};

/// Kernel entry point called by bootloader
#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial::init();
    gdt::init();
    interrupts::init();
    pic::init();
    memory::init(0 as *mut u8, 0, 0);
    vmm::init();
    unsafe { allocator::init(); }
    
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}