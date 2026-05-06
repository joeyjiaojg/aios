// AIOS Kernel Library
//
// Model: opencode
// Tool: opencode
// Prompt: Create kernel library root module exporting core functionality
//         including VMM and allocator.

#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(allocator_api)]

#[macro_use]
pub mod serial;
#[macro_use]
pub mod main;
pub mod vga;
pub mod memory;
pub mod gdt;
pub mod interrupts;
pub mod vmm;
pub mod allocator;
pub mod task;
pub mod keyboard;
pub mod pic;

pub mod boot_info {
    use core::slice;

    #[repr(C)]
    pub struct BootInfo {
        pub memory_map: MemoryMap,
    }

    #[repr(C)]
    pub struct MemoryMap {
        pub entries: *const MemoryRegion,
        pub len: usize,
    }

    impl MemoryMap {
        pub fn iter(&self) -> impl Iterator<Item = &MemoryRegion> {
            unsafe {
                slice::from_raw_parts(self.entries, self.len).iter()
            }
        }

        pub fn len(&self) -> usize {
            self.len
        }
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct MemoryRegion {
        pub start_addr: u64,
        pub len: u64,
        pub region_type: MemoryRegionType,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MemoryRegionType {
        Usable = 1,
        Reserved = 2,
        AcpiReclaimable = 3,
        AcpiNvs = 4,
        BadMemory = 5,
    }
}
