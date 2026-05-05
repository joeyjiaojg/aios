// AIOS Interrupt Descriptor Table (IDT)
//
// Model: opencode
// Tool: opencode
// Prompt: Create x86_64 IDT with exception handlers, IRQ remapping,
//         hardware interrupt handlers, PIT timer with tick counter,
//         keyboard buffer, LAPIC/IOAPIC stubs, and proper SAFETY comments.

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::structures::gdt::DescriptorIndex;
use x86_64::registers::control::Cr2;
use crate::println;
use crate::panic;
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};

/// PIT timer tick counter - incremented every 10ms (100Hz)
static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

/// Keyboard ring buffer for scancode storage
static KEYBOARD_BUFFER: Mutex<KeyboardBuffer> = Mutex::new(KeyboardBuffer::new());

const KEYBOARD_BUFFER_SIZE: usize = 256;

struct KeyboardBuffer {
    buffer: [u8; KEYBOARD_BUFFER_SIZE],
    head: usize,
    tail: usize,
    count: usize,
}

impl KeyboardBuffer {
    fn new() -> Self {
        Self {
            buffer: [0; KEYBOARD_BUFFER_SIZE],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn push(&mut self, scancode: u8) -> bool {
        if self.count >= KEYBOARD_BUFFER_SIZE {
            return false;
        }
        self.buffer[self.tail] = scancode;
        self.tail = (self.tail + 1) % KEYBOARD_BUFFER_SIZE;
        self.count += 1;
        true
    }

    fn pop(&mut self) -> Option<u8> {
        if self.count == 0 {
            return None;
        }
        let scancode = self.buffer[self.head];
        self.head = (self.head + 1) % KEYBOARD_BUFFER_SIZE;
        self.count -= 1;
        Some(scancode)
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Interrupt manager
pub struct InterruptManager {
    idt: InterruptDescriptorTable,
}

impl InterruptManager {
    /// Create and initialize IDT with all handlers
    pub fn new() -> Self {
        let mut idt = InterruptDescriptorTable::new();

        // Exception handlers (0-31)
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.non_maskable_interrupt.set_handler_fn(nmi_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.device_not_available.set_handler_fn(device_not_available_handler);

        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(DescriptorIndex::TRIPLE_FAULT);
        }

        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        idt.general_protection_fault.set_handler_fn(gpf_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);

        idt.x87_floating_point.set_handler_fn(x87_floating_point_handler);
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        idt.machine_check.set_handler_fn(machine_check_handler);
        idt.simd_floating_point.set_handler_fn(simd_floating_point_handler);
        idt.virtualization.set_handler_fn(virtualization_handler);

        // Hardware IRQ handlers (32-47)
        idt[32].set_handler_fn(timer_interrupt_handler);
        idt[33].set_handler_fn(keyboard_interrupt_handler);
        idt[34].set_handler_fn(cascade_handler);
        idt[35].set_handler_fn(com2_handler);
        idt[36].set_handler_fn(com1_handler);
        idt[37].set_handler_fn(lpt2_handler);
        idt[38].set_handler_fn(floppy_handler);
        idt[39].set_handler_fn(lpt1_handler);
        idt[40].set_handler_fn(rtc_handler);
        idt[41].set_handler_fn(irq8_handler);
        idt[42].set_handler_fn(irq9_handler);
        idt[43].set_handler_fn(irq10_handler);
        idt[44].set_handler_fn(irq11_handler);
        idt[45].set_handler_fn(irq12_handler);
        idt[46].set_handler_fn(irq13_handler);
        idt[47].set_handler_fn(irq14_handler);

        Self { idt }
    }

    /// Load the IDT into the CPU
    pub fn load(&'static self) {
        self.idt.load();
    }
}

// ============================================================================
// Exception Handlers
// ============================================================================

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    panic!(
        "EXCEPTION: DIVIDE ERROR\nRIP: {:#x}\nCS: {:#x}\nRFLAGS: {:#x}",
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.code_segment,
        stack_frame.cpu_flags
    );
}

extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: DEBUG\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn nmi_handler(_stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: NMI (Non-Maskable Interrupt)");
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: OVERFLOW\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: BOUND RANGE EXCEEDED\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    panic!(
        "EXCEPTION: INVALID OPCODE\nRIP: {:#x}",
        stack_frame.instruction_pointer.as_u64()
    );
}

extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: DEVICE NOT AVAILABLE\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!(
        "EXCEPTION: DOUBLE FAULT\nRIP: {:#x}\nError Code: {}",
        stack_frame.instruction_pointer.as_u64(),
        _error_code
    );
}

extern "x86-interrupt" fn invalid_tss_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: INVALID TSS\nError Code: {:#x}\n{:#?}",
        error_code,
        stack_frame
    );
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: SEGMENT NOT PRESENT\nError Code: {:#x}\nRIP: {:#x}",
        error_code,
        stack_frame.instruction_pointer.as_u64()
    );
}

extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: STACK SEGMENT FAULT\nError Code: {:#x}\n{:#?}",
        error_code,
        stack_frame
    );
}

