// AIOS Kernel Heap Allocator
//
// Model: opencode
// Tool: opencode
// Prompt: Create kernel heap allocator with tests for no_std Rust.

use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub use crate::vmm::HEAP_SIZE;

/// Initialize the kernel heap
///
/// # Safety
/// Must only be called once during boot. Heap memory must be mapped.
pub unsafe fn init() {
    use crate::vmm::HEAP_START;
    let heap_bottom = HEAP_START.as_u64() as *mut u8;
    let heap_size = crate::vmm::HEAP_SIZE as usize;
    ALLOCATOR.lock().init(heap_bottom, heap_size);
}

/// Allocate with alignment
pub fn alloc_aligned(layout: core::alloc::Layout) -> Result<*mut u8, core::alloc::AllocError> {
    use core::alloc::GlobalAlloc;
    let ptr = unsafe { ALLOCATOR.alloc(layout) };
    if ptr.is_null() {
        Err(core::alloc::AllocError)
    } else {
        Ok(ptr)
    }
}

/// Free allocation
pub fn dealloc(ptr: *mut u8, layout: core::alloc::Layout) {
    use core::alloc::GlobalAlloc;
    unsafe { ALLOCATOR.dealloc(ptr, layout) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocator_initialized() {
        use crate::vmm::HEAP_SIZE;
        assert!(HEAP_SIZE > 0);
    }

    #[test]
    fn test_layout_creation() {
        let layout = core::alloc::Layout::new::<u8>();
        assert_eq!(layout.size(), 1);
    }

    #[test]
    fn test_layout_alignment() {
        let layout = core::alloc::Layout::new::<u64>();
        assert!(layout.align() >= 8);
    }
}