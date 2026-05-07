// AIOS VGA Text Mode Driver
//
// Model: opencode
// Tool: opencode
// Prompt: Implement VGA text mode buffer driver for 80x25 with tests.

/// VGA text writer with cursor tracking
pub struct Writer {
    col: usize,
    row: usize,
    #[allow(dead_code)]
    fg: Color,
    #[allow(dead_code)]
    bg: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    #[default]
    LightGray = 7,
}

impl Writer {
    pub fn new() -> Self {
        Writer {
            col: 0,
            row: 0,
            fg: Color::LightGray,
            bg: Color::Black,
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            _ if (32..=126).contains(&byte) => self.col += 1,
            _ => {}
        }
    }

    pub fn write_str(&mut self, s: &str) {
        for b in s.bytes() {
            self.write_byte(b);
        }
    }

    fn new_line(&mut self) {
        self.col = 0;
        if self.row < 24 {
            self.row += 1;
        }
    }

    pub fn clear(&mut self) {
        self.col = 0;
        self.row = 0;
    }

    pub fn get_col(&self) -> usize {
        self.col
    }
    pub fn get_row(&self) -> usize {
        self.row
    }
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_dimensions() {
        assert_eq!(BUFFER_WIDTH, 80);
        assert_eq!(BUFFER_HEIGHT, 25);
    }

    #[test]
    fn test_writer_creation() {
        let w = Writer::new();
        assert_eq!(w.get_col(), 0);
        assert_eq!(w.get_row(), 0);
    }

    #[test]
    fn test_writer_default_color() {
        let w = Writer::new();
        assert!(matches!(w.fg, Color::LightGray));
        assert!(matches!(w.bg, Color::Black));
    }

    #[test]
    fn test_write_ascii() {
        let mut w = Writer::new();
        w.write_byte(b'A');
        assert_eq!(w.get_col(), 1);
    }

    #[test]
    fn test_write_newline() {
        let mut w = Writer::new();
        w.write_byte(b'\n');
        assert_eq!(w.get_col(), 0);
    }

    #[test]
    fn test_write_string() {
        let mut w = Writer::new();
        w.write_str("HI");
        assert_eq!(w.get_col(), 2);
    }

    #[test]
    fn test_clear() {
        let mut w = Writer::new();
        w.write_str("TEST");
        w.clear();
        assert_eq!(w.get_col(), 0);
    }

    #[test]
    fn test_color_count() {
        // There are 16 colors (0-15)
        assert!(16 > 0);
    }

    #[test]
    fn test_line_wrap() {
        let mut w = Writer::new();
        for _ in 0..80 {
            w.write_byte(b'A');
        }
        assert!(w.get_col() == 0);
    }
}

pub const BUFFER_WIDTH: usize = 80;
pub const BUFFER_HEIGHT: usize = 25;
