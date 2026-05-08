// AIOS Kernel Entry Point
//
// Model: MiniMax M2.5 Free
// Tool: opencode
// Prompt: Create x86_64 kernel entry point with heap initialization.

use crate::allocator;
use x86_64::VirtAddr;

#[inline]
fn hlt() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[repr(C)]
pub struct BootInfo {
    pub memory_map: crate::boot_info::MemoryMap,
}

#[repr(C)]
pub struct MemoryMap {
    pub entries: *const (),
    pub len: usize,
}

impl MemoryMap {
    pub fn iter(&self) -> core::slice::Iter<'static, ()> {
        unsafe { core::slice::from_raw_parts(self.entries, self.len).iter() }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    // Initialize the heap allocator using the first usable memory region.
    // Safety: We initialize the heap at the first usable memory region provided by the bootloader.
    // The address is guaranteed to be valid and available for heap use by the bootloader contract.
    let heap_start = boot_info
        .memory_map
        .iter()
        .find(|region| region.region_type == crate::boot_info::MemoryRegionType::Usable)
        .map(|region| region.start_addr)
        .expect("No usable memory region found");

    let heap_size = 1024 * 1024; // 1 MiB heap
    unsafe {
        // Safety: The heap_start address comes from the bootloader-provided memory map
        // and is guaranteed to be a valid, usable memory region
        allocator::init(VirtAddr::new(heap_start), heap_size)
    };

    crate::process::init();
    crate::syscalls::init();
    crate::shell::run_shell();

    loop {
        hlt();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        hlt();
    }
}
