// AIOS Shell Module
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Shell module root - exports parser, builtins, job_control, history submodules.

pub mod parser;
pub mod builtins;
pub mod job_control;
pub mod history;

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

use alloc::string::{String, ToString};

static CURRENT_DIR_BUFFER: spin::Mutex<[u8; 256]> = spin::Mutex::new([0u8; 256]);

pub fn get_current_dir() -> String {
    if let Some(proc) = crate::process::get_current_process() {
        let cwd = proc.get_cwd_str();
        let mut buf = CURRENT_DIR_BUFFER.lock();
        let len = cwd.as_bytes().len().min(255);
        buf[..len].copy_from_slice(cwd.as_bytes());
        buf[len] = 0;
        let result = core::str::from_utf8(&buf[..len]).unwrap_or("/").to_string();
        result
    } else {
        "/".to_string()
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