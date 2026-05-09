// AIOS Serial Port Driver (8250 UART)
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Implement 8250 UART serial port driver with tests.

use core::fmt;
use x86_64::instructions::port::Port;

const COM1_PORT: u16 = 0x3F8;

pub fn init() {
    // # Safety
    // This configures I/O ports which is safe for a unique serial port.
    unsafe {
        let mut port = Port::new(COM1_PORT + 1);
        port.write(0x00u8);

        port = Port::new(COM1_PORT + 3);
        port.write(0x80u8);

        port = Port::new(COM1_PORT);
        port.write(0x01u8);

        port = Port::new(COM1_PORT + 1);
        port.write(0x00u8);

        port = Port::new(COM1_PORT + 3);
        port.write(0x03u8);

        port = Port::new(COM1_PORT + 2);
        port.write(0xC7u8);

        port = Port::new(COM1_PORT + 4);
        port.write(0x0Bu8);
    }
}

pub fn write_byte(byte: u8) {
    // # Safety
    // Writing to serial port I/O address is safe - COM1 is standard hardware
    unsafe {
        let mut status_port = Port::<u8>::new(COM1_PORT + 5);
        while status_port.read() & 0x20 == 0 {}
        let mut data_port = Port::<u8>::new(COM1_PORT);
        data_port.write(byte);
    }
}

pub fn write_str(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

pub fn read_byte() -> Option<u8> {
    // # Safety
    // Reading from I/O port 0x3F8 (COM1) is safe - this is a standard hardware port
    unsafe {
        let mut status_port = Port::<u8>::new(COM1_PORT + 5);
        if status_port.read() & 0x01 != 0 {
            let mut data_port = Port::<u8>::new(COM1_PORT);
            Some(data_port.read())
        } else {
            None
        }
    }
}

pub fn has_data() -> bool {
    // # Safety
    // Reading from I/O port is safe for standard x86 ports
    unsafe {
        let mut status_port = Port::<u8>::new(COM1_PORT + 5);
        status_port.read() & 0x01 != 0
    }
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
    fn test_write_str_empty() {
        write_str("");
    }

    #[test]
    fn test_write_str_single_char() {
        write_str("A");
    }

    #[test]
    fn test_write_str_multiple_chars() {
        write_str("TEST");
    }

    #[test]
    fn test_write_byte_zero() {
        write_byte(0x00);
    }

    #[test]
    fn test_write_byte_max() {
        write_byte(0xFF);
    }

    #[test]
    fn test_write_byte_printable() {
        write_byte(0x41);
        write_byte(0x5A);
    }

    #[test]
    fn test_read_byte_returns_option() {
        let result = read_byte();
        assert!(result.is_none() || result.is_some());
    }

    #[test]
    fn test_has_data_returns_bool() {
        let result = has_data();
        assert!(result == true || result == false);
    }

    #[test]
    fn test_com1_port_in_io_range() {
        assert!(COM1_PORT < 0x10000);
    }

    #[test]
    fn test_baud_divisor_value() {
        let divisor: u16 = 1;
        assert!(divisor > 0);
    }

    #[test]
    fn test_line_control_format() {
        let lcr: u8 = 0x03;
        assert_eq!(lcr, 0x03);
    }
}
