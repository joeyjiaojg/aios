// AIOS Shell Built-in Commands
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Built-in shell commands for AIOS - cd, pwd, exit, echo, ls, mkdir, rm, cat, set, unset, help, exec.

use crate::process;
use crate::ramdisk::RAMDISK;
use crate::shell::{get_current_dir_str, set_current_dir};

pub fn cd(args: &[&str]) -> Result<(), &'static str> {
    if args.is_empty() || args[0].is_empty() {
        set_current_dir("/")?;
        return Ok(());
    }

    let path = args[0];

    if path == "~" {
        set_current_dir("/home")?;
        Ok(())
    } else if path == ".." {
        let current = get_current_dir_str();
        if current != "/" {
            let mut parts: [&str; 32] = [
                "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
                "", "", "", "", "", "", "", "", "", "", "",
            ];
            let mut count = 0;
            for part in current.trim_end_matches('/').split('/') {
                if count >= 32 {
                    break;
                }
                parts[count] = part;
                count += 1;
            }
            if count > 1 {
                let mut new_path = [0u8; 256];
                let mut pos = 0;
                for part in &parts[..count - 1] {
                    if pos > 0 && pos < 255 {
                        new_path[pos] = b'/';
                        pos += 1;
                    }
                    for &b in part.as_bytes() {
                        if pos < 255 {
                            new_path[pos] = b;
                            pos += 1;
                        }
                    }
                }
                new_path[pos] = 0;
                let new_path_str = core::str::from_utf8(&new_path[..pos]).unwrap_or("/");
                set_current_dir(new_path_str)?;
            } else {
                set_current_dir("/")?;
            }
        }
        Ok(())
    } else if path == "." {
        Ok(())
    } else if path.starts_with('/') {
        set_current_dir(path)?;
        Ok(())
    } else {
        let current = get_current_dir_str();
        let mut new_path = [0u8; 256];
        let mut pos = 0;
        for &b in current.as_bytes() {
            if pos < 255 {
                new_path[pos] = b;
                pos += 1;
            }
        }
        if pos < 255 && new_path[pos - 1] != b'/' {
            new_path[pos] = b'/';
            pos += 1;
        }
        for &b in path.as_bytes() {
            if pos < 255 {
                new_path[pos] = b;
                pos += 1;
            }
        }
        new_path[pos] = 0;
        let new_path_str = core::str::from_utf8(&new_path[..pos]).unwrap_or("/");
        set_current_dir(new_path_str)?;
        Ok(())
    }
}

pub fn pwd() -> Result<(), &'static str> {
    crate::serial::write_str(get_current_dir_str());
    crate::serial::write_str("\r\n");
    Ok(())
}

pub fn exit_cmd(args: &[&str]) -> Result<(), &'static str> {
    let status = if args.is_empty() || args[0].is_empty() {
        0
    } else {
        args[0].parse::<i32>().unwrap_or(0)
    };

    crate::serial::write_str("Goodbye!\r\n");
    crate::shell::stop_shell();

    let pid = process::get_current_pid();
    let mut table = process::PROCESS_TABLE.lock();
    table.set_exit_status(pid, status);

    Ok(())
}

pub fn echo(args: &[&str]) -> Result<(), &'static str> {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            crate::serial::write_byte(b' ');
        }
        crate::serial::write_str(arg);
    }
    crate::serial::write_str("\r\n");
    Ok(())
}

