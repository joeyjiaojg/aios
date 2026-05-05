// AIOS Serial Port Driver (8250 UART)
//
// Model: opencode
// Tool: opencode
// Prompt: Implement 8250 UART serial port driver for x86_64
//         with init, write_byte, and write_str functions.

use core::fmt;

const COM1_PORT: u16 = 0x3F8;

/// Initialize COM1 serial port at 115200 baud
pub fn init() {
    // Disable all interrupts
    outb(COM1_PORT + 1, 0x00);

    // Enable DLAB (set baud rate divisor)
    outb(COM1_PORT + 3, 0x80);

    // Set divisor to 1 (115200 baud)
    outb(COM1_PORT + 0, 0x01);
    outb(COM1_PORT + 1, 0x00);

    // 8 bits, no parity, one stop bit
    outb(COM1_PORT + 3, 0x03);

    // Enable FIFO, clear them, with 14-byte threshold
    outb(COM1_PORT + 2, 0xC7);

    // Enable IRQs, set RTS/DSR
    outb(COM1_PORT + 4, 0x0B);

    // Set loopback mode for self-test
    outb(COM1_PORT + 4, 0x1E);

    // Write test byte
    outb(COM1_PORT + 0, 0xAE);

    // Check if we read back the same byte
    assert_eq!(inb(COM1_PORT + 0), 0xAE);

    // Set normal operation mode
    outb(COM1_PORT + 4, 0x0F);
}

/// Check if serial port has data ready to read
pub fn has_data() -> bool {
    inb(COM1_PORT + 5) & 1 != 0
}

/// Read a byte from serial port
pub fn read_byte() -> u8 {
    while inb(COM1_PORT + 5) & 1 == 0 {}
    inb(COM1_PORT)
}

/// Write a byte to serial port
pub fn write_byte(byte: u8) {
    while inb(COM1_PORT + 5) & 0x20 == 0 {}
    outb(COM1_PORT, byte);
}

/// Write a string to serial port
pub fn write_str(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

/// Read a byte from an I/O port
unsafe fn inb(port: u16) -> u8 {
    x86_64::instructions::port::Port::new(port).read()
}

/// Write a byte to an I/O port
unsafe fn outb(port: u16, data: u8) {
    let mut p = x86_64::instructions::port::Port::new(port);
    p.write(data);
}

/// Implement core::fmt::Write for serial port
pub struct SerialPort;

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_str(s);
        Ok(())
    }
}

/// Global println macro for kernel debugging
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
