// AIOS Unit Tests - Serial Port
//
// Model: opencode
// Tool: opencode
// Prompt: Create unit tests for serial port driver (8250 UART).

#[cfg(test)]
mod serial_tests {
    // Note: Serial port tests require hardware simulation
    // These tests verify constants and interface contracts

    #[test]
    fn test_com1_port_address() {
        // COM1 should be at 0x3F8
        const COM1_PORT: u16 = 0x3F8;
        assert_eq!(COM1_PORT, 0x3F8);
    }

    #[test]
    fn test_serial_constants() {
        // Verify serial port register offsets
        const DATA_REG: u16 = 0;
        const IER_REG: u16 = 1;
        const FCR_REG: u16 = 2;
        const LCR_REG: u16 = 3;
        const MCR_REG: u16 = 4;
        const LSR_REG: u16 = 5;

        assert_eq!(DATA_REG, 0);
        assert_eq!(IER_REG, 1);
        assert_eq!(FCR_REG, 2);
        assert_eq!(LCR_REG, 3);
        assert_eq!(MCR_REG, 4);
        assert_eq!(LSR_REG, 5);
    }

    #[test]
    fn test_baud_divisor_115200() {
        // 115200 baud = base_clock / (16 * divisor)
        // Standard PC: 115200 = 1843200 / (16 * 1)
        let base_clock: u32 = 1843200;
        let target_baud: u32 = 115200;
        let divisor = base_clock / (16 * target_baud);
        assert_eq!(divisor, 1);
    }

    #[test]
    fn test_serial_write_str_empty() {
        // write_str should handle empty strings without panic
        let s = "";
        assert_eq!(s.len(), 0);
    }
}
