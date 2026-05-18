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

static mut TSS: TaskStateSegment = TaskStateSegment::new();
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
    // user_data MUST come before user_code so that sysretq works:
    // sysretq sets SS = STAR[63:48]+8 | 3, CS = STAR[63:48]+16 | 3
    // With STAR[63:48]=0x10 (kernel_data): SS=0x18|3=user_data, CS=0x20|3=user_code
    let user_data_selector = gdt.append(Descriptor::user_data_segment());
    let user_code_selector = gdt.append(Descriptor::user_code_segment());
    // # Safety
    // TSS is a static mut accessed only here during single-threaded init.
    // The Descriptor::tss_segment() call requires a reference to TSS, and
    // it's safe because no other code can access TSS concurrently at this point.
    let tss_selector = unsafe { gdt.append(Descriptor::tss_segment(&TSS)) };

    // # Safety
    // The GDT table is stored in a static spin::Mutex, meaning its
    // address in memory is fixed for the system lifetime. load_unsafe() performs
    // the lgdt instruction which reads the table address once. Since the backing
    // storage never moves, the loaded GDT pointer remains valid indefinitely.
    unsafe { gdt.load_unsafe() };

    // # Safety
    // All four segment selectors (code_selector at index 1, data_selector
    // at index 2, user_code at index 3, user_data at index 4) were just appended
    // to the GDT above and the GDT was loaded into the CPU. The tss_selector at
    // index 5 points to the static TSS struct which lives for the program duration.
    // Setting these segment registers with valid GDT entries is the standard way
    // to activate kernel segments on x86_64.
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
    // # Safety: same as above.
    unsafe {
        TSS.privilege_stack_table[0] = kernel_stack_top;
        // Also set IST[0] (index 0 = IST entry 1) so the page fault handler
        // can use an IST-based switch, which is more robust than RSP0.
        TSS.interrupt_stack_table[0] = kernel_stack_top;

        if crate::debug::is_debug_enabled() {
            crate::serial::write_str("[gdt] TSS privilege_stack_table[0] = 0x");
            let rsp0 = TSS.privilege_stack_table[0].as_u64();
            for i in (0..16).rev() {
                let nibble = ((rsp0 >> (i * 4)) & 0xF) as u8;
                crate::serial::write_byte(if nibble < 10 {
                    b'0' + nibble
                } else {
                    b'a' + (nibble - 10)
                });
            }
            crate::serial::write_str("\r\n");
        }
    }
}

pub fn get_selectors() -> Option<GdtSelectors> {
    SELECTORS.lock().clone()
}

/// Read the current TSS RSP0 (ring-0 stack pointer for ring-3→ring-0 transitions).
pub fn get_tss_rsp0() -> u64 {
    // # Safety: TSS is a static with a lifetime that spans the kernel's lifetime.
    unsafe { TSS.privilege_stack_table[0].as_u64() }
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
        gdt.append(Descriptor::user_data_segment());
        let sel = gdt.append(Descriptor::user_code_segment());
        assert_eq!(sel.index(), 4);
    }

    #[test]
    fn test_user_data_selector() {
        let mut gdt = GlobalDescriptorTable::<8>::empty();
        gdt.append(Descriptor::kernel_code_segment());
        gdt.append(Descriptor::kernel_data_segment());
        let sel = gdt.append(Descriptor::user_data_segment());
        assert_eq!(sel.index(), 3);
    }

    #[test]
    fn test_tss_selector() {
        let mut gdt = GlobalDescriptorTable::<8>::empty();
        gdt.append(Descriptor::kernel_code_segment());
        gdt.append(Descriptor::kernel_data_segment());
        gdt.append(Descriptor::user_data_segment());
        gdt.append(Descriptor::user_code_segment());
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
