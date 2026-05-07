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
    let heap_start = 0x_0010_0000_0000u64; // 1TB offset, choose a high address to avoid conflicts
    let heap_size = 1024 * 1024; // 1 MiB heap
    unsafe { allocator::init(VirtAddr::new(heap_start), heap_size) };

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