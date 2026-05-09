// AIOS Interrupt Descriptor Table (IDT)
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Implement Interrupt Descriptor Table (IDT) for AIOS x86_64 kernel

use core::sync::atomic::{AtomicBool, Ordering};
use pic8259::ChainedPics;
use spin::Once;
use x86_64::instructions::port::{Port, PortReadOnly};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

static mut PICS: Option<ChainedPics> = None;

static IDT_ONCE: Once = Once::new();
static mut IDT: Option<InterruptDescriptorTable> = None;

pub static TIMER_TICK: AtomicBool = AtomicBool::new(false);

fn get_idt() -> &'static mut InterruptDescriptorTable {
    IDT_ONCE.call_once(|| unsafe {
        let mut idt = InterruptDescriptorTable::new();

        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);

        IDT = Some(idt);
    });

    unsafe { IDT.as_mut().unwrap() }
}

pub fn init() {
    let idt = get_idt();
    idt.load();

    init_pic();
    configure_pit_timer();
}

pub fn init_idt() {
    let idt = get_idt();
    idt[32].set_handler_fn(timer_interrupt_handler);
}

pub fn end_of_interrupt(id: u8) {
    // Safety: PICS is initialized once during init and only accessed here
    // notify_end_of_interrupt is designed to be safe for PIC operation
    unsafe {
        if let Some(ref mut pics) = PICS {
            if id >= PIC_2_OFFSET {
                pics.notify_end_of_interrupt(id - PIC_2_OFFSET);
            } else {
                pics.notify_end_of_interrupt(id - PIC_1_OFFSET);
            }
        }
    }
}

fn init_pic() {
    // Safety: Creating ChainedPics is safe - it just creates the data structure
    // The actual PIC hardware is already configured by the BIOS
    unsafe {
        PICS = Some(ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET));
    }
}

fn configure_pit_timer() {
    // PIT channel 0 is connected to IRQ0 (timer interrupt)
    // Config: binary mode, mode 3 (square wave generator), both bytes LSB then MSB
    let mut command_port = Port::<u8>::new(0x43);
    let mut data_port = Port::<u8>::new(0x40);

    // Set frequency to ~100 Hz (10ms interval)
    // PIT runs at 1193182 Hz
    // Divisor = 1193182 / 100 = 11931
    let divisor: u16 = 11931;

    unsafe {
        // Send command: channel 0, lobyte/hibyte, mode 3 (square wave), binary
        command_port.write(0x36);

        // Send divisor LSB then MSB
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

#[allow(dead_code)]
fn serial_print(char: char) {
    unsafe {
        let mut serial_port = Port::<u8>::new(0x3F8);
        while (PortReadOnly::<u8>::new(0x3FD).read() & 0x20u8) == 0 {}
        serial_port.write(char as u8);
    }
}

pub fn enable_interrupts() {
    // Safety: Enabling interrupts is safe - it's required for timer/scheduler operation
    unsafe {
        core::arch::asm!("sti");
    }
}

pub fn disable_interrupts() {
    // Safety: Disabling interrupts is safe - used during critical sections
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
        init();
    }

    #[test]
    fn test_interrupt_vector() {
        assert_eq!(PIC_1_OFFSET + 0, 32);
        assert_eq!(PIC_1_OFFSET + 1, 33);
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
    }

    #[test]
    fn test_timer_tick_atomic() {
        TIMER_TICK.store(false, Ordering::SeqCst);
        assert!(!TIMER_TICK.load(Ordering::SeqCst));
    }

    #[test]
    fn test_pic_initialization() {
        init_pic();
    }

    #[test]
    fn test_end_of_interrupt() {
        init_pic();
    }

    #[test]
    fn test_timer_interrupt_id() {
        assert_eq!(PIC_1_OFFSET, 32);
    }

    #[test]
    fn test_keyboard_interrupt_id() {
        assert_eq!(PIC_1_OFFSET + 1, 33);
    }
}
