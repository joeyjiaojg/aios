// AIOS Kernel Library
//
// Model: MiniMax M2.5 Free
// Tool: opencode
// Prompt: Create kernel library root module exporting core functionality
//         including VMM and allocator.

#![no_std]

pub mod main;
pub mod serial;
pub mod vga;
pub mod memory;
pub mod gdt;
pub mod interrupts;
pub mod vmm;
pub mod allocator;
pub mod task;
