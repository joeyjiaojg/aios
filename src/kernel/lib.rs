// AIOS Kernel Library
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Create kernel library root module exporting core functionality.

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(asm_experimental_arch)]
#![allow(unused_features)]
#![allow(static_mut_refs)]

extern crate alloc;
extern crate spin;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::write_str("\r\n[PANIC] ");
    if let Some(location) = info.location() {
        serial::write_str("at ");
        serial::write_str(location.file());
        serial::write_str(":");
        // Print line number
        let line = location.line();
        let mut buf = [0u8; 10];
        let mut n = line;
        let mut i = 0;
        if n == 0 {
            buf[0] = b'0';
            i = 1;
        } else {
            while n > 0 {
                buf[i] = b'0' + (n % 10) as u8;
                n /= 10;
                i += 1;
            }
        }
        while i > 0 {
            i -= 1;
            serial::write_byte(buf[i]);
        }
    }
    serial::write_str("\r\n");
    loop {}
}

#[macro_use]
pub mod serial;

pub mod allocator;
pub mod ata;
pub mod debug;
pub mod driver;
pub mod e1000;
pub mod elf;
pub mod ext2;
pub mod gdt;
pub mod interrupts;
pub mod keyboard;
pub mod memory;
pub mod multiboot2;
pub mod network;
pub mod pci;
pub mod pic;
pub mod process;
pub mod ramdisk;
pub mod shell;
pub mod syscalls;
pub mod task;
pub mod user_init;
pub mod vfs;
pub mod vga;
pub mod vmm;

// Kernel entry point (called from boot.S after entering 64-bit mode)
mod kernel_entry;
