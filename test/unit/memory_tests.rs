// AIOS Unit Tests - Memory Manager
//
// Model: opencode
// Tool: opencode
// Prompt: Create comprehensive unit tests for physical memory manager.

#[cfg(test)]
mod memory_tests {
    use crate::kernel::memory::{FrameAllocator, PAGE_SIZE};

    #[test]
    fn test_basic_allocation() {
        let mut bitmap = [0u8; 16];
        let total_pages = 16 * 8;

        let mut alloc = unsafe { FrameAllocator::new(bitmap.as_mut_ptr(), 16, total_pages) };

        let frame = alloc.alloc_frame().expect("Failed to allocate");
        assert_eq!(frame, 0);
        assert_eq!(alloc.free_pages(), total_pages - 1);
    }

    #[test]
    fn test_allocation_returns_lowest_available() {
        let mut bitmap = [0u8; 8];
        let total_pages = 64;

        let mut alloc = unsafe { FrameAllocator::new(bitmap.as_mut_ptr(), 8, total_pages) };

        assert_eq!(alloc.alloc_frame(), Some(0));
        assert_eq!(alloc.alloc_frame(), Some(1));
        assert_eq!(alloc.alloc_frame(), Some(2));
    }

    #[test]
    fn test_deallocation_reuses_frame() {
        let mut bitmap = [0u8; 4];
        let total_pages = 32;

        let mut alloc = unsafe { FrameAllocator::new(bitmap.as_mut_ptr(), 4, total_pages) };

        let frame = alloc.alloc_frame().expect("Failed");
        unsafe { alloc.dealloc_frame(frame) };

        let reused = alloc.alloc_frame().expect("Failed after dealloc");
        assert_eq!(reused, frame);
    }

    #[test]
    fn test_exhaustion_returns_none() {
        let mut bitmap = [0u8; 1];
        let total_pages = 8;

        let mut alloc = unsafe { FrameAllocator::new(bitmap.as_mut_ptr(), 1, total_pages) };

        for _ in 0..8 {
            alloc.alloc_frame();
        }

        assert!(alloc.alloc_frame().is_none());
    }

    #[test]
    fn test_alloc_frame_addr_alignment() {
        let mut bitmap = [0u8; 8];
        let total_pages = 64;

        let mut alloc = unsafe { FrameAllocator::new(bitmap.as_mut_ptr(), 8, total_pages) };

        let addr = alloc.alloc_frame_addr().expect("Failed");
        assert_eq!(addr % PAGE_SIZE, 0);
        assert_eq!(addr, 0);
    }

    #[test]
    fn test_free_count_accurate() {
        let mut bitmap = [0u8; 16];
        let total_pages = 128;

        let mut alloc = unsafe { FrameAllocator::new(bitmap.as_mut_ptr(), 16, total_pages) };

        let initial_free = alloc.free_pages();
        assert_eq!(initial_free, total_pages);

        alloc.alloc_frame();
        alloc.alloc_frame();
        alloc.alloc_frame();

        assert_eq!(alloc.free_pages(), total_pages - 3);
    }
}
