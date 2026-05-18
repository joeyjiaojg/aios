// AIOS Shell Module
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Shell module root - exports parser, builtins, job_control, history submodules.

pub mod builtins;
pub mod history;
pub mod job_control;
pub mod parser;

pub const MAX_INPUT_LEN: usize = 256;
pub const MAX_ARGS: usize = 16;
pub const MAX_ENV_VARS: usize = 32;
pub const MAX_JOBS: usize = 16;
pub const MAX_HISTORY: usize = 64;
pub const MAX_PATH_LEN: usize = 256;

static SHELL_RUNNING: spin::Mutex<bool> = spin::Mutex::new(true);

pub fn is_running() -> bool {
    *SHELL_RUNNING.lock()
}

pub fn set_running(running: bool) {
    *SHELL_RUNNING.lock() = running;
}

pub fn stop_shell() {
    set_running(false);
}

/// Attempt to exec /bin/sh with no arguments.
///
/// Returns `Ok(())` only if the exec path was taken — but in practice
/// `exec_cmd` never returns on success because it does `iretq` into ring 3.
/// Returns `Err` when /bin/sh is not found in the ramdisk.
pub fn try_exec_sh() -> Result<(), &'static str> {
    if crate::ramdisk::lookup_file("/bin/sh").is_none() {
        return Err("/bin/sh not found");
    }
    // exec_args[0] must be the program path; no additional arguments for a
    // login shell.
    let exec_args: [&str; 1] = ["/bin/sh"];
    builtins::exec_cmd("/bin/sh", &exec_args)
}

/// Wrapper function for jumping from syscall trampoline after process exit.
///
/// Called via `jmp` (not `call`) from the syscall trampoline so no return
/// address is on the stack.  Must never return.
///
/// Behaviour:
///   1. Try to re-exec /bin/sh.  On success exec_cmd does `iretq` and this
///      function effectively never returns — the next process exit will jump
///      here again.
///   2. If /bin/sh is not found (ramdisk has no module), fall back to the
///      built-in prompt loop so the kernel stays interactive.
#[no_mangle]
pub extern "C" fn shell_prompt_loop_entry() -> ! {
    loop {
        // Try to re-exec the external shell first.
        match try_exec_sh() {
            Ok(_) => {
                // exec_cmd returned without launching (should not happen), so
                // fall through to the built-in shell below.
            }
            Err(_) => {
                // /bin/sh not present — run the built-in interactive shell.
            }
        }

        // Fall back: run the built-in shell loop.
        set_running(true);
        shell_prompt_loop();

        // User typed 'exit' in the built-in shell: pause, then loop and try
        // again (either re-exec /bin/sh or re-enter the built-in shell).
        crate::serial::write_str("Goodbye!\r\nPress Enter to continue...\r\n");
        loop {
            if let Some(b'\r') | Some(b'\n') = crate::serial::read_byte() {
                break;
            }
            // # Safety: pause reduces CPU usage in the busy-wait.
            unsafe { core::arch::asm!("pause") }
        }
        set_running(true);
    }
}

/// Start a shell session.
///
/// Tries /bin/sh first.  If it is present in the ramdisk the kernel will
/// `iretq` into user space and this function does not meaningfully return
/// (process exit fires `PROCESS_EXITED` which jmps to `shell_prompt_loop_entry`).
/// If /bin/sh is absent, falls back to the built-in interactive shell.
pub fn run_shell() {
    // Attempt to exec the external shell.
    match try_exec_sh() {
        Ok(_) => {
            // exec_cmd returned without launching — fall through to built-in.
        }
        Err(_) => {
            // /bin/sh not found: use the built-in shell.
            crate::serial::write_str("[init] /bin/sh not found, using built-in shell\r\n");
        }
    }

    // Built-in shell fallback.
    set_running(true);
    crate::serial::write_str("AIOS Shell v1.0\r\n");
    crate::serial::write_str("Type 'help' for available commands.\r\n\r\n");
    shell_prompt_loop();
}

/// Escape sequence parser state for arrow key detection.
#[derive(Copy, Clone, PartialEq)]
enum EscState {
    /// No escape sequence in progress.
    Normal,
    /// Received 0x1B (ESC); waiting for '['.
    GotEsc,
    /// Received ESC + '['; waiting for direction character.
    GotBracket,
}