extern "x86-interrupt" fn gpf_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT\nError Code: {:#x}\nRIP: {:#x}",
        error_code,
        stack_frame.instruction_pointer.as_u64()
    );
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let fault_addr = Cr2::read().as_u64();
    panic!(
        "EXCEPTION: PAGE FAULT\nError Code: {:?}\nFaulting Address: {:#x}\nRIP: {:#x}\nPresent: {}\nWrite: {}\nUser: {}",
        error_code,
        fault_addr,
        stack_frame.instruction_pointer.as_u64(),
        error_code.present(),
        error_code.write_access(),
        error_code.user_mode_access()
    );
}

extern "x86-interrupt" fn x87_floating_point_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: x87 FLOATING POINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: ALIGNMENT CHECK\nError Code: {:#x}\n{:#?}",
        error_code,
        stack_frame
    );
}

extern "x86-interrupt" fn machine_check_handler(_stack_frame: InterruptStackFrame) -> ! {
    panic!("EXCEPTION: MACHINE CHECK");
}

extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: SIMD FLOATING POINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn virtualization_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: VIRTUALIZATION\n{:#?}", stack_frame);
}

// ============================================================================
// Hardware IRQ Handlers
// ============================================================================

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // SAFETY: Updating atomic tick counter is safe - only timer interrupt writes
    // and it's the highest priority hardware interrupt.
    TICK_COUNT.fetch_add(1, Ordering::Relaxed);

    // SAFETY: Accessing PIC to send EOI requires correct IRQ mapping.
    // IRQ0 maps to interrupt 32 in the remapped IDT.
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(32);
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    // SAFETY: Reading from port 0x60 (keyboard data port) is safe -
    // this is the standard PS/2 keyboard data I/O port.
    let port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    // Store scancode in keyboard buffer
    // SAFETY: Mutex acquisition is safe - we're in interrupt context but
    // spin::Mutex is designed to handle this safely.
    if let Ok(mut buffer) = KEYBOARD_BUFFER.try_lock() {
        let _ = buffer.push(scancode);
    }

    // SAFETY: Sending EOI to PIC for IRQ1 (interrupt 33) is required
    // to allow subsequent keyboard interrupts.
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(33);
    }
}

extern "x86-interrupt" fn cascade_handler(_stack_frame: InterruptStackFrame) {
    // SAFETY: EOI for cascaded PIC (IRQ2)
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(34);
    }
}

extern "x86-interrupt" fn com2_handler(_stack_frame: InterruptStackFrame) {
    println!("IRQ: COM2");
    // SAFETY: EOI for IRQ3
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(35);
    }
}

extern "x86-interrupt" fn com1_handler(_stack_frame: InterruptStackFrame) {
    println!("IRQ: COM1");
    // SAFETY: EOI for IRQ4
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(36);
    }
}

extern "x86-interrupt" fn lpt2_handler(_stack_frame: InterruptStackFrame) {
    // SAFETY: EOI for IRQ5
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(37);
    }
}

extern "x86-interrupt" fn floppy_handler(_stack_frame: InterruptStackFrame) {
    // SAFETY: EOI for IRQ6
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(38);
    }
}

extern "x86-interrupt" fn lpt1_handler(_stack_frame: InterruptStackFrame) {
    // SAFETY: EOI for IRQ7
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(39);
    }
}

extern "x86-interrupt" fn rtc_handler(_stack_frame: InterruptStackFrame) {
    // SAFETY: EOI for IRQ8
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(40);
    }
}

