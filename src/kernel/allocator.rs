// AIOS Kernel Heap Allocator
//
// Model: MiniMax M2.5 Free
// Tool: opencode
// Prompt: Create kernel heap allocator with global allocator implementation
//         for no_std Rust, using linked list based allocator.

use linked_list_allocator::LockedHeap;
use x86_64::VirtAddr;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// HEAP_START and HEAP_SIZE are defined in vmm.rs
pub use crate::kernel::vmm::HEAP_SIZE;

/// Initialize the kernel heap
///
/// # Safety
/// Must only be called once during boot
pub unsafe fn init() {
    use x86_64::structures::paging::{Mapper, Page, Size4KiB, PageTableFlags};
    use crate::kernel::vmm::HEAP_START;

    let heap_start_page = Page::containing_address(HEAP_START);
    let heap_end_page = Page::containing_address(HEAP_START + HEAP_SIZE as u64 - 1);

    // Heap pages are mapped by init_paging() in vmm.rs

    // Initialize the heap allocator
    // SAFETY: HEAP_START is a valid virtual address mapped by the VMM,
    // and HEAP_SIZE is the correct size
    ALLOCATOR.lock().init(HEAP_START.as_u64() as usize, HEAP_SIZE as usize);
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

/// Get heap usage statistics
pub fn heap_usage() -> (usize, usize) {
    let guard = ALLOCATOR.lock();
    let total = HEAP_SIZE as usize;
    let used = guard.used();
    (used, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::alloc::Layout;

    #[test]
    fn test_heap_constants() {
        use crate::kernel::vmm::HEAP_SIZE;
        assert_eq!(HEAP_SIZE, 100 * 1024 * 1024);
        assert!(HEAP_SIZE > 0);
    }

    #[test]
    fn test_layout_creation() {
        let layout = Layout::new::<u8>();
        assert_eq!(layout.size(), 1);
        assert_eq!(layout.align(), 1);

        let layout = Layout::new::<u64>();
        assert_eq!(layout.size(), 8);
        assert_eq!(layout.align(), 8);
    }

    #[test]
    fn test_large_layout() {
        let layout = Layout::array::<u8>(1024).unwrap();
        assert_eq!(layout.size(), 1024);
        assert_eq!(layout.align(), 1);
    }
}
