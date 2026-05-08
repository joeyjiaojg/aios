// AIOS Global Descriptor Table (GDT)
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Create x86_64 GDT with user mode segments (ring 3 code/data), TSS with double-fault IST stack, and proper descriptor setup.

use x86_64::instructions::segmentation::{Segment, CS, DS, ES, SS};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

static TSS: TaskStateSegment = TaskStateSegment::new();
static GDT_TABLE: spin::Mutex<GlobalDescriptorTable<8>> =
    spin::Mutex::new(GlobalDescriptorTable::<8>::empty());

pub struct GdtSelectors {
    pub code_selector: SegmentSelector,
    pub data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
}

impl Clone for GdtSelectors {
    fn clone(&self) -> Self {
        Self {
            code_selector: SegmentSelector(self.code_selector.0),
            data_selector: SegmentSelector(self.data_selector.0),
            user_code_selector: SegmentSelector(self.user_code_selector.0),
            user_data_selector: SegmentSelector(self.user_data_selector.0),
            tss_selector: SegmentSelector(self.tss_selector.0),
        }
    }
}

static SELECTORS: spin::Mutex<Option<GdtSelectors>> = spin::Mutex::new(None);

pub fn init() {
    let mut gdt = GDT_TABLE.lock();
    let code_selector = gdt.append(Descriptor::kernel_code_segment());
    let data_selector = gdt.append(Descriptor::kernel_data_segment());
    let user_code_selector = gdt.append(Descriptor::user_code_segment());
    let user_data_selector = gdt.append(Descriptor::user_data_segment());
    let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));

    // Safety: The GDT table lives in a static mutex, ensuring it remains valid
    // for the lifetime of the system. The address won't change.
    unsafe { gdt.load_unsafe() };

    // Safety: The segment selectors just created point to valid GDT entries.
    unsafe {
        CS::set_reg(code_selector);
        DS::set_reg(data_selector);
        ES::set_reg(data_selector);
        SS::set_reg(data_selector);
        load_tss(tss_selector);
    }

    *SELECTORS.lock() = Some(GdtSelectors {
        code_selector,
        data_selector,
        user_code_selector,
        user_data_selector,
        tss_selector,
    });
}

pub fn setup_tss_stack(kernel_stack_top: VirtAddr) {
    unsafe {
        let tss = &TSS as *const TaskStateSegment as *mut TaskStateSegment;
        (*tss).privilege_stack_table[0] = kernel_stack_top;
    }
}

pub fn get_selectors() -> Option<GdtSelectors> {
    SELECTORS.lock().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdt_init() {
        init();
    }

    #[test]
    fn test_kernel_code_selector() {
        let mut gdt = GlobalDescriptorTable::<8>::empty();
        let sel = gdt.append(Descriptor::kernel_code_segment());
        assert_eq!(sel.index(), 1);
    }

    #[test]
    fn test_kernel_data_selector() {
        let mut gdt = GlobalDescriptorTable::<8>::empty();
        gdt.append(Descriptor::kernel_code_segment());
        let sel = gdt.append(Descriptor::kernel_data_segment());
        assert_eq!(sel.index(), 2);
    }

    #[test]
    fn test_user_code_selector() {
        let mut gdt = GlobalDescriptorTable::<8>::empty();
        gdt.append(Descriptor::kernel_code_segment());
        gdt.append(Descriptor::kernel_data_segment());
        let sel = gdt.append(Descriptor::user_code_segment());
        assert_eq!(sel.index(), 3);
    }

    #[test]
    fn test_user_data_selector() {
        let mut gdt = GlobalDescriptorTable::<8>::empty();
        gdt.append(Descriptor::kernel_code_segment());
        gdt.append(Descriptor::kernel_data_segment());
        gdt.append(Descriptor::user_code_segment());
        let sel = gdt.append(Descriptor::user_data_segment());
        assert_eq!(sel.index(), 4);
    }

    #[test]
    fn test_tss_selector() {
        let mut gdt = GlobalDescriptorTable::<8>::empty();
        gdt.append(Descriptor::kernel_code_segment());
        gdt.append(Descriptor::kernel_data_segment());
        gdt.append(Descriptor::user_code_segment());
        gdt.append(Descriptor::user_data_segment());
        let sel = gdt.append(Descriptor::tss_segment(&TSS));
        assert_eq!(sel.index(), 5);
    }

    #[test]
    fn test_selectors_return_some() {
        init();
        let s = get_selectors();
        assert!(s.is_some());
    }

    #[test]
    fn test_tss_setup() {
        init();
        setup_tss_stack(VirtAddr::new(0xFFFF_9000_0000_1000));
    }

    #[test]
    fn test_double_fault_ist_index() {
        assert_eq!(DOUBLE_FAULT_IST_INDEX, 0);
    }

    #[test]
    fn test_descriptor_types() {
        let kcode = Descriptor::kernel_code_segment();
        let udata = Descriptor::user_data_segment();
        let _ = (kcode, udata);
    }
}
