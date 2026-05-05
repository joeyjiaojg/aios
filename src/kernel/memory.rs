// AIOS Physical Memory Manager
//
// Model: opencode
// Tool: opencode
// Prompt: Create bitmap-based physical memory manager for x86_64
//         with page allocation and deallocation.

use spin::Mutex;

/// Page size in bytes (4KB)
pub const PAGE_SIZE: usize = 0x1000;

/// Physical memory manager using bitmap
pub struct FrameAllocator {
    bitmap: &'static mut [u8],
    total_pages: usize,
    used_pages: usize,
}

impl FrameAllocator {
    /// Create a new FrameAllocator
    ///
    /// # Safety
    /// This must only be called once during kernel initialization
    pub unsafe fn new(
        bitmap_start: *mut u8,
        bitmap_len: usize,
        total_pages: usize,
    ) -> Self {
        // Calculate required bitmap size (1 bit per page)
        let required_bytes = (total_pages + 7) / 8;
        assert!(bitmap_len >= required_bytes);

        // Zero out the bitmap
        core::ptr::write_bytes(bitmap_start, 0, required_bytes);

        FrameAllocator {
            bitmap: core::slice::from_raw_parts_mut(bitmap_start, bitmap_len),
            total_pages,
            used_pages: 0,
        }
    }

    /// Allocate a physical frame (4KB)
    /// Returns the frame index or None if out of memory
    pub fn alloc_frame(&mut self) -> Option<usize> {
        for i in self.used_pages..self.total_pages {
            if !self.is_page_used(i) {
                self.mark_page_used(i);
                self.used_pages += 1;
                return Some(i);
            }
        }
        None
    }

    /// Allocate a physical frame and return its address
    pub fn alloc_frame_addr(&mut self) -> Option<usize> {
        self.alloc_frame().map(|idx| idx * PAGE_SIZE)
    }

    /// Deallocate a physical frame
    ///
    /// # Safety
    /// The caller must ensure the frame is not in use
    pub unsafe fn dealloc_frame(&mut self, idx: usize) {
        assert!(idx < self.total_pages);
        self.mark_page_unused(idx);
        self.used_pages -= 1;
    }

    /// Deallocate a physical frame by address
    ///
    /// # Safety
    /// The caller must ensure the address is page-aligned and not in use
    pub unsafe fn dealloc_frame_addr(&mut self, addr: usize) {
        assert!(addr % PAGE_SIZE == 0);
        self.dealloc_frame(addr / PAGE_SIZE);
    }

    /// Get the number of free pages
    pub fn free_pages(&self) -> usize {
        self.total_pages - self.used_pages
    }

    /// Check if a page is used
    fn is_page_used(&self, idx: usize) -> bool {
        let byte = idx / 8;
        let bit = idx % 8;
        (self.bitmap[byte] & (1 << bit)) != 0
    }

    /// Mark a page as used
    fn mark_page_used(&mut self, idx: usize) {
        let byte = idx / 8;
        let bit = idx % 8;
        self.bitmap[byte] |= 1 << bit;
    }

    /// Mark a page as unused
    fn mark_page_unused(&mut self, idx: usize) {
        let byte = idx / 8;
        let bit = idx % 8;
        self.bitmap[byte] &= !(1 << bit);
    }
}

/// Global frame allocator (protected by mutex)
pub static FRAME_ALLOCATOR: Mutex<Option<FrameAllocator>> = Mutex::new(None);

/// Initialize the physical memory manager
pub fn init(bitmap_start: *mut u8, bitmap_len: usize, total_pages: usize) {
    let mut guard = FRAME_ALLOCATOR.lock();
    *guard = Some(unsafe { FrameAllocator::new(bitmap_start, bitmap_len, total_pages) });
}

/// Allocate a frame from the global allocator
pub fn alloc_frame() -> Option<usize> {
    let mut guard = FRAME_ALLOCATOR.lock();
    guard.as_mut()?.alloc_frame()
}

/// Allocate a frame address from the global allocator
pub fn alloc_frame_addr() -> Option<usize> {
    let mut guard = FRAME_ALLOCATOR.lock();
    guard.as_mut()?.alloc_frame_addr()
}

/// Deallocate a frame from the global allocator
pub fn dealloc_frame(idx: usize) {
    let mut guard = FRAME_ALLOCATOR.lock();
    if let Some(alloc) = guard.as_mut() {
        unsafe { alloc.dealloc_frame(idx) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_dealloc() {
        let mut bitmap = [0u8; 64];
        let total_pages = 64 * 8; // 512 pages

        let mut alloc = unsafe { FrameAllocator::new(bitmap.as_mut_ptr(), 64, total_pages) };

        // Allocate a frame
        let frame = alloc.alloc_frame().expect("Failed to allocate frame");
        assert_eq!(frame, 0);

        // Allocate another frame
        let frame2 = alloc.alloc_frame().expect("Failed to allocate second frame");
        assert_eq!(frame2, 1);

        // Deallocate first frame
        unsafe { alloc.dealloc_frame(frame) };

        // Allocate again - should get the freed frame
        let frame3 = alloc.alloc_frame().expect("Failed to allocate third frame");
        assert_eq!(frame3, 0);
    }

    #[test]
    fn test_exhaustion() {
        let mut bitmap = [0u8; 1]; // Only 8 pages
        let total_pages = 8;

        let mut alloc = unsafe { FrameAllocator::new(bitmap.as_mut_ptr(), 1, total_pages) };

        // Allocate all frames
        for _ in 0..8 {
            assert!(alloc.alloc_frame().is_some());
        }

        // Should fail - out of memory
        assert!(alloc.alloc_frame().is_none());
    }
}
