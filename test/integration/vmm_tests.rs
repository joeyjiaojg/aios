// AIOS Integration Tests - Virtual Memory Manager
//
// Model: MiniMax M2.5 Free
// Tool: opencode
// Prompt: Create integration tests for VMM including page table setup,
//         memory mapping, and translation tests.

#[cfg(test)]
mod vmm_integration_tests {
    use x86_64::{VirtAddr, structures::paging::page_table::PageTableFlags};

    #[test]
    fn test_page_table_flags_combination() {
        // Test kernel read/write flags
        let kernel_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        assert!(kernel_flags.contains(PageTableFlags::PRESENT));
        assert!(kernel_flags.contains(PageTableFlags::WRITABLE));

        // Test user space flags
        let user_flags = PageTableFlags::PRESENT
            | PageTableFlags::WRITABLE
            | PageTableFlags::USER_ACCESSIBLE;
        assert!(user_flags.contains(PageTableFlags::USER_ACCESSIBLE));

        // Test execute disable flag
        let no_exec = PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE;
        assert!(no_exec.contains(PageTableFlags::NO_EXECUTE));
    }

    #[test]
    fn test_virtual_address_creation() {
        let addr = VirtAddr::new(0xffff_8000_0000_0000);
        assert_eq!(addr.as_u64(), 0xffff_8000_0000_0000);
        assert!(!addr.is_null());
    }

    #[test]
    fn test_page_calculation() {
        use x86_64::structures::paging::{Page, Size4KiB};

        let addr = VirtAddr::new(0x1000);
        let page = Page::<Size4KiB>::containing_address(addr);
        assert_eq!(page.start_address().as_u64(), 0x1000);

        let addr = VirtAddr::new(0x1FFF);
        let page = Page::<Size4KiB>::containing_address(addr);
        assert_eq!(page.start_address().as_u64(), 0x1000);

        let addr = VirtAddr::new(0x2000);
        let page = Page::<Size4KiB>::containing_address(addr);
        assert_eq!(page.start_address().as_u64(), 0x2000);
    }

    #[test]
    fn test_phys_frame_calculation() {
        use x86_64::{PhysAddr, structures::paging::PhysFrame};

        let addr = PhysAddr::new(0x1000);
        let frame = PhysFrame::containing_address(addr);
        assert_eq!(frame.start_address().as_u64(), 0x1000);
    }
}
