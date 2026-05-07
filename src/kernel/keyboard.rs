// AIOS PS/2 Keyboard Driver
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Implement PS/2 keyboard driver for AIOS x86_64 kernel in Rust no_std.
//         Include keyboard initialization, IRQ handler, scancode decoding (set 1),
//         circular buffer for keypresses, and read_key() function.

use spin::Mutex;

const KEYBOARD_BUFFER_SIZE: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModifierState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub caps_lock: bool,
}

impl ModifierState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub scancode: u8,
    pub pressed: bool,
}

#[derive(Debug)]
struct KeyboardBuffer {
    buffer: [Option<KeyEvent>; KEYBOARD_BUFFER_SIZE],
    head: usize,
    tail: usize,
    count: usize,
}

impl KeyboardBuffer {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            buffer: [None; KEYBOARD_BUFFER_SIZE],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn push(&mut self, event: KeyEvent) -> bool {
        if self.count >= KEYBOARD_BUFFER_SIZE {
            return false;
        }
        self.buffer[self.tail] = Some(event);
        self.tail = (self.tail + 1) % KEYBOARD_BUFFER_SIZE;
        self.count += 1;
        true
    }

    fn pop(&mut self) -> Option<KeyEvent> {
        if self.count == 0 {
            return None;
        }
        let event = self.buffer[self.head];
        self.buffer[self.head] = None;
        self.head = (self.head + 1) % KEYBOARD_BUFFER_SIZE;
        self.count -= 1;
        event
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }
}

static KEYBOARD_BUFFER: Mutex<KeyboardBuffer> = Mutex::new(KeyboardBuffer {
    buffer: [None; KEYBOARD_BUFFER_SIZE],
    head: 0,
    tail: 0,
    count: 0,
});
static MODIFIERS: Mutex<ModifierState> = Mutex::new(ModifierState {
    shift: false,
    ctrl: false,
    alt: false,
    caps_lock: false,
});

#[allow(dead_code)]
const SCANCODE_MAP: [Option<char>; 86] = [
    None,         // 0x00
    Some('\x1b'), // 0x01 - Esc
    Some('1'),    // 0x02
    Some('2'),    // 0x03
    Some('3'),    // 0x04
    Some('4'),    // 0x05
    Some('5'),    // 0x06
    Some('6'),    // 0x07
    Some('7'),    // 0x08
    Some('8'),    // 0x09
    Some('9'),    // 0x0A
    Some('0'),    // 0x0B
    Some('-'),    // 0x0C
    Some('='),    // 0x0D
    Some('\x08'), // 0x0E - Backspace
    Some('\t'),   // 0x0F - Tab
    Some('q'),    // 0x10
    Some('w'),    // 0x11
    Some('e'),    // 0x12
    Some('r'),    // 0x13
    Some('t'),    // 0x14
    Some('y'),    // 0x15
    Some('u'),    // 0x16
    Some('i'),    // 0x17
    Some('o'),    // 0x18
    Some('p'),    // 0x19
    Some('['),    // 0x1A
    Some(']'),    // 0x1B
    Some('\n'),   // 0x1C - Enter
    None,         // 0x1D - Left Ctrl
    Some('a'),    // 0x1E
    Some('s'),    // 0x1F
    Some('d'),    // 0x20
    Some('f'),    // 0x21
    Some('g'),    // 0x22
    Some('h'),    // 0x23
    Some('j'),    // 0x24
    Some('k'),    // 0x25
    Some('l'),    // 0x26
    Some(';'),    // 0x27
    Some('\''),   // 0x28
    Some('`'),    // 0x29
    None,         // 0x2A - Left Shift
    Some('\\'),   // 0x2B
    Some('z'),    // 0x2C
    Some('x'),    // 0x2D
    Some('c'),    // 0x2E
    Some('v'),    // 0x2F
    Some('b'),    // 0x30
    Some('n'),    // 0x31
    Some('m'),    // 0x32
    Some(','),    // 0x33
    Some('.'),    // 0x34
    Some('/'),    // 0x35
    None,         // 0x36 - Right Shift
    Some('*'),    // 0x37
    None,         // 0x38 - Left Alt
    Some(' '),    // 0x39 - Space
    None,         // 0x3A - Caps Lock
    None,         // 0x3B - F1
    None,         // 0x3C - F2
    None,         // 0x3D - F3
    None,         // 0x3E - F4
    None,         // 0x3F - F5
    None,         // 0x40 - F6
    None,         // 0x41 - F7
    None,         // 0x42 - F8
    None,         // 0x43 - F9
    None,         // 0x44 - F10
    None,         // 0x57 - F11
    None,         // 0x58 - F12
    None,         // 0x45 - Num Lock
    None,         // 0x46 - Scroll Lock
    None,         // 0x47 - Home
    None,         // 0x48 - Up
    None,         // 0x49 - Page Up
    None,         // 0x4A - Keypad -
    None,         // 0x4B - Left
    None,         // 0x4C - Center
    None,         // 0x4D - Right
    None,         // 0x4E - Keypad +
    None,         // 0x4F - End
    None,         // 0x50 - Down
    None,         // 0x51 - Page Down
    None,         // 0x52 - Insert
    None,         // 0x53 - Delete
];

