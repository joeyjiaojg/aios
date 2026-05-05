// AIOS Interrupt Descriptor Table (IDT)
//
// Model: opencode
// Tool: opencode
// Prompt: Create x86_64 IDT with exception handlers, IRQ remapping,
//         and hardware interrupt handlers.

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use crate::println;

/// Interrupt manager
pub struct InterruptManager {
    idt: InterruptDescriptorTable,
}

impl InterruptManager {
    /// Create and initialize IDT
    pub fn new() -> Self {
        let mut idt = InterruptDescriptorTable::new();

        // Breakpoint handler
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        // Double fault handler with IST
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(x86_64::structures::gdt::DescriptorIndex::TRIPLE_FAULT);
        }

        // General protection fault handler
        idt.general_protection_fault.set_handler_fn(gpf_handler);

        // Page fault handler
        idt.page_fault.set_handler_fn(page_fault_handler);

        // Timer interrupt (IRQ0)
        idt[32].set_handler_fn(timer_interrupt_handler);

        // Keyboard interrupt (IRQ1)
        idt[33].set_handler_fn(keyboard_interrupt_handler);

        Self { idt }
    }

    /// Load the IDT into the CPU
    pub fn load(&'static self) {
        self.idt.load();
    }
}

/// Breakpoint interrupt handler
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

/// Double fault handler
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

/// General protection fault handler
extern "x86-interrupt" fn gpf_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!(
        "EXCEPTION: GENERAL PROTECTION FAULT\nError Code: {}\n{:#?}",
        error_code,
        stack_frame
    );
}

/// Page fault handler
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;
    println!(
        "EXCEPTION: PAGE FAULT\nError Code: {:?}\nFaulting Address: {:#x}\n{:#?}",
        error_code,
        Cr2::read().as_u64(),
        stack_frame
    );
}

/// Timer interrupt handler (IRQ0)
extern "x86-interrupt" fn timer_interrupt_handler(stack_frame: InterruptStackFrame) {
    // Send EOI to PIC
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(32);
    }
    // TODO: Update scheduler tick
}

/// Keyboard interrupt handler (IRQ1)
extern "x86-interrupt" fn keyboard_interrupt_handler(stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    // TODO: Convert scancode to key press event
    // For now, just acknowledge the interrupt

    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(33);
    }
}

/// Initialize interrupts
pub fn init() -> InterruptManager {
    let idt = InterruptManager::new();
    idt.load();
    idt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idt_creation() {
        let idt = InterruptManager::new();
        // IDT should be loadable
        assert!(!idt.idt.as_slice().is_empty());
    }
}
