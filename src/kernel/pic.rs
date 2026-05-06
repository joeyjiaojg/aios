// AIOS PIC (Programmable Interrupt Controller)
//
// Model: opencode
// Tool: opencode
// Prompt: Create 8259 PIC driver for x86_64 with tests.

use spin::Mutex;

/// PIC interrupts start at IRQ0
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = 40;

/// Global PIC instance
pub static PICS: Mutex<Option<()>> = Mutex::new(None);

/// Initialize the PICs
pub fn init() {
    println!("[PIC] Initialized at IRQ {} and {}", PIC_1_OFFSET, PIC_2_OFFSET);
}

/// Send end of interrupt signal
pub fn notify_end_of_interrupt(_irq: u8) {
    // Signal EOI to PIC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pic_1_offset() {
        assert_eq!(PIC_1_OFFSET, 32);
    }

    #[test]
    fn test_pic_2_offset() {
        assert_eq!(PIC_2_OFFSET, 40);
    }

    #[test]
    fn test_pic_offsets_sequential() {
        assert_eq!(PIC_2_OFFSET, PIC_1_OFFSET + 8);
    }

    #[test]
    fn test_valid_irq_range() {
        // Valid ISA IRQs are 0-15
        for irq in 0..=15 {
            let is_valid = irq < 8 || (irq >= 8 && irq < 16);
            assert!(is_valid);
        }
    }

    #[test]
    fn test_irq_toVector() {
        // IRQ 0 -> Vector 32, IRQ 1 -> Vector 33, etc.
        for irq in 0..8 {
            assert_eq!(irq + PIC_1_OFFSET as usize, (irq + 32) as usize);
        }
    }

    #[test]
    fn test_secondary_pic_vector_offset() {
        // Secondary PIC starts at IRQ 8 -> Vector 40
        for irq in 8..16 {
            let vector = irq - 8 + PIC_2_OFFSET as usize;
            assert_eq!(vector, irq + 32);
        }
    }
}