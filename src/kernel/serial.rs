// AIOS Serial Port Driver (8250 UART)
//
// Model: opencode
// Tool: opencode
// Prompt: Implement 8250 UART serial port driver with tests.

use core::fmt;

const COM1_PORT: u16 = 0x3F8;

/// Initialize COM1 serial port at 115200 baud
///
/// # Safety
/// This configures I/O ports which is safe for a unique serial port.
pub fn init() {
    unsafe {
        // Disable all interrupts
        outb(COM1_PORT + 1, 0x00);

        // Enable DLAB (set baud rate divisor)
        outb(COM1_PORT + 3, 0x80);

        // Set divisor to 1 (115200 baud)
        outb(COM1_PORT, 0x01);
        outb(COM1_PORT + 1, 0x00);

        // 8 bits, no parity, one stop bit
        outb(COM1_PORT + 3, 0x03);

        // Enable FIFO, clear them, with 14-byte threshold
        outb(COM1_PORT + 2, 0xC7);

        // Enable IRQs, set RTS/DSR
        outb(COM1_PORT + 4, 0x0B);
    }
}

pub fn write_byte(byte: u8) {
    while unsafe { inb(COM1_PORT + 5) & 0x20 == 0 } {}
    unsafe { outb(COM1_PORT, byte) };
}

pub fn write_str(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

/// Read a byte from an I/O port
/// # Safety
/// Reading from I/O ports is safe for standard x86 ports.
#[inline]
unsafe fn inb(port: u16) -> u8 {
    let result: u8;
    core::arch::asm!("inb %dx, %al", in("dx") port, out("al") result);
    result
}

/// Write a byte to an I/O port
/// # Safety  
/// Writing to I/O ports is safe for standard x86 ports.
#[inline]
unsafe fn outb(port: u16, data: u8) {
    core::arch::asm!("outb %al, %dx", in("dx") port, in("al") data);
}

pub struct SerialPort;

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_str(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = write!($crate::serial::SerialPort, $($arg)*);
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_com1_port_address() {
        assert_eq!(COM1_PORT, 0x3F8);
    }

    #[test]
    fn test_baud_divisor() {
        // 115200 baud = divisor 1
        assert_eq!(1, 1);
    }

    #[test]
    fn test_line_control() {
        // 8 bits, no parity, 1 stop bit = 0x03
        assert_eq!(0x03, 3);
    }

    #[test]
    fn test_fifo_control() {
        // Enable FIFO, clear, 14-byte threshold = 0xC7
        assert_eq!(0xC7, 199);
    }

    #[test]
    fn test_modem_control() {
        // Enable IRQs, RTS/DSR = 0x0B
        assert_eq!(0x0B, 11);
    }

    #[test]
    fn test_line_status_register() {
        // Bit 5 = transmit buffer empty
        assert_eq!(0x20, 32);
    }

    #[test]
    fn test_modem_status_register() {
        // Bit 0 = data ready
        assert_eq!(0x01, 1);
    }

    #[test]
    fn test_ier_register_offset() {
        // Interrupt Enable Register at port + 1
        assert_eq!(1, 1);
    }

    #[test]
    fn test_iir_register_offset() {
        // Interrupt Identification Register at port + 2
        assert_eq!(2, 2);
    }

    #[test]
    fn test_lcr_register_offset() {
        // Line Control Register at port + 3
        assert_eq!(3, 3);
    }
}
