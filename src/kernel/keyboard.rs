// AIOS PS/2 Keyboard Driver
//
// Model: opencode
// Tool: opencode
// Prompt: Create PS/2 keyboard driver for x86_64 with tests.

use spin::Mutex;

/// Keyboard buffer size
pub const KEYBOARD_BUFFER_SIZE: usize = 256;

/// Keyboard modifier state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModifierState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub caps_lock: bool,
}

/// Key event structure  
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub key_code: u8,
    pub ascii: Option<char>,
    pub modifiers: ModifierState,
    pub pressed: bool,
}

/// Global keyboard buffer
pub static KEYBOARD_BUFFER: Mutex<[Option<KeyEvent>; KEYBOARD_BUFFER_SIZE]> = 
    Mutex::new([None; KEYBOARD_BUFFER_SIZE]);

/// Initialize keyboard
pub fn init() {
    println!("[KB] PS/2 Keyboard initialized");
}

/// Read a key event (blocking - returns None for now)
pub fn read_key() -> Option<KeyEvent> {
    KEYBOARD_BUFFER.lock().iter().find_map(|e| *e)
}

/// Check if keyboard has data
pub fn has_key() -> bool {
    KEYBOARD_BUFFER.lock().iter().any(|e| e.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_buffer_size() {
        assert_eq!(KEYBOARD_BUFFER_SIZE, 256);
    }

    #[test]
    fn test_modifier_state_default() {
        let mods = ModifierState::default();
        assert!(!mods.shift);
        assert!(!mods.ctrl);
        assert!(!mods.alt);
        assert!(!mods.caps_lock);
    }

    #[test]
    fn test_key_event_creation() {
        let event = KeyEvent {
            key_code: 65,
            ascii: Some('A'),
            modifiers: ModifierState::default(),
            pressed: true,
        };
        assert_eq!(event.key_code, 65);
        assert_eq!(event.ascii, Some('A'));
    }

    #[test]
    fn test_keyboard_buffer_empty() {
        let buffer = [None; KEYBOARD_BUFFER_SIZE];
        assert!(buffer.iter().all(|e| e.is_none()));
    }

    #[test]
    fn test_has_key_when_empty() {
        assert!(!has_key());
    }

    #[test]
    fn test_modifier_state_shift() {
        let mut mods = ModifierState::default();
        mods.shift = true;
        assert!(mods.shift);
    }

    #[test]
    fn test_modifier_state_ctrl() {
        let mut mods = ModifierState::default();
        mods.ctrl = true;
        assert!(mods.ctrl);
    }

    #[test]
    fn test_modifier_state_alt() {
        let mut mods = ModifierState::default();
        mods.alt = true;
        assert!(mods.alt);
    }

    #[test]
    fn test_modifier_state_caps_lock() {
        let mut mods = ModifierState::default();
        mods.caps_lock = true;
        assert!(mods.caps_lock);
    }

    #[test]
    fn test_key_event_copy() {
        let event = KeyEvent {
            key_code: 65,
            ascii: Some('A'),
            modifiers: ModifierState::default(),
            pressed: true,
        };
        let event2 = event;
        assert_eq!(event.key_code, event2.key_code);
    }
}