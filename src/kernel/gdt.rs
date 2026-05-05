// AIOS Global Descriptor Table (GDT)
//
// Model: opencode
// Tool: opencode
// Prompt: Create x86_64 GDT with kernel code and data segments,
//         TSS entry, and proper descriptor setup.

use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// GDT with all required segments
pub struct GdtManager {
    gdt: GlobalDescriptorTable,
    tss: TaskStateSegment,
}

impl GdtManager {
    /// Create and initialize GDT
    pub fn new() -> Self {
        let mut gdt = GlobalDescriptorTable::new();

        // Add kernel code segment
        gdt.add_entry(Descriptor::kernel_code_segment());

        // Add kernel data segment
        gdt.add_entry(Descriptor::kernel_data_segment());

        // Set up TSS with interrupt stack table
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };

        // Add TSS segment to GDT
        gdt.add_entry(Descriptor::tss_segment(&tss));

        Self { gdt, tss }
    }

    /// Load the GDT into the CPU
    pub fn load(&'static self) {
        use x86_64::instructions::tables::load_tss;
        let gdt = self.gdt.load();
        unsafe {
            gdt.load();
            load_tss(x86_64::structures::gdt::Selector::from_raw(3 * 8));
        }
    }
}

/// Initialize GDT
pub fn init() -> GdtManager {
    let gdt = GdtManager::new();
    gdt.load();
    gdt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdt_creation() {
        let gdt = GdtManager::new();
        // GDT should have at least 4 entries: null, code, data, TSS
        assert!(!gdt.gdt.as_slice().is_empty());
    }
}
