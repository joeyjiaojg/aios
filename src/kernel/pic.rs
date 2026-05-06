// AIOS PIC (Programmable Interrupt Controller)
//
// Model: opencode
// Tool: opencode
// Prompt: Create 8259 PIC driver stub.

use spin::Mutex;

/// PIC interrupts start at IRQ0
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = 40;

/// Global PIC instance
pub static PICS: Mutex<Option<()>> = Mutex::new(None);

/// Initialize the PICs
pub fn init() {
    PICS.lock();
    println!("[PIC] Initialized at IRQ {} and {}", PIC_1_OFFSET, PIC_2_OFFSET);
}

/// Send end of interrupt signal
pub fn notify_end_of_interrupt(_irq: u8) {}