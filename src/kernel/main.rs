// AIOS Kernel Entry Point
//
// Model: MiniMax M2.5 Free
// Tool: opencode
// Prompt: Create x86_64 kernel entry point stub.

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(asm_experimental_arch)]

#[inline]
fn hlt() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[repr(C)]
pub struct BootInfo {
    pub memory_map: MemoryMap,
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
pub extern "C" fn _start(_boot_info: &'static BootInfo) -> ! {
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
