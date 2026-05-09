// AIOS Kernel Entry Point
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Create x86_64 kernel entry point with scheduler integration

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

    // Initialize interrupt subsystem and configure timer
    crate::interrupts::init();
    crate::interrupts::init_idt();

    // Initialize task manager
    crate::process::init();
    crate::syscalls::init();

    // Enable interrupts to start timer and scheduler
    crate::interrupts::enable_interrupts();

    // Run the scheduler with idle task
    crate::task::run_scheduler();
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        hlt();
    }
}