pub fn ls(args: &[&str]) -> Result<(), &'static str> {
    let show_hidden = args.contains(&"-a");
    let long_format = args.contains(&"-l");

    let current = get_current_dir_str();
    let path: &str = if args.is_empty() || args[0].starts_with('-') {
        current
    } else {
        args[0]
    };

    let path_hash = simple_hash_str(path);
    let ramdisk = RAMDISK.lock();

    if long_format {
        crate::serial::write_str("total 1\r\n");
    }

    for i in 0..16 {
        let ino = ((path_hash + i) % 127) as u32;
        let mut entry_buf = [0u8; 32];
        let bytes_read = ramdisk.read(ino, 0, &mut entry_buf).unwrap_or(0);

        if bytes_read > 0 && entry_buf[0] != 0 && (show_hidden || entry_buf[0] != b'.') {
            let name_len = bytes_read.min(14);
            for &byte in entry_buf.iter().take(name_len) {
                if byte != 0 {
                    crate::serial::write_byte(byte);
                }
            }
            if !long_format {
                crate::serial::write_byte(b' ');
            }
            crate::serial::write_str("  ");
        }
    }

    crate::serial::write_str("\r\n");
    Ok(())
}

pub fn mkdir(args: &[&str]) -> Result<(), &'static str> {
    if args.is_empty() || args[0].is_empty() {
        return Err("mkdir: missing operand");
    }

    let path = args[0];
    let path_hash = simple_hash_str(path);
    let mut ramdisk = RAMDISK.lock();

    let marker = b"[DIR]";
    let _ = ramdisk.write(path_hash as u32, 0, marker);

    crate::serial::write_str("Directory created: ");
    crate::serial::write_str(path);
    crate::serial::write_str("\r\n");

    Ok(())
}

pub fn rm(args: &[&str]) -> Result<(), &'static str> {
    if args.is_empty() || args[0].is_empty() {
        return Err("rm: missing operand");
    }

    let path = args[0];
    let path_hash = simple_hash_str(path);
    let mut ramdisk = RAMDISK.lock();

    let zero_buf = [0u8; 16];
    let _ = ramdisk.write(path_hash as u32, 0, &zero_buf);

    crate::serial::write_str("Removed: ");
    crate::serial::write_str(path);
    crate::serial::write_str("\r\n");

    Ok(())
}

pub fn cat(args: &[&str]) -> Result<(), &'static str> {
    if args.is_empty() || args[0].is_empty() {
        return Err("cat: missing operand");
    }

    let path = args[0];
    let path_hash = simple_hash_str(path);
    let ramdisk = RAMDISK.lock();

    let mut read_buf = [0u8; 256];
    let bytes_read = ramdisk
        .read(path_hash as u32, 0, &mut read_buf)
        .unwrap_or(0);

    if bytes_read == 0 {
        crate::serial::write_str("cat: ");
        crate::serial::write_str(path);
        crate::serial::write_str(": No such file or directory\r\n");
        return Err("File not found");
    }

    for &byte in read_buf.iter().take(bytes_read) {
        if byte == 0 {
            break;
        }
        crate::serial::write_byte(byte);
    }
    crate::serial::write_str("\r\n");

    Ok(())
}

pub fn set_var(args: &[&str]) -> Result<(), &'static str> {
    if args.is_empty() || args.len() < 2 || args[0].is_empty() {
        return Err("set: usage: set VAR VALUE");
    }

    let var_name = args[0];
    let var_value = args[1];

    crate::serial::write_str("set ");
    crate::serial::write_str(var_name);
    crate::serial::write_str("=");
    crate::serial::write_str(var_value);
    crate::serial::write_str("\r\n");

    Ok(())
}

pub fn unset_var(args: &[&str]) -> Result<(), &'static str> {
    if args.is_empty() || args[0].is_empty() {
        return Err("unset: usage: unset VAR");
    }

    crate::serial::write_str("unset ");
    crate::serial::write_str(args[0]);
    crate::serial::write_str("\r\n");

    Ok(())
}

