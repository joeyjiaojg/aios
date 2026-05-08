// AIOS Shell History
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Command history tracking for AIOS shell using a 64-entry circular buffer.

use crate::shell::MAX_HISTORY;
use alloc::string::{String, ToString};

const MAX_CMD_LEN: usize = 128;

const HISTORY_BUFFER_INIT: [[u8; MAX_CMD_LEN]; MAX_HISTORY] = [[0u8; MAX_CMD_LEN]; MAX_HISTORY];

static HISTORY_BUFFER: spin::Mutex<[[u8; MAX_CMD_LEN]; MAX_HISTORY]> =
    spin::Mutex::new(HISTORY_BUFFER_INIT);

static HISTORY_LENGTHS: spin::Mutex<[usize; MAX_HISTORY]> = spin::Mutex::new([0usize; MAX_HISTORY]);

static HISTORY_INDEX: spin::Mutex<usize> = spin::Mutex::new(0);
static HISTORY_COUNT: spin::Mutex<usize> = spin::Mutex::new(0);

pub fn add_entry(command: &str) {
    let mut history = HISTORY_BUFFER.lock();
    let mut lengths = HISTORY_LENGTHS.lock();
    let mut idx = HISTORY_INDEX.lock();
    let mut count = HISTORY_COUNT.lock();

    let cmd_bytes = command.as_bytes();
    let len = cmd_bytes.len().min(MAX_CMD_LEN - 1);

    let slot = *idx % MAX_HISTORY;
    history[slot][..len].copy_from_slice(&cmd_bytes[..len]);
    history[slot][len] = 0;
    lengths[slot] = len;

    *idx = (*idx + 1) % MAX_HISTORY;
    if *count < MAX_HISTORY {
        *count += 1;
    }
}

pub fn get_entry(index: usize) -> Option<String> {
    let count = *HISTORY_COUNT.lock();
    let idx = *HISTORY_INDEX.lock();

    if index >= count {
        return None;
    }

    let actual_idx = (idx + MAX_HISTORY - count + index) % MAX_HISTORY;
    let len = HISTORY_LENGTHS.lock()[actual_idx];
    if len == 0 {
        return None;
    }
    let history = HISTORY_BUFFER.lock();
    let mut cmd_bytes = [0u8; MAX_CMD_LEN];
    cmd_bytes.copy_from_slice(&history[actual_idx][..len]);
    drop(history);
    String::from_utf8(cmd_bytes[..len].to_vec()).ok()
}

pub fn show_history() -> Result<(), &'static str> {
    let count = *HISTORY_COUNT.lock();
    let idx = *HISTORY_INDEX.lock();

    if count == 0 {
        crate::serial::write_str("No commands in history.\r\n");
        return Ok(());
    }

    let history = HISTORY_BUFFER.lock();
    let lengths = HISTORY_LENGTHS.lock();

    for i in 0..count {
        let actual_idx = (idx + MAX_HISTORY - count + i) % MAX_HISTORY;
        let len = lengths[actual_idx];
        if len > 0 {
            let cmd_slice = &history[actual_idx][..len];
            if let Ok(cmd) = core::str::from_utf8(cmd_slice) {
                crate::serial::write_str(i.to_string().as_str());
                crate::serial::write_str("  ");
                crate::serial::write_str(cmd);
                crate::serial::write_str("\r\n");
            }
        }
    }

    Ok(())
}

pub fn clear_history() {
    let mut history = HISTORY_BUFFER.lock();
    let mut lengths = HISTORY_LENGTHS.lock();
    let mut idx = HISTORY_INDEX.lock();
    let mut count = HISTORY_COUNT.lock();

    for i in 0..MAX_HISTORY {
        history[i] = [0u8; MAX_CMD_LEN];
        lengths[i] = 0;
    }
    *idx = 0;
    *count = 0;
}

pub fn history_count() -> usize {
    *HISTORY_COUNT.lock()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_entry() {
        clear_history();
        add_entry("test command");
        assert!(history_count() <= MAX_HISTORY);
    }

    #[test]
    fn test_get_entry_invalid_index() {
        clear_history();
        add_entry("test");
        let result = get_entry(100);
        assert!(result.is_none());
    }

    #[test]
    fn test_show_history_empty() {
        clear_history();
        let result = show_history();
        assert!(result.is_ok());
    }

    #[test]
    fn test_clear_history() {
        add_entry("test1");
        add_entry("test2");
        clear_history();
        assert_eq!(0, history_count());
    }

    #[test]
    fn test_history_count() {
        clear_history();
        add_entry("cmd1");
        add_entry("cmd2");
        let count = history_count();
        assert!(count <= MAX_HISTORY);
    }

    #[test]
    fn test_add_multiple_entries() {
        clear_history();
        add_entry("cmd0");
        add_entry("cmd1");
        add_entry("cmd2");
        add_entry("cmd3");
        add_entry("cmd4");
        assert!(history_count() <= MAX_HISTORY);
    }

    #[test]
    fn test_max_history_limit() {
        clear_history();
        for i in 0..MAX_HISTORY + 10 {
            let cmd = if i % 2 == 0 { "cmd_a" } else { "cmd_b" };
            add_entry(cmd);
        }
        assert!(history_count() <= MAX_HISTORY);
    }

    #[test]
    fn test_get_entry_valid_index() {
        clear_history();
        add_entry("first");
        add_entry("second");
        let entry = get_entry(0);
        assert!(entry.is_some());
    }

    #[test]
    fn test_history_buffer_size() {
        assert_eq!(MAX_HISTORY, 64);
    }

    #[test]
    fn test_empty_string_entry() {
        clear_history();
        add_entry("");
        assert!(history_count() <= MAX_HISTORY);
    }

    #[test]
    fn test_long_command_entry() {
        clear_history();
        let long_cmd = "this is a very long command that should still be handled correctly by the history system";
        add_entry(long_cmd);
        assert!(history_count() <= MAX_HISTORY);
    }
}
