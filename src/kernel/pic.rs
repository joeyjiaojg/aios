// AIOS PIC (Programmable Interrupt Controller)
//
// Model: opencode
// Tool: opencode
// Prompt: Create 8259 PIC driver for x86_64 with interrupt masking
//         and end-of-interrupt handling.

use pic8259::ChainedPics;
use spin::Mutex;

/// PIC interrupts start at IRQ0
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

/// Global PIC instance
pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe {
    ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)
});

/// Initialize the PICs
pub fn init() {
    unsafe {
        PICS.lock().initialize();
    }
}

/// Mask a specific IRQ line
pub fn mask_irq(irq: u8) {
    let mut pics = PICS.lock();
    if irq < 8 {
        unsafe { pics.1.mask(irq) };
    } else {
        unsafe { pics.2.mask(irq - 8) };
    }
}

/// Unmask a specific IRQ line
pub fn unmask_irq(irq: u8) {
    let mut pics = PICS.lock();
    if irq < 8 {
        unsafe { pics.1.unmask(irq) };
    } else {
        unsafe { pics.2.unmask(irq - 8) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pic_constants() {
        assert_eq!(PIC_1_OFFSET, 32);
        assert_eq!(PIC_2_OFFSET, 40);
    }
}
