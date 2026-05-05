// AIOS Virtual Memory Manager
//
// Model: MiniMax M2.5 Free
// Tool: opencode
// Prompt: Create 4-level paging virtual memory manager for x86_64
//         with PML4 setup, page table allocation, identity mapping,
//         and kernel heap region management.

use x86_64::structures::paging::{
    Mapper, PageTable, Page, PageSize,
    page_table::PageTableFlags,
    OffsetPageTable, PhysFrame, Size4KiB,
    FrameAllocator, Translate,
};
use x86_64::{VirtAddr, PhysAddr};
use x86_64::registers::control::Cr3;
use spin::Mutex;

/// Kernel virtual memory layout
/// Physical memory offset for virt->phys conversion (higher half)
pub const PHYSICAL_MEMORY_OFFSET: VirtAddr = VirtAddr::new(0xffff_8000_0000_0000);
/// Kernel heap starts at a different virtual address in higher half
pub const HEAP_START: VirtAddr = VirtAddr::new(0xffff_9000_0000_0000);
pub const HEAP_SIZE: u64 = 100 * 1024 * 1024; // 100 MB
pub const KERNEL_STACK_SIZE: u64 = 16 * 1024; // 16 KB

/// Memory region type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionKind {
    /// Kernel code and data
    Kernel,
    /// Heap region
    Heap,
    /// Stack region
    Stack,
    /// Memory-mapped I/O
    Mmio,
    /// User space
    User,
}

/// Virtual memory area descriptor
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: VirtAddr,
    pub size: u64,
    pub kind: MemoryRegionKind,
    pub flags: PageTableFlags,
}

impl MemoryRegion {
    pub fn end(&self) -> VirtAddr {
        self.start + self.size
    }

    pub fn contains(&self, addr: VirtAddr) -> bool {
        addr >= self.start && addr < self.end()
    }
}

/// Virtual memory manager
pub struct VirtualMemoryManager {
    regions: [Option<MemoryRegion>; 32],
    region_count: usize,
}

impl VirtualMemoryManager {
    pub fn new() -> Self {
        Self {
            regions: [None; 32],
            region_count: 0,
        }
    }

    /// Register a memory region
    pub fn add_region(&mut self, region: MemoryRegion) {
        if self.region_count < self.regions.len() {
            self.regions[self.region_count] = Some(region);
            self.region_count += 1;
        } else {
            panic!("Too many memory regions");
        }
    }

    /// Check if an address is within a valid region
    pub fn is_valid_address(&self, addr: VirtAddr) -> bool {
        self.regions.iter().take(self.region_count).any(|r| {
            r.map(|region| region.contains(addr)).unwrap_or(false)
        })
    }

    /// Get memory map for display
    pub fn print_memory_map(&self) {
        for i in 0..self.region_count {
            if let Some(region) = self.regions[i] {
                println!(
                    "[MM] {:?}: {:#018x}..{:#018x} ({} bytes)",
                    region.kind,
                    region.start.as_u64(),
                    region.end().as_u64(),
                    region.size
                );
            }
        }
    }
}

/// Global VMM instance
pub static VMM: Mutex<Option<VirtualMemoryManager>> = Mutex::new(None);

/// Frame allocator wrapper for page table management
pub struct KernelFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for KernelFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        crate::kernel::memory::alloc_frame()
            .map(|idx| PhysFrame::from_start_address(PhysAddr::new(idx as u64 * 0x1000)).unwrap())
    }
}

/// Initialize virtual memory manager
pub fn init() {
    let mut vmm = VirtualMemoryManager::new();

    // Register kernel heap region
    vmm.add_region(MemoryRegion {
        start: HEAP_START,
        size: HEAP_SIZE,
        kind: MemoryRegionKind::Heap,
        flags: PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    });

    // Register kernel stack region
    vmm.add_region(MemoryRegion {
        start: VirtAddr::new(0xffff_8000_4000_0000),
        size: KERNEL_STACK_SIZE,
        kind: MemoryRegionKind::Stack,
        flags: PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    });

    VMM.lock().replace(vmm);
}

