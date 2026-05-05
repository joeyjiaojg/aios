// AIOS Boot Information Library
//
// Model: opencode
// Tool: opencode
// Prompt: Create boot_info library to parse bootloader-provided memory map and boot parameters.

#![no_std]

use core::slice;

/// Boot information passed from bootloader
#[repr(C)]
pub struct BootInfo {
    pub memory_map: MemoryMap,
}

/// Memory map from bootloader
#[repr(C)]
pub struct MemoryMap {
    pub entries: *const MemoryRegion,
    pub len: usize,
}

impl MemoryMap {
    /// Get an iterator over memory regions
    pub fn iter(&self) -> impl Iterator<Item = &MemoryRegion> {
        unsafe {
            slice::from_raw_parts(self.entries, self.len).iter()
        }
    }

    /// Get the length of the memory map
    pub fn len(&self) -> usize {
        self.len
    }
}

/// Single memory region
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start_addr: u64,
    pub len: u64,
    pub region_type: MemoryRegionType,
}

/// Type of memory region
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// Usable RAM
    Usable = 1,
    /// Reserved (do not use)
    Reserved = 2,
    /// ACPI reclaimable memory
    AcpiReclaimable = 3,
    /// ACPI NVS memory
    AcpiNvs = 4,
    /// Bad memory (do not use)
    BadMemory = 5,
}
