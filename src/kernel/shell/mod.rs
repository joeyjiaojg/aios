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

/// Wrapper function for jumping from syscall trampoline after process exit.
/// This function is called via jmp from the syscall trampoline, so it must
/// not return (it's an infinite loop).
#[no_mangle]
pub extern "C" fn shell_prompt_loop_entry() -> ! {
    loop {
        shell_prompt_loop();
        // If shell_prompt_loop returns, we've exited the shell
        // Just halt the CPU
        // # Safety
        // HLT in the idle loop is safe; we've exited the shell loop.
        unsafe { core::arch::asm!("hlt") }
    }
}

pub fn run_shell() {
    set_running(true);
    crate::serial::write_str("AIOS Shell v1.0\r\n");
    crate::serial::write_str("Type 'help' for available commands.\r\n\r\n");
    shell_prompt_loop();
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
        loop {
            match crate::serial::read_byte() {
                Some(b'\r') | Some(b'\n') => {
                    crate::serial::write_str("\r\n");
                    break;
                }
                Some(0x7F) | Some(0x08) => {
                    if input_len > 0 {
                        input_len -= 1;
                        crate::serial::write_str("\x08 \x08");
                    }
                }
                Some(b) if (0x20..0x7F).contains(&b) && input_len < MAX_INPUT_LEN - 1 => {
                    input_buf[input_len] = b;
                    input_len += 1;
                    crate::serial::write_byte(b);
                }
                Some(_) => {}
                None => {
                    // No byte ready - continue polling
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
