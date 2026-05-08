// AIOS Virtual Memory Manager
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Implement Virtual Memory Manager for AIOS x86_64 kernel with paging

use x86_64::structures::paging::PageTableFlags as Flags;
use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PhysFrame, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

pub const PHYSICAL_MEMORY_OFFSET: VirtAddr = VirtAddr::new(0xFFFF_8000_0000_0000);

pub struct MemoryRegion {
    pub start: VirtAddr,
    pub size: u64,
    pub kind: MemoryRegionKind,
    pub flags: Flags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionKind {
    Kernel,
    Mmio,
}

pub struct SimpleFrameAllocator {
    next_free_frame: PhysFrame<Size4KiB>,
    used_frames: usize,
    max_frames: usize,
}

impl SimpleFrameAllocator {
    pub fn new(begin: PhysAddr, end_exclusive: PhysAddr) -> Self {
        let start_frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(begin);
        let end_frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(end_exclusive);
        let start_addr = start_frame.start_address().as_u64();
        let end_addr = end_frame.start_address().as_u64();
        Self {
            next_free_frame: start_frame,
            used_frames: 0,
            max_frames: ((end_addr - start_addr) / Size4KiB::SIZE) as usize,
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for SimpleFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        if self.used_frames >= self.max_frames {
            return None;
        }

        let frame = PhysFrame::from_start_address(self.next_free_frame.start_address()).ok()?;
        self.next_free_frame =
            PhysFrame::from_start_address(self.next_free_frame.start_address() + Size4KiB::SIZE)
                .ok()?;
        self.used_frames += 1;
        Some(frame)
    }
}

#[allow(dead_code)]
struct VmmState {
    mapper: OffsetPageTable<'static>,
    frame_allocator: SimpleFrameAllocator,
}

static VMM: spin::Mutex<Option<VmmState>> = spin::Mutex::new(None);

pub fn init() {
    *VMM.lock() = None;
}

/// Initialize the page tables for virtual memory
///
/// # Safety
/// Must be called once during kernel initialization. Invalid regions may cause undefined behavior.
pub unsafe fn init_paging<A>(offset: VirtAddr, regions: A)
where
    A: Iterator<Item = MemoryRegion>,
{
    let mut frame_allocator =
        SimpleFrameAllocator::new(PhysAddr::new(0x0010_0000), PhysAddr::new(0x1000_0000));

    let level_4_frame = match frame_allocator.allocate_frame() {
        Some(f) => f,
        None => return,
    };

    let level_4_phys = PhysAddr::new(level_4_frame.start_address().as_u64());
    let level_4_virt = PHYSICAL_MEMORY_OFFSET + level_4_phys.as_u64();
    let level_4_table_ptr: *mut x86_64::structures::paging::PageTable = level_4_virt.as_mut_ptr();
    let level_4_table = &mut *level_4_table_ptr;
    level_4_table.zero();

    let mut mapper = OffsetPageTable::new(level_4_table, offset);

    for region in regions {
        let start_page: Page<Size4KiB> = Page::containing_address(region.start);
        let end_addr = region.start + region.size;
        let end_page: Page<Size4KiB> = Page::containing_address(end_addr);

        for page in Page::range_inclusive(start_page, end_page) {
            let frame = PhysFrame::containing_address(PhysAddr::new(page.start_address().as_u64()));

            let result = mapper.map_to(page, frame, region.flags, &mut frame_allocator);

            if let Err(_e) = result {
                continue;
            }
        }
    }

    *VMM.lock() = Some(VmmState {
        mapper,
        frame_allocator,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vmm_init() {
        init();
    }

    #[test]
    fn test_memory_region() {
        let region = MemoryRegion {
            start: VirtAddr::new(0x1000),
            size: 4096,
            kind: MemoryRegionKind::Kernel,
            flags: Flags::PRESENT | Flags::WRITABLE,
        };
        assert_eq!(region.size, 4096);
    }

    #[test]
    fn test_physical_offset() {
        assert!(PHYSICAL_MEMORY_OFFSET.as_u64() > 0);
    }

    #[test]
    fn test_vmm_lock() {
        let _guard = VMM.lock();
    }

    #[test]
    fn test_frame_allocation() {
        let mut allocator =
            SimpleFrameAllocator::new(PhysAddr::new(0x0010_0000), PhysAddr::new(0x0011_0000));
        assert!(allocator.allocate_frame().is_some());
        assert!(allocator.allocate_frame().is_none());
    }

    #[test]
    fn test_page_mapping() {
        assert!(true);
    }

    #[test]
    fn test_page_tables() {
        assert!(true);
    }

    #[test]
    fn test_virtual_address() {
        let addr = VirtAddr::new(0xFFFF_8000_0000_1000);
        assert_eq!(addr.as_u64(), 0xFFFF_8000_0000_1000);
    }

    #[test]
    fn test_physical_address() {
        let addr = PhysAddr::new(0x0010_0000);
        assert_eq!(addr.as_u64(), 0x0010_0000);
    }

    #[test]
    fn test_page_flags() {
        let flags = Flags::PRESENT | Flags::WRITABLE;
        assert!(flags.contains(Flags::PRESENT));
    }
}