fn decode_scancode(scancode: u8) -> Option<KeyEvent> {
    let pressed = (scancode & 0x80) == 0;
    let code = scancode & 0x7F;

    match code {
        0x2A | 0x36 => {
            let mut m = MODIFIERS.lock();
            m.shift = pressed;
            return None;
        }
        0x1D => {
            let mut m = MODIFIERS.lock();
            m.ctrl = pressed;
            return None;
        }
        0x38 => {
            let mut m = MODIFIERS.lock();
            m.alt = pressed;
            return None;
        }
        0x3A => {
            if pressed {
                let mut m = MODIFIERS.lock();
                m.caps_lock = !m.caps_lock;
            }
            return None;
        }
        _ => {}
    }

    if !pressed {
        return None;
    }

    Some(KeyEvent {
        scancode: code,
        pressed: true,
    })
}

pub fn handle_keyboard_interrupt(scancode: u8) {
    if let Some(event) = decode_scancode(scancode) {
        let mut buffer = KEYBOARD_BUFFER.lock();
        let _ = buffer.push(event);
    }
}

pub fn init() {
    use x86_64::instructions::port::Port;
    // SAFETY: Reading from PS/2 keyboard data port (0x60) clears any pending scancodes.
    // This is the standard way to reset keyboard state; reading discards data.
    unsafe {
        let mut port = Port::new(0x60);
        let _: u8 = port.read();
    }
    *MODIFIERS.lock() = ModifierState::new();
}

pub fn read_key() -> Option<KeyEvent> {
    KEYBOARD_BUFFER.lock().pop()
}

pub fn has_key() -> bool {
    !KEYBOARD_BUFFER.lock().is_empty()
}

pub fn get_modifiers() -> ModifierState {
    *MODIFIERS.lock()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_buffer_new_is_empty() {
        let buf = KeyboardBuffer::new();
        assert!(buf.is_empty());
    }

    #[test]
    fn test_keyboard_buffer_push_pop() {
        let mut buf = KeyboardBuffer::new();
        let event = KeyEvent {
            scancode: 0x10,
            pressed: true,
        };
        assert!(buf.push(event));
        assert!(!buf.is_empty());
        assert_eq!(buf.pop(), Some(event));
        assert!(buf.is_empty());
    }

    #[test]
    fn test_keyboard_buffer_full() {
        let mut buf = KeyboardBuffer::new();
        for _ in 0..KEYBOARD_BUFFER_SIZE {
            assert!(buf.push(KeyEvent {
                scancode: 0,
                pressed: true
            }));
        }
        assert!(!buf.push(KeyEvent {
            scancode: 0,
            pressed: true
        }));
    }

    #[test]
    fn test_scancode_decode_q() {
        let event = decode_scancode(0x10);
        assert!(event.is_some());
        let e = event.unwrap();
        assert_eq!(e.scancode, 0x10);
        assert!(e.pressed);
    }

    #[test]
    fn test_scancode_decode_escape() {
        let event = decode_scancode(0x01);
        assert!(event.is_some());
    }

    #[test]
    fn test_scancode_decode_release() {
        let event = decode_scancode(0x90);
        assert!(event.is_none());
    }

    #[test]
    fn test_scancode_decode_ctrl() {
        let event = decode_scancode(0x1D);
        assert!(event.is_none());
    }

    #[test]
    fn test_modifier_shift() {
        decode_scancode(0x2A);
        let m = MODIFIERS.lock();
        assert!(m.shift);
    }

    #[test]
    fn test_modifier_ctrl() {
        decode_scancode(0x1D);
        let m = MODIFIERS.lock();
        assert!(m.ctrl);
    }

    #[test]
    fn test_modifier_alt() {
        decode_scancode(0x38);
        let m = MODIFIERS.lock();
        assert!(m.alt);
    }

    #[test]
    fn test_modifier_state_default() {
        let m = ModifierState::new();
        assert!(!m.shift);
        assert!(!m.ctrl);
        assert!(!m.alt);
        assert!(!m.caps_lock);
    }

    #[test]
    fn test_handle_interrupt() {
        handle_keyboard_interrupt(0x1E);
        assert!(has_key());
    }

    #[test]
    fn test_read_key() {
        handle_keyboard_interrupt(0x10);
        let key = read_key();
        assert!(key.is_some());
    }

    #[test]
    fn test_scancode_map() {
        assert_eq!(SCANCODE_MAP[0x10], Some('q'));
        assert_eq!(SCANCODE_MAP[0x11], Some('w'));
        assert_eq!(SCANCODE_MAP[0x12], Some('e'));
    }

    #[test]
    fn test_keyboard_init() {
        init();
    }
}
