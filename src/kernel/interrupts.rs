// AIOS Interrupt Descriptor Table (IDT)
//
// Model: claude-sonnet-4-6
// Tool: claude-code
// Prompt: Fix IDT init so timer handler is loaded into the live IDT; fix PIC initialization.

use core::sync::atomic::{AtomicBool, Ordering};
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::instructions::port::PortReadOnly;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

static PICS: Mutex<Option<ChainedPics>> = Mutex::new(None);

// The IDT must live in a static with a stable address for the lifetime of the kernel.
// It is written once during init() (before interrupts are enabled) and never modified
// again, so no runtime lock is needed for reads after that point.
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub static TIMER_TICK: AtomicBool = AtomicBool::new(false);

pub fn init() {
    // # Safety
    // `IDT` is a `static mut` which normally risks aliased mutable references.
    // This is safe here because:
    //   1. `init()` runs exactly once during single-threaded boot, before
    //      interrupts are enabled (no concurrent access is possible).
    //   2. After `IDT.load()` the IDT is only read by the CPU's interrupt
    //      dispatch mechanism, never written again — so no aliasing occurs
    //      at runtime.
    unsafe {
        IDT.breakpoint.set_handler_fn(breakpoint_handler);
        IDT.double_fault.set_handler_fn(double_fault_handler);
        // IRQ 0 (timer) → vector PIC_1_OFFSET (32), IRQ 1 (keyboard) → 33
        IDT[PIC_1_OFFSET].set_handler_fn(timer_interrupt_handler);
        IDT[PIC_1_OFFSET + 1].set_handler_fn(keyboard_interrupt_handler);
        IDT.load();
    }

    init_pic();
    configure_pit_timer();
}

// Kept for compatibility with kernel_entry call sequence; IDT is fully set up in init().
pub fn init_idt() {}

pub fn end_of_interrupt(irq: u8) {
    let mut pics_guard = PICS.lock();
    if let Some(ref mut pics) = *pics_guard {
        // # Safety
        // notify_end_of_interrupt is safe when called with a valid IRQ vector that
        // was delivered by the PIC hardware.
        unsafe {
            pics.notify_end_of_interrupt(irq);
        }
    }
}

fn init_pic() {
    let mut pics_guard = PICS.lock();
    if pics_guard.is_none() {
        let mut pics = unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) };
        // # Safety
        // initialize() programs the 8259 PIC hardware. Called once at boot before
        // interrupts are enabled. No other code touches the PIC ports at this point.
        unsafe { pics.initialize() };
        *pics_guard = Some(pics);
    }
}

fn configure_pit_timer() {
    // PIT runs at ~1193182 Hz; divisor 11931 → ~100 Hz (10 ms ticks).
    let divisor: u16 = 11931;
    // # Safety
    // Writing to PIT ports 0x43/0x40 is standard x86 practice during boot.
    unsafe {
        use x86_64::instructions::port::Port;
        let mut cmd: Port<u8> = Port::new(0x43);
        let mut data: Port<u8> = Port::new(0x40);
        cmd.write(0x36); // channel 0, lobyte/hibyte, mode 3
        data.write((divisor & 0xFF) as u8);
        data.write(((divisor >> 8) & 0xFF) as u8);
    }
}

extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    loop {
        // # Safety
        // HLT in a double-fault handler is safe; we cannot recover so we halt.
        unsafe { core::arch::asm!("hlt") }
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    TIMER_TICK.store(true, Ordering::SeqCst);
    end_of_interrupt(PIC_1_OFFSET);
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut data = PortReadOnly::<u8>::new(0x60);
    let _scan_code: u8 = unsafe { data.read() };
    end_of_interrupt(PIC_1_OFFSET + 1);
}

pub fn enable_interrupts() {
    // # Safety
    // Enabling interrupts is safe after all handlers are registered via init().
    unsafe { core::arch::asm!("sti") }
}

pub fn disable_interrupts() {
    // # Safety
    // Disabling interrupts is safe; used during critical sections.
    unsafe { core::arch::asm!("cli") }
}

pub fn is_timer_tick() -> bool {
    TIMER_TICK.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_vector_offsets() {
        assert_eq!(PIC_1_OFFSET, 32);
        assert_eq!(PIC_1_OFFSET + 1, 33);
        assert_eq!(PIC_2_OFFSET, 40);
    }

    #[test]
    fn test_exception_handler_signature() {
        extern "x86-interrupt" fn test_handler(_: InterruptStackFrame) {}
        let _: extern "x86-interrupt" fn(InterruptStackFrame) = test_handler;
    }

    #[test]
    fn test_pit_divisor_calculation() {
        let divisor: u16 = 11931;
        assert!(divisor > 0);
        let frequency = 1193182u32 / u32::from(divisor);
        assert_eq!(frequency, 100);
    }

    #[test]
    fn test_timer_tick_atomic_flag() {
        TIMER_TICK.store(false, Ordering::SeqCst);
        assert!(!TIMER_TICK.load(Ordering::SeqCst));
        TIMER_TICK.store(true, Ordering::SeqCst);
        assert!(TIMER_TICK.load(Ordering::SeqCst));
        let tick = TIMER_TICK.swap(false, Ordering::SeqCst);
        assert!(tick);
        assert!(!TIMER_TICK.load(Ordering::SeqCst));
    }

    #[test]
    fn test_pic_offsets_sequential() {
        assert_eq!(PIC_2_OFFSET, PIC_1_OFFSET + 8);
    }

    #[test]
    fn test_timer_interrupt_vector() {
        assert_eq!(PIC_1_OFFSET as usize, 32);
    }

    #[test]
    fn test_keyboard_interrupt_vector() {
        assert_eq!(PIC_1_OFFSET as usize + 1, 33);
    }
}
