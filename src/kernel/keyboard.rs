// AIOS PS/2 Keyboard Driver
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Create PS/2 keyboard driver for AIOS x86_64 kernel in Rust no_std.
//         Implement keyboard initialization, IRQ handler, scancode decoding (set 1),
//         circular buffer for keypresses, and read_key() function.

#![no_std]

use spin::Mutex;

/// Keyboard buffer size
const KEYBOARD_BUFFER_SIZE: usize = 256;

/// Number of scancodes in set 1
const SCANCODE_COUNT: usize = 128;

/// Keyboard modifier state
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModifierState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub caps_lock: bool,
}

impl ModifierState {
    pub fn new() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
            caps_lock: false,
        }
    }
}

/// Key event structure
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyEvent {
    pub key_code: u8,
    pub ascii: Option<char>,
    pub modifiers: ModifierState,
    pub pressed: bool,
}

/// Keyboard buffer for key events
struct KeyboardBuffer {
    buffer: [KeyEvent; KEYBOARD_BUFFER_SIZE],
    head: usize,
    tail: usize,
    count: usize,
}

impl KeyboardBuffer {
    fn new() -> Self {
        Self {
            buffer: [KeyEvent {
                key_code: 0,
                ascii: None,
                modifiers: ModifierState::new(),
                pressed: false,
            }; KEYBOARD_BUFFER_SIZE],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn push(&mut self, event: KeyEvent) -> bool {
        if self.count >= KEYBOARD_BUFFER_SIZE {
            return false;
        }
        self.buffer[self.tail] = event;
        self.tail = (self.tail + 1) % KEYBOARD_BUFFER_SIZE;
        self.count += 1;
        true
    }

    fn pop(&mut self) -> Option<KeyEvent> {
        if self.count == 0 {
            return None;
        }
        let event = self.buffer[self.head];
        self.head = (self.head + 1) % KEYBOARD_BUFFER_SIZE;
        self.count -= 1;
        Some(event)
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Global keyboard buffer
static KEYBOARD_BUFFER: Mutex<KeyboardBuffer> = Mutex::new(KeyboardBuffer::new());

/// Global modifier state
static MODIFIERS: Mutex<ModifierState> = Mutex::new(ModifierState::new());

/// Scancode set 1 to ASCII mapping (without modifiers)
const SCANCODE_MAP: [Option<char>; SCANCODE_COUNT] = [
    None,       // 0x00 - Error
    None,       // 0x01 - Esc
    Some('1'),  // 0x02
    Some('2'),  // 0x03
    Some('3'),  // 0x04
    Some('4'),  // 0x05
    Some('5'),  // 0x06
    Some('6'),  // 0x07
    Some('7'),  // 0x08
    Some('8'),  // 0x09
    Some('9'),  // 0x0A
    Some('0'),  // 0x0B
    Some('-'),  // 0x0C
    Some('='),  // 0x0D
    Some('\x08'), // 0x0E - Backspace
    Some('\t'), // 0x0F - Tab
    Some('q'),  // 0x10
    Some('w'),  // 0x11
    Some('e'),  // 0x12
    Some('r'),  // 0x13
    Some('t'),  // 0x14
    Some('y'),  // 0x15
    Some('u'),  // 0x16
    Some('i'),  // 0x17
    Some('o'),  // 0x18
    Some('p'),  // 0x19
    Some('['),  // 0x1A
    Some(']'),  // 0x1B
    Some('\n'), // 0x1C - Enter
    None,       // 0x1D - Ctrl
    Some('a'),  // 0x1E
    Some('s'),  // 0x1F
    Some('d'),  // 0x20
    Some('f'),  // 0x21
    Some('g'),  // 0x22
    Some('h'),  // 0x23
    Some('j'),  // 0x24
    Some('k'),  // 0x25
    Some('l'),  // 0x26
    Some(';'),  // 0x27
    Some('\''), // 0x28
    Some('`'),  // 0x29
    None,       // 0x2A - Left Shift
    Some('\\'), // 0x2B
    Some('z'),  // 0x2C
    Some('x'),  // 0x2D
    Some('c'),  // 0x2E
    Some('v'),  // 0x2F
    Some('b'),  // 0x30
    Some('n'),  // 0x31
    Some('m'),  // 0x32
    Some(','),  // 0x33
    Some('.'),  // 0x34
    Some('/'),  // 0x35
    None,       // 0x36 - Right Shift
    Some('*'),  // 0x37 - Keypad *
    None,       // 0x38 - Alt
    Some(' '),  // 0x39 - Space
    None,       // 0x3A - Caps Lock
    Some('F1'), // 0x3B - F1
    Some('F2'), // 0x3C - F2
    Some('F3'), // 0x3D - F3
    Some('F4'), // 0x3E - F4
    Some('F5'), // 0x3F - F5
    Some('F6'), // 0x40 - F6
    Some('F7'), // 0x41 - F7
    Some('F8'), // 0x42 - F8
    Some('F9'), // 0x43 - F9
    Some('F10'), // 0x44 - F10
    Some('F11'), // 0x57 - F11 (scancode extended)
    Some('F12'), // 0x58 - F12 (scancode extended)
    None,       // 0x45 - Num Lock (not fully mapped)
    None,       // 0x46 - Scroll Lock
    None,       // 0x47 - Home
    None,       // 0x48 - Up
    None,       // 0x49 - Page Up
    None,       // 0x4A - Keypad -
    None,       // 0x4B - Left
    None,       // 0x4C - Center
    None,       // 0x4D - Right
    None,       // 0x4E - Keypad +
    None,       // 0x4F - End
    None,       // 0x50 - Down
    None,       // 0x51 - Page Down
    None,       // 0x52 - Insert
    None,       // 0x53 - Delete
    None,       // 0x54 - Keypad Enter (scancode extended)
    None,       // 0x56 - Keypad /
    None,       // 0x5E - SysRq
    None,       // 0x5F - Pause
    None,       // 0x5C - Keypad Right Slash
    None,       // 0x68 - F13
    None,       // 0x66 - F14
    None,       // 0x67 - F15
    None,       // 0x68 - F16
    None,       // 0x69 - F17
    None,       // 0x6A - F18
    None,       // 0x6B - F19
    None,       // 0x6C - F20
    None,       // 0x6D - F21
    None,       // 0x6E - F22
    None,       // 0x76 - F23
    None,       // 0x78 - F24
];

/// Decode scancode to key event
fn decode_scancode(scancode: u8) -> Option<KeyEvent> {
    let pressed = (scancode & 0x80) == 0;
    let code = scancode & 0x7F;

    let mut modifiers = *MODIFIERS.lock();

    match code {
        0x2A => { modifiers.shift = pressed; *MODIFIERS.lock() = modifiers; return None; }
        0x36 => { modifiers.shift = pressed; *MODIFIERS.lock() = modifiers; return None; }
        0x1D => { modifiers.ctrl = pressed; *MODIFIERS.lock() = modifiers; return None; }
        0x38 => { modifiers.alt = pressed; *MODIFIERS.lock() = modifiers; return None; }
        0x3A => {
            if pressed {
                modifiers.caps_lock = !modifiers.caps_lock;
                *MODIFIERS.lock() = modifiers;
            }
            return None;
        }
        _ => {}
    }

    if code as usize >= SCANCODE_COUNT {
        return None;
    }

    let ascii = SCANCODE_MAP[code as usize];

    if !pressed {
        return None;
    }

    Some(KeyEvent {
        key_code: code,
        ascii,
        modifiers,
        pressed,
    })
}

/// Handle keyboard interrupt - call this from the IRQ1 handler
pub fn handle_keyboard_interrupt(scancode: u8) {
    if let Some(event) = decode_scancode(scancode) {
        if let Ok(mut buffer) = KEYBOARD_BUFFER.try_lock() {
            let _ = buffer.push(event);
        }
    }
}

/// Initialize the PS/2 keyboard controller
pub fn init() {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0x60);

        let _ = port.read();
    }

    let mut mods = MODIFIERS.lock();
    *mods = ModifierState::new();
}

/// Read the next key event from the keyboard buffer (non-blocking)
pub fn read_key_event() -> Option<KeyEvent> {
    KEYBOARD_BUFFER.lock().pop()
}

/// Read a key and wait until one is available
pub fn wait_for_key() -> KeyEvent {
    loop {
        if let Some(event) = read_key_event() {
            return event;
        }
    }
}

/// Check if there is a key available
pub fn has_key() -> bool {
    !KEYBOARD_BUFFER.lock().is_empty()
}

/// Get current modifier state
pub fn get_modifiers() -> ModifierState {
    *MODIFIERS.lock()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_buffer_new() {
        let buffer = KeyboardBuffer::new();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_keyboard_buffer_push_pop() {
        let mut buffer = KeyboardBuffer::new();
        let event = KeyEvent {
            key_code: 0x10,
            ascii: Some('q'),
            modifiers: ModifierState::new(),
            pressed: true,
        };
        assert!(buffer.push(event));
        assert!(!buffer.is_empty());
        assert_eq!(buffer.pop(), Some(event));
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_keyboard_buffer_full() {
        let mut buffer = KeyboardBuffer::new();
        for _ in 0..KEYBOARD_BUFFER_SIZE {
            let event = KeyEvent {
                key_code: 0,
                ascii: None,
                modifiers: ModifierState::new(),
                pressed: true,
            };
            assert!(buffer.push(event));
        }
        let event = KeyEvent {
            key_code: 0,
            ascii: None,
            modifiers: ModifierState::new(),
            pressed: true,
        };
        assert!(!buffer.push(event));
    }

    #[test]
    fn test_scancode_decode_q() {
        let event = decode_scancode(0x10);
        assert!(event.is_some());
        let e = event.unwrap();
        assert_eq!(e.key_code, 0x10);
        assert_eq!(e.ascii, Some('q'));
    }

    #[test]
    fn test_scancode_decode_caps() {
        let event = decode_scancode(0x3A);
        assert!(event.is_none());
    }

    #[test]
    fn test_scancode_decode_released() {
        let event = decode_scancode(0x90);
        assert!(event.is_none());
    }

    #[test]
    fn test_modifier_shift() {
        let mut mods = MODIFIERS.lock();
        mods.shift = true;
        drop(mods);

        let mods = get_modifiers();
        assert!(mods.shift);
    }

    #[test]
    fn test_has_key_false() {
        assert!(!has_key());
    }

    #[test]
    fn test_read_key_event_none() {
        assert_eq!(read_key_event(), None);
    }

    #[test]
    fn test_modifier_state_new() {
        let mods = ModifierState::new();
        assert!(!mods.shift);
        assert!(!mods.ctrl);
        assert!(!mods.alt);
        assert!(!mods.caps_lock);
    }
}