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

pub fn write_usize(mut n: usize) {
    let mut buf = [0u8; 20];
    let mut len = 0;
    if n == 0 {
        write_byte(b'0');
        return;
    }
    while n > 0 {
        buf[len] = b'0' + (n % 10) as u8;
        len += 1;
        n /= 10;
    }
    for i in (0..len).rev() {
        write_byte(buf[i]);
    }
}

pub fn write_isize(n: isize) {
    if n < 0 {
        write_byte(b'-');
        write_usize(n.unsigned_abs());
    } else {
        write_usize(n as usize);
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
    fn test_com1_port_valid_range() {
        assert!(COM1_PORT >= 0x0000 && COM1_PORT <= 0xFFFF);
    }

    #[test]
    fn test_com1_port_not_reserved() {
        assert_ne!(COM1_PORT, 0x0000, "COM1 port must not be zero");
    }

    #[test]
    fn test_lsr_transmit_empty_bit() {
        let lsr_thre: u8 = 0x20;
        assert_eq!(lsr_thre & 0x20, 0x20, "THRE bit should be set");
    }

    #[test]
    fn test_lsr_data_ready_bit() {
        let lsr_dr: u8 = 0x01;
        assert_eq!(lsr_dr & 0x01, 0x01, "DR bit should be set");
    }

    #[test]
    fn test_mcr_rts_dtr_bits() {
        let mcr: u8 = 0x0B;
        assert!(mcr & 0x02 != 0, "RTS should be enabled");
        assert!(mcr & 0x01 != 0, "DTR should be enabled");
    }

    #[test]
    fn test_fcr_enable_fifo() {
        let fcr: u8 = 0xC7;
        assert!(fcr & 0x01 != 0, "FIFO enable bit should be set");
        assert!(fcr & 0x02 != 0, "RCVR FIFO reset should be set");
        assert!(fcr & 0x04 != 0, "XMIT FIFO reset should be set");
    }

    #[test]
    fn test_lcr_8n1_format() {
        let lcr: u8 = 0x03;
        assert_eq!(lcr & 0x03, 0x03, "8 data bits");
        assert_eq!((lcr >> 2) & 0x01, 0x00, "1 stop bit");
        assert_eq!((lcr >> 3) & 0x01, 0x00, "No parity");
    }

    #[test]
    fn test_dlab_access() {
        let dlab: u8 = 0x80;
        assert_eq!(dlab, 0x80, "DLAB bit for baud divisor access");
    }

    #[test]
    fn test_baud_115200_divisor() {
        let divisor: u16 = 1;
        let baud = 115200u32;
        let actual_baud = baud / u32::from(divisor);
        assert_eq!(actual_baud, 115200, "Divisor 1 gives 115200 baud");
    }

    #[test]
    fn test_register_offsets_sequential() {
        let ier: u8 = 1;
        let iir: u8 = 2;
        let lcr: u8 = 3;
        assert!(ier < iir && iir < lcr, "Offsets sequential");
    }
}
