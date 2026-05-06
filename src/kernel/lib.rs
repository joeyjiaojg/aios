// AIOS Kernel Library
//
// Model: MiniMax M2.5 Free
// Tool: opencode
// Prompt: Create kernel library root module exporting core functionality
//         including VMM and allocator.

#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(asm_experimental_arch)]

#[macro_use]
pub mod serial;

pub mod allocator;
pub mod gdt;
pub mod interrupts;
pub mod keyboard;
pub mod main;
pub mod memory;
pub mod pic;
pub mod task;
pub mod vga;
pub mod vmm;