extern "x86-interrupt" fn irq8_handler(_stack_frame: InterruptStackFrame) {
    // SAFETY: EOI for IRQ9
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(41);
    }
}

extern "x86-interrupt" fn irq9_handler(_stack_frame: InterruptStackFrame) {
    // SAFETY: EOI for IRQ10
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(42);
    }
}

extern "x86-interrupt" fn irq10_handler(_stack_frame: InterruptStackFrame) {
    // SAFETY: EOI for IRQ11
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(43);
    }
}

extern "x86-interrupt" fn irq11_handler(_stack_frame: InterruptStackFrame) {
    // SAFETY: EOI for IRQ12
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(44);
    }
}

extern "x86-interrupt" fn irq12_handler(_stack_frame: InterruptStackFrame) {
    // SAFETY: EOI for IRQ13
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(45);
    }
}

extern "x86-interrupt" fn irq14_handler(_stack_frame: InterruptStackFrame) {
    // SAFETY: EOI for IRQ14
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(46);
    }
}

extern "x86-interrupt" fn irq13_handler(_stack_frame: InterruptStackFrame) {
    // SAFETY: EOI for IRQ15
    unsafe {
        crate::pic::PICS.lock().notify_end_of_interrupt(47);
    }
}

// ============================================================================
// LAPIC/IOAPIC Stubs for SMP Support
// ============================================================================

/// Initialize Local APIC (LAPIC) for SMP
/// SAFETY: This function should only be called from the BSP (Bootstrap Processor)
/// during early boot before other CPUs are started.
pub unsafe fn init_lapic() {
    // TODO: Implement LAPIC initialization
    // - Enable LAPIC via MSR IA32_APIC_BASE
    // - Set Task Priority Register (TPR)
    // - Configure Spurious Interrupt Vector Register
    // - Enable APIC
}

/// Get current LAPIC ID
pub fn get_lapic_id() -> u32 {
    // TODO: Read LAPIC ID from current processor
    0
}

/// Send IPI (Inter-Processor Interrupt) to target CPU
/// SAFETY: Caller must ensure the target CPU is online and can receive IPIs.
pub unsafe fn send_ipi(cpu_id: u32, vector: u8) {
    // TODO: Implement IPI via LAPIC ICR (Interrupt Command Register)
    let _ = (cpu_id, vector);
}

/// Initialize IOAPIC for SMP
/// SAFETY: This function should only be called during early boot.
// TODO: Implement IOAPIC initialization
pub unsafe fn init_ioapic() {
    // TODO: Implement IOAPIC initialization
    // - Detect IOAPIC
    // - Configure IRQ to vector mappings
    // - Enable all IRQs
}

/// Route IRQ to specific CPU
/// SAFETY: Caller must ensure IRQ is valid and CPU is online.
pub unsafe fn route_irq_to_cpu(irq: u8, cpu_id: u32) {
    // TODO: Implement IRQ routing to specific CPU via IOAPIC
    let _ = (irq, cpu_id);
}

/// Enable specific IRQ line
pub unsafe fn enable_irq(irq: u8) {
    // TODO: Enable IRQ in IOAPIC
    let _ = irq;
}

/// Disable specific IRQ line
pub unsafe fn disable_irq(irq: u8) {
    // TODO: Disable IRQ in IOAPIC
    let _ = irq;
}

// ============================================================================
// Public API
// ============================================================================

/// Get current tick count
pub fn get_tick_count() -> u64 {
    TICK_COUNT.load(Ordering::Relaxed)
}

/// Read a key from the keyboard buffer (non-blocking)
pub fn read_key() -> Option<u8> {
    // SAFETY: Locking keyboard buffer is safe - this is called from task context.
    KEYBOARD_BUFFER.lock().pop()
}

/// Check if keyboard buffer has data
pub fn keyboard_has_data() -> bool {
    !KEYBOARD_BUFFER.lock().is_empty()
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
        assert!(!idt.idt.as_slice().is_empty());
    }

    #[test]
    fn test_keyboard_buffer() {
        let mut buffer = KeyboardBuffer::new();
        assert!(buffer.is_empty());

        assert!(buffer.push(0x1E)); // 'A' scancode
        assert!(!buffer.is_empty());

        assert_eq!(buffer.pop(), Some(0x1E));
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_tick_count() {
        let initial = get_tick_count();
        assert_eq!(initial, 0);
    }
}