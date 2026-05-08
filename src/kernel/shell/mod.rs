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