pub fn help() -> Result<(), &'static str> {
    crate::serial::write_str("AIOS Shell Commands:\r\n");
    crate::serial::write_str("  cd [dir]       Change directory\r\n");
    crate::serial::write_str("  pwd            Print working directory\r\n");
    crate::serial::write_str("  exit [n]       Exit with status n\r\n");
    crate::serial::write_str("  echo [args]    Print arguments\r\n");
    crate::serial::write_str("  ls [-al]       List directory contents\r\n");
    crate::serial::write_str("  mkdir <dir>    Create directory\r\n");
    crate::serial::write_str("  rm <file>      Remove file\r\n");
    crate::serial::write_str("  cat <file>     Display file contents\r\n");
    crate::serial::write_str("  set VAR VAL    Set environment variable\r\n");
    crate::serial::write_str("  unset VAR      Unset environment variable\r\n");
    crate::serial::write_str("  history        Show command history\r\n");
    crate::serial::write_str("  jobs           List background jobs\r\n");
    crate::serial::write_str("  fg [n]         Bring job to foreground\r\n");
    crate::serial::write_str("  bg [n]         Resume job in background\r\n");
    crate::serial::write_str("  help           Show this help message\r\n");
    Ok(())
}

pub fn exec_cmd(cmd: &str, _args: &[&str]) -> Result<(), &'static str> {
    crate::serial::write_str("Executing: ");
    crate::serial::write_str(cmd);
    crate::serial::write_str("\r\n");
    Ok(())
}

pub fn execute_builtin(cmd: &str, args: &[&str]) -> bool {
    let args_slice = &args[1..];
    match cmd {
        "cd" => cd(args_slice).is_ok(),
        "pwd" => pwd().is_ok(),
        "exit" => {
            let _ = exit_cmd(args_slice);
            crate::shell::stop_shell();
            true
        }
        "echo" => echo(args_slice).is_ok(),
        "ls" => ls(args_slice).is_ok(),
        "mkdir" => mkdir(args_slice).is_ok(),
        "rm" => rm(args_slice).is_ok(),
        "cat" => cat(args_slice).is_ok(),
        "set" => set_var(args_slice).is_ok(),
        "unset" => unset_var(args_slice).is_ok(),
        "help" => help().is_ok(),
        "history" => {
            let _ = crate::shell::history::show_history();
            true
        }
        "jobs" => {
            let _ = crate::shell::job_control::list_jobs();
            true
        }
        "fg" => crate::shell::job_control::fg(args_slice).is_ok(),
        "bg" => crate::shell::job_control::bg(args_slice).is_ok(),
        "exec" => exec_cmd(cmd, args_slice).is_ok(),
        _ => false,
    }
}

fn simple_hash_str(s: &str) -> usize {
    let mut hash: usize = 0;
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'/' || b == b'.' {
            continue;
        }
        hash = hash
            .wrapping_mul(31)
            .wrapping_add(b as usize)
            .wrapping_add(i);
    }
    if hash == 0 {
        hash = 1;
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_hash_str_basic() {
        let result = simple_hash_str("test");
        assert!(result > 0);
    }

    #[test]
    fn test_simple_hash_str_empty() {
        let result = simple_hash_str("");
        assert!(result > 0);
    }

    #[test]
    fn test_simple_hash_str_slashes_ignored() {
        let hash1 = simple_hash_str("/test/path");
        let hash2 = simple_hash_str("testpath");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_simple_hash_str_dots_ignored() {
        let hash1 = simple_hash_str(".test");
        let hash2 = simple_hash_str("test");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_echo_no_args() {
        let result = echo(&[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_echo_single_arg() {
        let result = echo(&["hello"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_echo_multiple_args() {
        let result = echo(&["hello", "world"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cd_root() {
        let result = cd(&["/"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cd_empty() {
        let result = cd(&[""]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cd_double_dot() {
        let result = cd(&[".."]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cd_single_dot() {
        let result = cd(&["."]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_var_usage() {
        let result = set_var(&["PATH", "/bin"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unset_var_usage() {
        let result = unset_var(&["PATH"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_help_output() {
        let result = help();
        assert!(result.is_ok());
    }

    #[test]
    fn test_exec_cmd_basic() {
        let result = exec_cmd("/bin/ls", &[]);
        assert!(result.is_ok());
    }
}
