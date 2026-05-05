// AIOS VGA Text Mode Driver
//
// Model: opencode
// Tool: opencode
// Prompt: Implement VGA text mode buffer driver for 80x25
//         color text display with scroll support.

use core::fmt;
use volatile::Volatile;

/// VGA text buffer at physical address 0xB8000
#[repr(transparent)]
pub struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub const BUFFER_WIDTH: usize = 80;
pub const BUFFER_HEIGHT: usize = 25;

/// VGA color codes
#[allow(dead_code)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// Combine foreground and background color into single byte
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(fg: Color, bg: Color) -> Self {
        ColorCode((bg as u8) << 4 | (fg as u8))
    }
}

/// Single character cell with color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii: u8,
    color_code: ColorCode,
}

/// VGA text writer with cursor tracking
pub struct Writer {
    col: usize,
    row: usize,
    fg: Color,
    bg: Color,
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

    /// Write a byte to the VGA buffer
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b if (32..=126).contains(&b) => {
                if self.col >= BUFFER_WIDTH {
                    self.new_line();
                }

                let buffer_ptr = 0xb8000 as *mut Buffer;
                let buffer = unsafe { &mut *buffer_ptr };

                buffer.chars[self.row][self.col].write(ScreenChar {
                    ascii: byte,
                    color_code: ColorCode::new(self.fg, self.bg),
                });

                self.col += 1;
            }
            _ => {
                // Write replacement character
                self.write_byte(b'?');
            }
        }
    }

    /// Write a string to the VGA buffer
    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    /// Move to next line, scrolling if necessary
    fn new_line(&mut self) {
        if self.row + 1 < BUFFER_HEIGHT {
            self.row += 1;
        } else {
            self.scroll();
        }
        self.col = 0;
    }

    /// Scroll the screen up by one line
    fn scroll(&mut self) {
        let buffer_ptr = 0xb8000 as *mut Buffer;
        let buffer = unsafe { &mut *buffer_ptr };

        // Copy each line up by one
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let ch = buffer.chars[row][col].read();
                buffer.chars[row - 1][col].write(ch);
            }
        }

        // Clear last line
        for col in 0..BUFFER_WIDTH {
            buffer.chars[BUFFER_HEIGHT - 1][col].write(ScreenChar {
                ascii: b' ',
                color_code: ColorCode::new(self.fg, self.bg),
            });
        }
    }

    /// Clear the entire screen
    pub fn clear(&mut self) {
        let buffer_ptr = 0xb8000 as *mut Buffer;
        let buffer = unsafe { &mut *buffer_ptr };

        for row in 0..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                buffer.chars[row][col].write(ScreenChar {
                    ascii: b' ',
                    color_code: ColorCode::new(self.fg, self.bg),
                });
            }
        }

        self.col = 0;
        self.row = 0;
    }
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}
