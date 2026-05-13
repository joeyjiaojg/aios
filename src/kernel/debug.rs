// AIOS Debug Control
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Implement debug flag mechanism to control verbose output in kernel

use core::sync::atomic::{AtomicBool, Ordering};

static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn enable_debug() {
    DEBUG_ENABLED.store(true, Ordering::Relaxed);
}

pub fn disable_debug() {
    DEBUG_ENABLED.store(false, Ordering::Relaxed);
}

pub fn is_debug_enabled() -> bool {
    DEBUG_ENABLED.load(Ordering::Relaxed)
}

pub fn toggle_debug() {
    let current = DEBUG_ENABLED.load(Ordering::Relaxed);
    DEBUG_ENABLED.store(!current, Ordering::Relaxed);
}

#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        if $crate::debug::is_debug_enabled() {
            $crate::serial::write_str(&format!($($arg)*));
            $crate::serial::write_str("\r\n");
        }
    };
}

#[macro_export]
macro_rules! debug_write {
    ($($arg:tt)*) => {
        if $crate::debug::is_debug_enabled() {
            $crate::serial::write_str(&format!($($arg)*));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_disabled_by_default() {
        assert!(!is_debug_enabled());
    }

    #[test]
    fn test_enable_debug() {
        disable_debug();
        enable_debug();
        assert!(is_debug_enabled());
    }

    #[test]
    fn test_disable_debug() {
        enable_debug();
        disable_debug();
        assert!(!is_debug_enabled());
    }

    #[test]
    fn test_toggle_debug() {
        disable_debug();
        toggle_debug();
        assert!(is_debug_enabled());
        toggle_debug();
        assert!(!is_debug_enabled());
    }
}
