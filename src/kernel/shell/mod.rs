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

pub fn run_shell() {
    set_running(true);
    crate::serial::write_str("AIOS Shell v1.0\r\n");
    crate::serial::write_str("Type 'help' for available commands.\r\n\r\n");

    let mut input_buf = [0u8; MAX_INPUT_LEN];
    let mut input_len: usize;

    loop {
        if !is_running() {
            break;
        }

        crate::serial::write_str("aios$ ");

        input_len = 0;
        loop {
            if let Some(byte) = crate::serial::read_byte() {
                match byte {
                    b'\r' | b'\n' => {
                        crate::serial::write_str("\r\n");
                        break;
                    }
                    0x7F | 0x08 => {
                        if input_len > 0 {
                            input_len -= 1;
                            crate::serial::write_str("\x08 \x08");
                        }
                    }
                    b if (0x20..0x7F).contains(&b) && input_len < MAX_INPUT_LEN - 1 => {
                        input_buf[input_len] = b;
                        input_len += 1;
                        crate::serial::write_byte(b);
                    }
                    _ => {}
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
            crate::serial::write_str("Command not found: ");
            crate::serial::write_str(cmd);
            crate::serial::write_str("\r\n");
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
