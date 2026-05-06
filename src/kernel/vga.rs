// AIOS VGA Text Mode Driver (stub)
//
// Model: opencode
// Tool: opencode
// Prompt: Implement VGA text mode buffer driver stub.

/// VGA text writer with cursor tracking
pub struct Writer;

impl Writer {
    pub fn new() -> Self { Writer }
    
    pub fn write_byte(&mut self, _byte: u8) {}
    pub fn write_str(&mut self, s: &str) { for b in s.bytes() { self.write_byte(b); } }
    pub fn clear(&mut self) {}
}

impl Default for Writer {
    fn default() -> Self { Self::new() }
}