/// Redraw the input line after a history load or in-place edit.
///
/// Moves the terminal cursor to the start of the line, overwrites with the
/// new content, then erases any leftover characters from the previous line,
/// and repositions the cursor at `cursor_pos`.
fn redraw_input(buf: &[u8], len: usize, cursor_pos: usize) {
    // Return cursor to start of input (after the prompt "aios$ ").
    crate::serial::write_str("\r\x1b[6C"); // CR then move right 6 cols past prompt
                                           // Write current buffer contents.
    for &b in buf[..len].iter() {
        crate::serial::write_byte(b);
    }
    // Erase to end of line to remove stale characters from a longer previous entry.
    crate::serial::write_str("\x1b[K");
    // Reposition cursor: move back to end, then move left by (len - cursor_pos).
    let move_left = len.saturating_sub(cursor_pos);
    if move_left > 0 {
        // Emit ESC [ N D
        crate::serial::write_str("\x1b[");
        write_usize_serial(move_left);
        crate::serial::write_byte(b'D');
    }
}

/// Write a usize to serial without allocation.
fn write_usize_serial(mut n: usize) {
    if n == 0 {
        crate::serial::write_byte(b'0');
        return;
    }
    let mut buf = [0u8; 20];
    let mut len = 0;
    while n > 0 {
        buf[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    for i in (0..len).rev() {
        crate::serial::write_byte(buf[i]);
    }
}

/// Inner prompt loop — called by run_shell() and re-entered after a user process exits.
pub fn shell_prompt_loop() {
    let mut input_buf = [0u8; MAX_INPUT_LEN];
    let mut input_len: usize;

    loop {
        if !is_running() {
            break;
        }

        crate::serial::write_str("aios$ ");

        input_len = 0;
        // cursor_pos: logical index where the next character would be inserted.
        let mut cursor_pos: usize = 0;
        // history_pos: None = editing new line; Some(i) = browsing history entry i.
        // Index 0 = oldest visible entry, history_count-1 = most recent.
        let mut history_pos: Option<usize> = None;
        // Saved new-line buffer so Down-arrow can restore it.
        let mut saved_buf = [0u8; MAX_INPUT_LEN];
        let mut saved_len: usize = 0;
        // Escape sequence parser state.
        let mut esc_state = EscState::Normal;

        loop {
            match crate::serial::read_byte() {
                // ----------------------------------------------------------------
                // Enter
                // ----------------------------------------------------------------
                Some(b'\r') | Some(b'\n') => {
                    crate::serial::write_str("\r\n");
                    break;
                }
                // ----------------------------------------------------------------
                // Backspace / DEL
                // ----------------------------------------------------------------
                Some(0x7F) | Some(0x08) => {
                    esc_state = EscState::Normal;
                    if cursor_pos > 0 {
                        // Remove character at cursor_pos - 1.
                        for i in (cursor_pos - 1)..input_len.saturating_sub(1) {
                            input_buf[i] = input_buf[i + 1];
                        }
                        cursor_pos -= 1;
                        input_len -= 1;
                        input_buf[input_len] = 0;
                        // Redraw from the deletion point.
                        crate::serial::write_str("\x08"); // move left one
                        for &b in input_buf[cursor_pos..input_len].iter() {
                            crate::serial::write_byte(b);
                        }
                        crate::serial::write_str("\x1b[K"); // erase to EOL
                                                            // Move cursor back to cursor_pos.
                        let move_back = input_len - cursor_pos;
                        if move_back > 0 {
                            crate::serial::write_str("\x1b[");
                            write_usize_serial(move_back);
                            crate::serial::write_byte(b'D');
                        }
                    }
                }
                // ----------------------------------------------------------------
                // Escape — start of potential arrow key sequence
                // ----------------------------------------------------------------
                Some(0x1B) => {
                    esc_state = EscState::GotEsc;
                }
                // ----------------------------------------------------------------
                // '[' — second byte of CSI
                // ----------------------------------------------------------------
                Some(b'[') if esc_state == EscState::GotEsc => {
                    esc_state = EscState::GotBracket;
                }
                // ----------------------------------------------------------------
                // Arrow keys — final byte of CSI sequence
                // ----------------------------------------------------------------
                Some(b'A') if esc_state == EscState::GotBracket => {
                    // Up arrow: navigate history backwards (towards older entries).
                    esc_state = EscState::Normal;
                    let count = history::history_count();
                    if count == 0 {
                        // nothing to do
                    } else {
                        let new_pos = match history_pos {
                            None => {
                                // Save current editing buffer before we browse.
                                saved_buf[..input_len].copy_from_slice(&input_buf[..input_len]);
                                saved_len = input_len;
                                count - 1 // most-recent entry
                            }
                            Some(0) => 0, // already at oldest; stay
                            Some(p) => p - 1,
                        };
                        let mut tmp = [0u8; MAX_INPUT_LEN];
                        if let Some(len) = history::get_entry(new_pos, &mut tmp) {
                            input_buf[..len].copy_from_slice(&tmp[..len]);
                            input_len = len;
                            cursor_pos = len;
                            history_pos = Some(new_pos);
                            redraw_input(&input_buf, input_len, cursor_pos);
                        }
                    }
                }
                Some(b'B') if esc_state == EscState::GotBracket => {
                    // Down arrow: navigate history forwards (towards newer / blank).
                    esc_state = EscState::Normal;
                    match history_pos {
                        None => {} // already on new line, nothing to do
                        Some(p) => {
                            let count = history::history_count();
                            if p + 1 >= count {
                                // Restore the saved new-line buffer.
                                input_buf[..saved_len].copy_from_slice(&saved_buf[..saved_len]);
                                input_len = saved_len;
                                cursor_pos = saved_len;
                                history_pos = None;
                            } else {
                                let new_pos = p + 1;
                                let mut tmp = [0u8; MAX_INPUT_LEN];
                                if let Some(len) = history::get_entry(new_pos, &mut tmp) {
                                    input_buf[..len].copy_from_slice(&tmp[..len]);
                                    input_len = len;
                                    cursor_pos = len;
                                    history_pos = Some(new_pos);
                                }
                            }
                            redraw_input(&input_buf, input_len, cursor_pos);
                        }
                    }
                }
                Some(b'C') if esc_state == EscState::GotBracket => {
                    // Right arrow: move cursor right.
                    esc_state = EscState::Normal;
                    if cursor_pos < input_len {
                        cursor_pos += 1;
                        crate::serial::write_str("\x1b[C");
                    }
                }
                Some(b'D') if esc_state == EscState::GotBracket => {
                    // Left arrow: move cursor left.
                    esc_state = EscState::Normal;
                    if cursor_pos > 0 {
                        cursor_pos -= 1;
                        crate::serial::write_str("\x1b[D");
                    }
                }
                // ----------------------------------------------------------------
                // Printable characters
                // ----------------------------------------------------------------
                Some(b) if (0x20..0x7F).contains(&b) && input_len < MAX_INPUT_LEN - 1 => {
                    esc_state = EscState::Normal;
                    // Insert at cursor_pos (shift right if not at end).
                    if cursor_pos == input_len {
                        input_buf[input_len] = b;
                        input_len += 1;
                        cursor_pos += 1;
                        crate::serial::write_byte(b);
                    } else {
                        // Shift characters right to make room.
                        let mut i = input_len;
                        while i > cursor_pos {
                            input_buf[i] = input_buf[i - 1];
                            i -= 1;
                        }
                        input_buf[cursor_pos] = b;
                        input_len += 1;
                        cursor_pos += 1;
                        // Redraw from cursor_pos - 1 onwards.
                        for &b in input_buf[(cursor_pos - 1)..input_len].iter() {
                            crate::serial::write_byte(b);
                        }
                        // Move cursor back to cursor_pos.
                        let move_back = input_len - cursor_pos;
                        if move_back > 0 {
                            crate::serial::write_str("\x1b[");
                            write_usize_serial(move_back);
                            crate::serial::write_byte(b'D');
                        }
                    }
                }
                Some(_) => {
                    // Any other byte resets escape state (e.g. unrecognised escape).
                    esc_state = EscState::Normal;
                }
                None => {
                    // No byte ready - yield to reduce CPU usage
                    // # Safety
                    // Pause instruction is safe; it reduces CPU usage in busy-wait loops
                    unsafe { core::arch::asm!("pause") }
                }
            }
        }

        if input_len == 0 {
            continue;
        }

        input_buf[input_len] = 0;
        let input_str = core::str::from_utf8(&input_buf[..input_len]).unwrap_or("");

        if input_str.trim().is_empty() {
            continue;
        }

        history::add_entry(input_str);

        let (args, arg_count) = parser::split_command_args(input_str);
        if arg_count == 0 {
            continue;
        }

        let cmd = args[0];
        let result = builtins::execute_builtin(cmd, &args[..arg_count]);
        if !result {
            // Resolve to absolute path: "/bin/cmd" for bare names
            let mut path_buf = [0u8; 256];
            let resolved: &str = if cmd.starts_with('/') {
                cmd
            } else {
                let prefix = b"/bin/";
                let cb = cmd.as_bytes();
                let total = prefix.len() + cb.len();
                if total < 256 {
                    path_buf[..prefix.len()].copy_from_slice(prefix);
                    path_buf[prefix.len()..total].copy_from_slice(cb);
                    core::str::from_utf8(&path_buf[..total]).unwrap_or(cmd)
                } else {
                    cmd
                }
            };

            if crate::ramdisk::lookup_file(resolved).is_some() {
                // exec_cmd uses exec_args[0] as the program path
                let mut exec_args: [&str; MAX_ARGS] = [""; MAX_ARGS];
                exec_args[0] = resolved;
                exec_args[1..arg_count].copy_from_slice(&args[1..arg_count]);
                let _ = builtins::exec_cmd(resolved, &exec_args[..arg_count]);
            } else {
                crate::serial::write_str("Command not found: ");
                crate::serial::write_str(cmd);
                crate::serial::write_str("\r\n");
            }
        }
    }
}

pub fn get_current_dir_str() -> &'static str {
    "/"
}

pub fn get_current_dir(buf: &mut [u8]) -> usize {
    if let Some(proc) = crate::process::get_current_process() {
        let cwd = proc.get_cwd_str();
        let src_bytes = cwd.as_bytes();
        let copy_len = src_bytes.len().min(buf.len().saturating_sub(1)).min(255);
        buf[..copy_len].copy_from_slice(&src_bytes[..copy_len]);
        buf[copy_len] = 0;
        copy_len
    } else {
        buf[0] = b'/';
        buf[1] = 0;
        1
    }
}

pub fn set_current_dir(path: &str) -> Result<(), &'static str> {
    let pid = crate::process::get_current_pid();
    let mut table = crate::process::PROCESS_TABLE.lock();
    if let Some(proc) = table.get_process_mut(pid) {
        proc.set_cwd(path.as_bytes());
        Ok(())
    } else {
        Err("Failed to set directory")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── write_usize_serial ──────────────────────────────────────────────────────
    // The function emits bytes to serial; tests verify it does not panic and
    // handles edge cases correctly (we cannot capture serial output in unit
    // tests, but we can ensure the code paths are exercised without panic).

    #[test]
    fn test_shell_mod_write_usize_serial_zero() {
        // Must not panic on zero input.
        write_usize_serial(0);
    }

    #[test]
    fn test_shell_mod_write_usize_serial_one() {
        write_usize_serial(1);
    }

    #[test]
    fn test_shell_mod_write_usize_serial_large() {
        write_usize_serial(usize::MAX);
    }

    #[test]
    fn test_shell_mod_write_usize_serial_power_of_ten() {
        write_usize_serial(1000);
    }

    // ── is_running / set_running ───────────────────────────────────────────────

    #[test]
    fn test_shell_mod_set_running_true() {
        set_running(true);
        assert!(is_running());
    }

    #[test]
    fn test_shell_mod_set_running_false() {
        set_running(false);
        assert!(!is_running());
        // Restore for other tests.
        set_running(true);
    }

    #[test]
    fn test_shell_mod_stop_shell_clears_running() {
        set_running(true);
        stop_shell();
        assert!(!is_running());
        // Restore.
        set_running(true);
    }

    // ── try_exec_sh ────────────────────────────────────────────────────────────
    // In a unit-test environment the ramdisk is empty, so try_exec_sh must
    // return Err (not panic).

    #[test]
    fn test_shell_mod_try_exec_sh_returns_err_when_no_ramdisk() {
        // Ramdisk is empty in test context → must return Err.
        let result = try_exec_sh();
        assert!(result.is_err());
    }

    #[test]
    fn test_shell_mod_try_exec_sh_error_message_nonempty() {
        let err = try_exec_sh().unwrap_err();
        assert!(!err.is_empty());
    }

    // ── redraw_input ───────────────────────────────────────────────────────────

    #[test]
    fn test_shell_mod_redraw_input_empty_buf() {
        let buf = [0u8; MAX_INPUT_LEN];
        // Must not panic on empty buffer.
        redraw_input(&buf, 0, 0);
    }

    #[test]
    fn test_shell_mod_redraw_input_cursor_at_end() {
        let mut buf = [0u8; MAX_INPUT_LEN];
        buf[0] = b'a';
        buf[1] = b'b';
        redraw_input(&buf, 2, 2);
    }

    #[test]
    fn test_shell_mod_redraw_input_cursor_in_middle() {
        let mut buf = [0u8; MAX_INPUT_LEN];
        buf[0] = b'a';
        buf[1] = b'b';
        buf[2] = b'c';
        redraw_input(&buf, 3, 1);
    }

    // ── get_current_dir_str ────────────────────────────────────────────────────

    #[test]
    fn test_shell_mod_get_current_dir_str_is_root() {
        assert_eq!(get_current_dir_str(), "/");
    }
}