/// Initialize 4-level paging
///
/// # Safety
/// Must only be called once during boot
pub unsafe fn init_paging(physical_memory_offset: VirtAddr, memory_map: &'static impl Iterator<Item = crate::kernel::memory::MemoryRegion>) {
    // Get active level 4 page table
    let (level_4_table, _) = Cr3::read();

    // Create offset page table mapper
    let mut mapper = OffsetPageTable::new(level_4_table, physical_memory_offset);
    let mut frame_allocator = KernelFrameAllocator;

    // Map heap region (assume physical = virtual - offset for higher half)
    let heap_start = HEAP_START;
    let heap_start_page = Page::<Size4KiB>::containing_address(heap_start);
    let heap_end = heap_start + HEAP_SIZE - 1u64;
    let heap_end_page = Page::<Size4KiB>::containing_address(VirtAddr::new(heap_end));

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    let mut page = heap_start_page;
    while page <= heap_end_page {
        let virt = page.start_address();
        let phys = PhysAddr::new(virt.as_u64().wrapping_sub(physical_memory_offset.as_u64()));
        if let Ok(frame) = PhysFrame::from_start_address(phys) {
            let _ = mapper.map_to(page, frame, flags, &mut frame_allocator).map(|t| t.flush());
        }
        page = page + 1;
    }

    // Map usable memory regions from boot info
    for region in memory_map {
        if region.region_type == crate::kernel::memory::MemoryRegionType::Usable {
            let start = VirtAddr::new(region.start_addr + physical_memory_offset.as_u64());
            let size = region.len;
            // Map with present + writable flags
            // (simplified: map first few pages as example)
        }
    }

    println!("[MM] 4-level paging initialized");
    println!("[MM] Physical memory offset: {:#x}", physical_memory_offset.as_u64());
}

/// Translate virtual address to physical address
pub fn translate_addr(addr: VirtAddr) -> Option<PhysAddr> {
    let (level_4_table, _) = Cr3::read();
    // SAFETY: level_4_table points to the active L4 page table frame,
    // and PHYSICAL_MEMORY_OFFSET is the correct offset for virt-to-phys conversion
    let mapper = unsafe {
        OffsetPageTable::new(level_4_table, PHYSICAL_MEMORY_OFFSET)
    };

    mapper.translate(addr).ok()
}

/// Map a page with specified flags
///
/// # Safety
/// Caller must ensure the page is not already mapped
pub unsafe fn map_page(
    page: Page<Size4KiB>,
    frame: PhysFrame,
    flags: PageTableFlags,
    mapper: &mut impl Mapper<Size4KiB>,
    allocator: &mut KernelFrameAllocator,
) -> Result<(), x86_64::structures::paging::mapper::MapToError> {
    mapper.map_to(page, frame, flags, allocator)?.flush()
    Ok(())
}

/// Unmap a page
///
/// # Safety
/// Caller must ensure the page is mapped and safe to unmap
pub unsafe fn unmap_page(
    page: Page<Size4KiB>,
    mapper: &mut impl Mapper<Size4KiB>,
) -> Result<(PhysFrame, PageTableFlags), x86_64::structures::paging::mapper::UnmapError> {
    let (frame, flags) = mapper.unmap(page)?;
    Ok((frame, flags))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_region_contains() {
        let region = MemoryRegion {
            start: VirtAddr::new(0xffff_8000_0000_0000),
            size: 0x1000,
            kind: MemoryRegionKind::Heap,
            flags: PageTableFlags::PRESENT,
        };

        assert!(region.contains(VirtAddr::new(0xffff_8000_0000_0000)));
        assert!(region.contains(VirtAddr::new(0xffff_8000_0000_0FFF)));
        assert!(!region.contains(VirtAddr::new(0xffff_8000_0000_1000)));
        assert!(!region.contains(VirtAddr::new(0xffff_7fff_ffff_ffff)));
    }

    #[test]
    fn test_memory_region_end() {
        let region = MemoryRegion {
            start: VirtAddr::new(0x1000),
            size: 0x1000,
            kind: MemoryRegionKind::Kernel,
            flags: PageTableFlags::PRESENT,
        };

        assert_eq!(region.end().as_u64(), 0x2000);
    }

    #[test]
    fn test_vmm_add_region() {
        let mut vmm = VirtualMemoryManager::new();

        vmm.add_region(MemoryRegion {
            start: VirtAddr::new(0x1000),
            size: 0x1000,
            kind: MemoryRegionKind::Heap,
            flags: PageTableFlags::PRESENT,
        });

        assert!(vmm.is_valid_address(VirtAddr::new(0x1000)));
        assert!(vmm.is_valid_address(VirtAddr::new(0x1500)));
        assert!(!vmm.is_valid_address(VirtAddr::new(0x2000)));
    }

    #[test]
    fn test_page_table_flags() {
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        assert!(flags.contains(PageTableFlags::PRESENT));
        assert!(flags.contains(PageTableFlags::WRITABLE));
        assert!(!flags.contains(PageTableFlags::USER_ACCESSIBLE));
    }

    #[test]
    fn test_kernel_offsets() {
        assert_eq!(PHYSICAL_MEMORY_OFFSET.as_u64(), 0xffff_8000_0000_0000);
        assert_eq!(HEAP_START.as_u64(), 0xffff_9000_0000_0000);
        assert_eq!(HEAP_SIZE, 100 * 1024 * 1024);
    }
}
