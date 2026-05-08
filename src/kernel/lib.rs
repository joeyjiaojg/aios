// AIOS Kernel Library
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Create kernel library root module exporting core functionality.

#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(asm_experimental_arch)]
#![allow(unused_features)]
#![allow(static_mut_refs)]

extern crate alloc;
extern crate spin;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[macro_use]
pub mod serial;

pub mod allocator;
pub mod ata;
pub mod ext2;
pub mod gdt;
pub mod interrupts;
pub mod keyboard;
pub mod memory;
pub mod network;
pub mod pic;
pub mod ramdisk;
pub mod syscalls;
pub mod task;
pub mod vga;
pub mod vmm;
