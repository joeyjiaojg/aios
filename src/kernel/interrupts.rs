// AIOS Interrupt Descriptor Table (IDT)
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Implement Interrupt Descriptor Table (IDT) for AIOS x86_64 kernel

use core::sync::atomic::{AtomicBool, Ordering};
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::instructions::port::{Port, PortReadOnly};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// # Safety
// PICS is protected by a Mutex and initialized once during boot.
// Only accessed through init_pic() and end_of_interrupt() functions.
static PICS: Mutex<Option<ChainedPics>> = Mutex::new(None);

// # Safety
// IDT is initialized once during boot. load() must be called after all handlers are registered.
// Used only through init() function which is called once.
static mut IDT_FOR_LOAD: Option<InterruptDescriptorTable> = None;

// # Safety
// IDT is protected by a Mutex and initialized once during boot.
// Only accessed through init_idt() function for registering handlers.
static IDT: Mutex<Option<InterruptDescriptorTable>> = Mutex::new(None);

pub static TIMER_TICK: AtomicBool = AtomicBool::new(false);

fn init_idt_once() {
    // # Safety
    // IDT initialization happens once during kernel boot before interrupts are enabled.
    // This is a single-core kernel, so there are no data races during initialization.
    let mut idt_guard = IDT.lock();
    if idt_guard.is_none() {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        // # Safety
        // IDT_FOR_LOAD is used only in init() which is called once after initialization.
        // This is safe because we immediately store the IDT and it stays valid.
        unsafe {
            IDT_FOR_LOAD = Some(core::ptr::read(&idt));
        }
        *idt_guard = Some(idt);
    }
}

pub fn init() {
    init_idt_once();
    init_pic();
    configure_pit_timer();

    // # Safety
    // Loading IDT is safe - IDT_FOR_LOAD is set in init_idt_once() above.
    // This enables CPU exception handling. Called once during boot.
    unsafe {
        IDT_FOR_LOAD.as_ref().unwrap().load();
    }
}

pub fn init_idt() {
    // # Safety
    // Registering timer interrupt handler is safe - IDT is initialized during boot.
    // This enables hardware timer interrupts for the scheduler.
    let mut guard = IDT.lock();
    if let Some(ref mut idt) = *guard {
        idt[32].set_handler_fn(timer_interrupt_handler);
    }
}

pub fn end_of_interrupt(id: u8) {
    // # Safety
    // PICS is protected by Mutex. notify_end_of_interrupt is designed to be safe.
    // The unsafe block is required by the ChainedPics API.
    let mut pics_guard = PICS.lock();
    if let Some(ref mut pics) = *pics_guard {
        // # Safety
        // notify_end_of_interrupt is safe when called with valid IRQ numbers.
        unsafe {
            if id >= PIC_2_OFFSET {
                pics.notify_end_of_interrupt(id - PIC_2_OFFSET);
            } else {
                pics.notify_end_of_interrupt(id - PIC_1_OFFSET);
            }
        }
    }
}

fn init_pic() {
    // # Safety
    // PICS initialization happens once during kernel boot.
    // Creating ChainedPics just creates the data structure.
    let mut pics_guard = PICS.lock();
    if pics_guard.is_none() {
        *pics_guard = Some(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
    }
}

fn configure_pit_timer() {
    // # Safety
    // Writing to PIT I/O ports (0x43, 0x40) is safe - these are standard PC hardware.
    // The PIT is a well-documented legacy device. We configure channel 0 in mode 3.
    let mut command_port = Port::<u8>::new(0x43);
    let mut data_port = Port::<u8>::new(0x40);

    // Set frequency to ~100 Hz (10ms interval)
    // PIT runs at 1193182 Hz
    // Divisor = 1193182 / 100 = 11931
    let divisor: u16 = 11931;

    // # Safety
    // Writing to PIT control register and data port is standard kernel init.
    unsafe {
        command_port.write(0x36);
        data_port.write((divisor & 0xFF) as u8);
        data_port.write(((divisor >> 8) & 0xFF) as u8);
    }
}

extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("Double fault exception");
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    TIMER_TICK.store(true, Ordering::SeqCst);
    end_of_interrupt(PIC_1_OFFSET);
}

#[allow(dead_code)]
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    end_of_interrupt(PIC_1_OFFSET + 1);
    let mut keyboard_data = PortReadOnly::<u8>::new(0x60);
    let _scan_code: u8 = unsafe { keyboard_data.read() };
}

pub fn enable_interrupts() {
    // # Safety
    // Enabling interrupts is safe - it's required for timer and scheduler operation.
    // This is called after all handlers are registered.
    unsafe {
        core::arch::asm!("sti");
    }
}

pub fn disable_interrupts() {
    // # Safety
    // Disabling interrupts is safe - used during critical sections.
    unsafe {
        core::arch::asm!("cli");
    }
}

pub fn is_timer_tick() -> bool {
    TIMER_TICK.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idt_init() {
        init_idt_once();
    }

    #[test]
    fn test_interrupt_vector() {
        assert_eq!(PIC_1_OFFSET, 32);
        assert_eq!(PIC_1_OFFSET + 1, 33);
        assert_eq!(PIC_1_OFFSET + 8, 40);
    }

    #[test]
    fn test_exception_handler() {
        extern "x86-interrupt" fn test_handler(_: InterruptStackFrame) {}
        let _: extern "x86-interrupt" fn(InterruptStackFrame) = test_handler;
    }

    #[test]
    fn test_irq_handler() {
        extern "x86-interrupt" fn test_handler(_: InterruptStackFrame) {}
        let _: extern "x86-interrupt" fn(InterruptStackFrame) = test_handler;
    }

    #[test]
    fn test_pit_divisor() {
        let divisor: u16 = 11931;
        assert!(divisor > 0);
        assert!(divisor <= 65535);
        let frequency = 1193182u32 / u32::from(divisor);
        assert_eq!(frequency, 100);
    }

    #[test]
    fn test_timer_tick_atomic() {
        TIMER_TICK.store(false, Ordering::SeqCst);
        assert!(!TIMER_TICK.load(Ordering::SeqCst));
        TIMER_TICK.store(true, Ordering::SeqCst);
        assert!(TIMER_TICK.load(Ordering::SeqCst));
        let tick = TIMER_TICK.swap(false, Ordering::SeqCst);
        assert!(tick);
        assert!(!TIMER_TICK.load(Ordering::SeqCst));
    }

    #[test]
    fn test_pic_initialization() {
        init_pic();
    }

    #[test]
    fn test_end_of_interrupt() {
        init_pic();
        end_of_interrupt(PIC_1_OFFSET);
    }

    #[test]
    fn test_timer_interrupt_id() {
        assert_eq!(PIC_1_OFFSET, 32);
    }

    #[test]
    fn test_keyboard_interrupt_id() {
        assert_eq!(PIC_1_OFFSET + 1, 33);
    }

    #[test]
    fn test_pic_offsets_sequential() {
        assert_eq!(PIC_2_OFFSET, PIC_1_OFFSET + 8);
    }
}
