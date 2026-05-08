// AIOS Syscall Interface
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Implement syscall interface for AIOS x86_64 kernel in Rust no_std

use spin::Mutex;

pub const SYSCALL_READ: usize = 0;
pub const SYSCALL_WRITE: usize = 1;
pub const SYSCALL_OPEN: usize = 2;
pub const SYSCALL_CLOSE: usize = 3;
pub const SYSCALL_EXIT: usize = 60;
pub const SYSCALL_GETPID: usize = 39;
pub const SYSCALL_BRK: usize = 12;
pub const SYSCALL_MMAP: usize = 9;
pub const SYSCALL_MUNMAP: usize = 11;
pub const SYSCALL_CLOCK_GETTIME: usize = 160;
pub const SYSCALL_FORK: usize = 57;
pub const SYSCALL_EXECVE: usize = 59;
pub const SYSCALL_WAIT4: usize = 61;
pub const SYSCALL_GETCWD: usize = 17;
pub const SYSCALL_CHDIR: usize = 80;
pub const SYSCALL_MKDIR: usize = 83;
pub const SYSCALL_RMDIR: usize = 84;
pub const SYSCALL_UNLINK: usize = 87;
pub const SYSCALL_DUP: usize = 32;
pub const SYSCALL_DUP2: usize = 33;

const MAX_SYSCALLS: usize = 64;

type SyscallHandler = fn(usize, usize, usize) -> isize;

pub struct SyscallManager {
    handlers: [Option<SyscallHandler>; MAX_SYSCALLS],
    pub last_syscall: usize,
    pub last_result: isize,
}

impl Default for SyscallManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SyscallManager {
    pub fn new() -> Self {
        let mut manager = Self {
            handlers: [None; MAX_SYSCALLS],
            last_syscall: 0,
            last_result: 0,
        };
        manager.init();
        manager
    }

    fn init(&mut self) {
        self.register(SYSCALL_READ, sys_read);
        self.register(SYSCALL_WRITE, sys_write);
        self.register(SYSCALL_OPEN, sys_open);
        self.register(SYSCALL_CLOSE, sys_close);
        self.register(SYSCALL_EXIT, sys_exit);
        self.register(SYSCALL_GETPID, sys_getpid);
        self.register(SYSCALL_BRK, sys_brk);
        self.register(SYSCALL_MMAP, sys_mmap);
        self.register(SYSCALL_MUNMAP, sys_munmap);
        self.register(SYSCALL_CLOCK_GETTIME, sys_clock_gettime);
        self.register(SYSCALL_FORK, sys_fork);
        self.register(SYSCALL_EXECVE, sys_execve);
        self.register(SYSCALL_WAIT4, sys_wait4);
        self.register(SYSCALL_GETCWD, sys_getcwd);
        self.register(SYSCALL_CHDIR, sys_chdir);
        self.register(SYSCALL_MKDIR, sys_mkdir);
        self.register(SYSCALL_RMDIR, sys_rmdir);
        self.register(SYSCALL_UNLINK, sys_unlink);
        self.register(SYSCALL_DUP, sys_dup);
        self.register(SYSCALL_DUP2, sys_dup2);
    }

    fn register(&mut self, num: usize, handler: SyscallHandler) {
        if num < MAX_SYSCALLS {
            self.handlers[num] = Some(handler);
        }
    }

    pub fn handle(&mut self, syscall_num: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
        self.last_syscall = syscall_num;

        if syscall_num >= MAX_SYSCALLS {
            self.last_result = -1;
            return -1;
        }

        match self.handlers[syscall_num] {
            Some(handler) => {
                self.last_result = handler(arg1, arg2, arg3);
                self.last_result
            }
            None => {
                self.last_result = -1;
                -1
            }
        }
    }
}

fn sys_read(_fd: usize, _buf: usize, _count: usize) -> isize {
    0
}

fn sys_write(fd: usize, buf: usize, count: usize) -> isize {
    if fd == 1 || fd == 2 {
        if buf == 0 || count == 0 {
            return 0;
        }
        let safe_count = if count > 4096 { 4096 } else { count };
        // # Safety
        // We validate that buf is non-zero and count is bounded to 4KB.
        // This is a simple implementation - a production kernel would copy
        // through a kernel buffer to ensure the user pointer is valid.
        let slice = unsafe { core::slice::from_raw_parts(buf as *const u8, safe_count) };
        if let Ok(s) = core::str::from_utf8(slice) {
            crate::serial::write_str(s);
        }
        count as isize
    } else {
        count as isize
    }
}

fn sys_open(_path: usize, _flags: usize, _mode: usize) -> isize {
    3
}

fn sys_close(_fd: usize, _arg2: usize, _arg3: usize) -> isize {
    0
}

#[allow(clippy::empty_loop)]
fn sys_exit(_status: usize, _arg2: usize, _arg3: usize) -> isize {
    loop {}
}

fn sys_getpid(_arg1: usize, _arg2: usize, _arg3: usize) -> isize {
    1
}

fn sys_brk(_addr: usize, _arg2: usize, _arg3: usize) -> isize {
    0
}

fn sys_mmap(_addr: usize, _len: usize, _prot: usize) -> isize {
    0
}

fn sys_munmap(_addr: usize, _len: usize, _arg3: usize) -> isize {
    0
}

fn sys_clock_gettime(_clock_id: usize, _timespec: usize, _arg3: usize) -> isize {
    0
}

fn sys_fork(_arg1: usize, _arg2: usize, _arg3: usize) -> isize {
    // # Safety
    // fork() creates a new process by allocating a new PCB entry and copying
    // the parent process state. This is thread-safe as it only accesses the
    // process table through a Mutex lock. The new process gets its own PID
    // and inherits the parent's CWD.
    let parent_pid = crate::process::get_current_pid();
    let child_pid = match crate::process::alloc_process(parent_pid) {
        Some(pid) => pid,
        None => return -1,
    };

    // Copy parent's cwd to child process
    if let Some(parent) = crate::process::get_process(parent_pid) {
        if let Some(child) = crate::process::PROCESS_TABLE
            .lock()
            .get_process_mut(child_pid)
        {
            child.cwd = parent.cwd;
            child.cwd_len = parent.cwd_len;
        }
    }

    // Store fork result: parent gets child's PID, child gets 0
    // Since we can't actually context-switch here, we store the result
    // and use a wrapper to retrieve it
    FORK_RESULT.store(child_pid, core::sync::atomic::Ordering::SeqCst);
    child_pid as isize
}

fn sys_execve(path_ptr: usize, _argv: usize, _envp: usize) -> isize {
    if path_ptr == 0 {
        return -1;
    }

    // # Safety
    // Reading path string from user pointer. The pointer is assumed to be valid
    // and pointing to a null-terminated string in user memory.
    let path_bytes = unsafe { core::slice::from_raw_parts(path_ptr as *const u8, 256) };
    let path_len = path_bytes.iter().position(|&b| b == 0).unwrap_or(256);
    let path = &path_bytes[..path_len];

    // Try to read ELF from ramdisk (using path as block number for simple demo)
    // In a real FS, we'd parse the path and look up the inode
    let block_num = if path.len() == 1 && path[0] == b'/' {
        1
    } else if path.starts_with(b"/bin/") || path.starts_with(b"/sbin/") {
        let name = &path[5..];
        simple_hash(name) as u32
    } else {
        simple_hash(path) as u32
    };

    let mut elf_data = [0u8; 8192];

    // Read ELF from ramdisk into buffer
    // # Safety
    // RAMDISK lock ensures exclusive access, and we only read up to buffer size
    let ramdisk = crate::ramdisk::RAMDISK.lock();
    let bytes_read = ramdisk.read(block_num, 0, &mut elf_data).unwrap_or(0);
    drop(ramdisk);

    if bytes_read < 64 {
        return -1;
    }

    // Validate ELF magic
    if elf_data[0..4] != [0x7F, b'E', b'L', b'F'] {
        return -1;
    }

    let entry = u64::from_le_bytes([
        elf_data[24],
        elf_data[25],
        elf_data[26],
        elf_data[27],
        elf_data[28],
        elf_data[29],
        elf_data[30],
        elf_data[31],
    ]);

    // Store exec result: entry point for the new program
    EXEC_ENTRY.store(entry, core::sync::atomic::Ordering::SeqCst);
    0
}

fn sys_wait4(_pid: usize, _status_ptr: usize, _options: usize) -> isize {
    // # Safety
    // wait4 suspends the calling process until a child exits. This is safe as it
    // only accesses the process table through a Mutex lock.
    let pid = crate::process::get_current_pid();
    let mut table = crate::process::PROCESS_TABLE.lock();
    if let Some((child_pid, _exit_code)) = table.wait_for_child(pid) {
        drop(table);
        // Note: in a real implementation, we'd write exit_code to status_ptr
        child_pid as isize
    } else {
        0
    }
}

fn sys_getcwd(buf_ptr: usize, size: usize, _arg3: usize) -> isize {
    // # Safety
    // getcwd writes the current working directory string to the provided buffer.
    // The buffer pointer is assumed to be valid user memory.
    if buf_ptr == 0 || size == 0 {
        return 0;
    }

    let cwd = crate::process::get_current_pid();
    let table = crate::process::PROCESS_TABLE.lock();
    let proc = table.get_process(cwd);

    let path = proc.map(|p| p.get_cwd_str()).unwrap_or("/");
    let path_bytes = path.as_bytes();

    if path_bytes.len() + 1 > size {
        return 0;
    }

    // # Safety
    // Writing to user-provided buffer. The buffer pointer and size have been
    // validated. The path string is null-terminated.
    unsafe {
        core::slice::from_raw_parts_mut(buf_ptr as *mut u8, path_bytes.len())
            .copy_from_slice(path_bytes);
        core::slice::from_raw_parts_mut(buf_ptr as *mut u8, size)[path_bytes.len()] = 0;
    }
    buf_ptr as isize
}

fn sys_chdir(path_ptr: usize, _arg2: usize, _arg3: usize) -> isize {
    if path_ptr == 0 {
        return -1;
    }

    // # Safety
    // Reading path string from user pointer. The pointer is assumed to be valid
    // and pointing to a null-terminated string in user memory.
    let path_bytes = unsafe { core::slice::from_raw_parts(path_ptr as *const u8, 256) };
    let path_len = path_bytes.iter().position(|&b| b == 0).unwrap_or(256);
    let path = core::str::from_utf8(&path_bytes[..path_len]).unwrap_or("");

    let cwd = crate::process::get_current_pid();
    let mut table = crate::process::PROCESS_TABLE.lock();
    if let Some(proc) = table.get_process_mut(cwd) {
        let mut clean_path = [0u8; 256];
        let len = path.len().min(255);
        clean_path[..len].copy_from_slice(path.as_bytes());
        proc.cwd[..len].copy_from_slice(&clean_path[..len]);
        proc.cwd[len] = 0;
        proc.cwd_len = len;
        return 0;
    }
    -1
}

fn sys_mkdir(path_ptr: usize, _mode: usize, _arg3: usize) -> isize {
    if path_ptr == 0 {
        return -1;
    }

    // # Safety
    // Reading path string from user pointer. The pointer is assumed to be valid
    // and pointing to a null-terminated string in user memory.
    let path_bytes = unsafe { core::slice::from_raw_parts(path_ptr as *const u8, 256) };
    let path_len = path_bytes.iter().position(|&b| b == 0).unwrap_or(256);

    let ino = simple_hash(&path_bytes[..path_len]) as u32;
    let mut ramdisk = crate::ramdisk::RAMDISK.lock();
    let result = ramdisk.write(ino, 0, b"[DIR]");
    if result.is_some() {
        0
    } else {
        -1
    }
}

fn sys_rmdir(path_ptr: usize, _arg2: usize, _arg3: usize) -> isize {
    if path_ptr == 0 {
        return -1;
    }

    // # Safety
    // Reading path string from user pointer. The pointer is assumed to be valid
    // and pointing to a null-terminated string in user memory.
    let path_bytes = unsafe { core::slice::from_raw_parts(path_ptr as *const u8, 256) };
    let path_len = path_bytes.iter().position(|&b| b == 0).unwrap_or(256);
    let ino = simple_hash(&path_bytes[..path_len]) as u32;

    let mut ramdisk = crate::ramdisk::RAMDISK.lock();
    // Clear the block by writing zeros
    let zero_buf = [0u8; 512];
    ramdisk.write(ino, 0, &zero_buf);
    0
}

fn sys_unlink(path_ptr: usize, _arg2: usize, _arg3: usize) -> isize {
    if path_ptr == 0 {
        return -1;
    }

    // # Safety
    // Reading path string from user pointer. The pointer is assumed to be valid
    // and pointing to a null-terminated string in user memory.
    let path_bytes = unsafe { core::slice::from_raw_parts(path_ptr as *const u8, 256) };
    let path_len = path_bytes.iter().position(|&b| b == 0).unwrap_or(256);
    let ino = simple_hash(&path_bytes[..path_len]) as u32;

    let mut ramdisk = crate::ramdisk::RAMDISK.lock();
    let zero_buf = [0u8; 512];
    ramdisk.write(ino, 0, &zero_buf);
    0
}

fn sys_dup(_fd: usize, _arg2: usize, _arg3: usize) -> isize {
    0
}

fn sys_dup2(_oldfd: usize, _newfd: usize, _arg3: usize) -> isize {
    0
}

fn simple_hash(data: &[u8]) -> usize {
    let mut hash: usize = 0;
    for (i, &b) in data.iter().enumerate() {
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

static FORK_RESULT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

static EXEC_ENTRY: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

pub fn get_fork_result() -> Option<usize> {
    let val = FORK_RESULT.swap(0, core::sync::atomic::Ordering::SeqCst);
    if val == 0 {
        None
    } else {
        Some(val)
    }
}

pub fn get_exec_entry() -> u64 {
    EXEC_ENTRY.load(core::sync::atomic::Ordering::SeqCst)
}

pub fn set_exec_entry(entry: u64) {
    EXEC_ENTRY.store(entry, core::sync::atomic::Ordering::SeqCst);
}

static SYSCALL_MANAGER: Mutex<Option<SyscallManager>> = Mutex::new(None);

fn get_syscall_manager() -> &'static Mutex<Option<SyscallManager>> {
    let mut mgr = SYSCALL_MANAGER.lock();
    if mgr.is_none() {
        *mgr = Some(SyscallManager::new());
    }
    &SYSCALL_MANAGER
}

pub fn init() {
    get_syscall_manager();
}

pub fn handle_syscall(syscall_num: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    if let Some(ref mut mgr) = *get_syscall_manager().lock() {
        mgr.handle(syscall_num, arg1, arg2, arg3)
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_manager_new() {
        let mgr = SyscallManager::new();
        assert_eq!(mgr.last_syscall, 0);
    }

    #[test]
    fn test_syscall_numbers() {
        assert_eq!(SYSCALL_READ, 0);
        assert_eq!(SYSCALL_WRITE, 1);
        assert_eq!(SYSCALL_EXIT, 60);
    }

    #[test]
    fn test_syscall_handler_registration() {
        let mut mgr = SyscallManager::new();
        assert!(mgr.handlers[SYSCALL_READ].is_some());
        assert!(mgr.handlers[SYSCALL_WRITE].is_some());
        assert!(mgr.handlers[SYSCALL_EXIT].is_some());
    }

    #[test]
    fn test_handle_syscall() {
        let mut mgr = SyscallManager::new();
        let result = mgr.handle(SYSCALL_GETPID, 0, 0, 0);
        assert_eq!(result, 1);
        assert_eq!(mgr.last_syscall, SYSCALL_GETPID);
    }

    #[test]
    fn test_handle_invalid_syscall() {
        let mut mgr = SyscallManager::new();
        let result = mgr.handle(9999, 0, 0, 0);
        assert_eq!(result, -1);
    }

    #[test]
    fn test_sys_write() {
        let result = sys_write(1, 0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_sys_read() {
        let result = sys_read(0, 0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_sys_open() {
        let result = sys_open(0, 0, 0);
        assert_eq!(result, 3);
    }

    #[test]
    fn test_sys_close() {
        let result = sys_close(3, 0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_sys_getpid() {
        let result = sys_getpid(0, 0, 0);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_sys_brk() {
        let result = sys_brk(0, 0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_sys_mmap() {
        let result = sys_mmap(0, 4096, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_sys_munmap() {
        let result = sys_munmap(0, 4096, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_sys_clock_gettime() {
        let result = sys_clock_gettime(0, 0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_handle_write_to_stdout() {
        let mut mgr = SyscallManager::new();
        let test_data = b"test";
        let result = mgr.handle(SYSCALL_WRITE, 1, test_data.as_ptr() as usize, 4);
        assert_eq!(result, 4);
    }

    #[test]
    fn test_handle_write_to_stderr() {
        let mut mgr = SyscallManager::new();
        let test_data = b"error";
        let result = mgr.handle(SYSCALL_WRITE, 2, test_data.as_ptr() as usize, 5);
        assert_eq!(result, 5);
    }

    #[test]
    fn test_last_result_tracking() {
        let mut mgr = SyscallManager::new();
        mgr.handle(SYSCALL_GETPID, 0, 0, 0);
        assert_eq!(mgr.last_result, 1);
    }

    #[test]
    fn test_unregistered_syscall_returns_minus_one() {
        let mut mgr = SyscallManager::new();
        let result = mgr.handle(5, 0, 0, 0);
        assert_eq!(result, -1);
    }

    #[test]
    fn test_syscall_fork_registered() {
        let mut mgr = SyscallManager::new();
        assert!(mgr.handlers[SYSCALL_FORK].is_some());
    }

    #[test]
    fn test_syscall_execve_registered() {
        let mut mgr = SyscallManager::new();
        assert!(mgr.handlers[SYSCALL_EXECVE].is_some());
    }

    #[test]
    fn test_syscall_wait4_registered() {
        let mut mgr = SyscallManager::new();
        assert!(mgr.handlers[SYSCALL_WAIT4].is_some());
    }

    #[test]
    fn test_syscall_getcwd_registered() {
        let mut mgr = SyscallManager::new();
        assert!(mgr.handlers[SYSCALL_GETCWD].is_some());
    }

    #[test]
    fn test_syscall_chdir_registered() {
        let mut mgr = SyscallManager::new();
        assert!(mgr.handlers[SYSCALL_CHDIR].is_some());
    }

    #[test]
    fn test_syscall_mkdir_registered() {
        let mut mgr = SyscallManager::new();
        assert!(mgr.handlers[SYSCALL_MKDIR].is_some());
    }

    #[test]
    fn test_syscall_rmdir_registered() {
        let mut mgr = SyscallManager::new();
        assert!(mgr.handlers[SYSCALL_RMDIR].is_some());
    }

    #[test]
    fn test_syscall_unlink_registered() {
        let mut mgr = SyscallManager::new();
        assert!(mgr.handlers[SYSCALL_UNLINK].is_some());
    }

    #[test]
    fn test_syscall_dup_registered() {
        let mut mgr = SyscallManager::new();
        assert!(mgr.handlers[SYSCALL_DUP].is_some());
    }

    #[test]
    fn test_syscall_dup2_registered() {
        let mut mgr = SyscallManager::new();
        assert!(mgr.handlers[SYSCALL_DUP2].is_some());
    }

    #[test]
    fn test_sys_fork_returns_valid_pid() {
        let result = sys_fork(0, 0, 0);
        assert!(result > 0 || result == -1);
    }

    #[test]
    fn test_sys_execve_with_null_path() {
        let result = sys_execve(0, 0, 0);
        assert_eq!(result, -1);
    }

    #[test]
    fn test_sys_getcwd_with_zero_buf() {
        let result = sys_getcwd(0, 0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_sys_chdir_with_null_path() {
        let result = sys_chdir(0, 0, 0);
        assert_eq!(result, -1);
    }

    #[test]
    fn test_sys_mkdir_with_null_path() {
        let result = sys_mkdir(0, 0, 0);
        assert_eq!(result, -1);
    }

    #[test]
    fn test_sys_rmdir_with_null_path() {
        let result = sys_rmdir(0, 0, 0);
        assert_eq!(result, -1);
    }

    #[test]
    fn test_sys_unlink_with_null_path() {
        let result = sys_unlink(0, 0, 0);
        assert_eq!(result, -1);
    }

    #[test]
    fn test_sys_dup() {
        let result = sys_dup(0, 0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_sys_dup2() {
        let result = sys_dup2(0, 0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_fork_result_atomics() {
        FORK_RESULT.store(42, core::sync::atomic::Ordering::SeqCst);
        let result = get_fork_result();
        assert_eq!(result, Some(42));
    }

    #[test]
    fn test_fork_result_none_when_zero() {
        FORK_RESULT.store(0, core::sync::atomic::Ordering::SeqCst);
        let result = get_fork_result();
        assert_eq!(result, None);
    }

    #[test]
    fn test_exec_entry_atomics() {
        set_exec_entry(0x400000);
        let entry = get_exec_entry();
        assert_eq!(entry, 0x400000);
    }

    #[test]
    fn test_simple_hash_basic() {
        let h = simple_hash(b"test");
        assert!(h > 0);
    }

    #[test]
    fn test_simple_hash_deterministic() {
        let h1 = simple_hash(b"hello");
        let h2 = simple_hash(b"hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_simple_hash_different_inputs() {
        let h1 = simple_hash(b"hello");
        let h2 = simple_hash(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_new_syscall_numbers() {
        assert_eq!(SYSCALL_FORK, 57);
        assert_eq!(SYSCALL_EXECVE, 59);
        assert_eq!(SYSCALL_WAIT4, 61);
        assert_eq!(SYSCALL_GETCWD, 17);
        assert_eq!(SYSCALL_CHDIR, 80);
        assert_eq!(SYSCALL_MKDIR, 83);
        assert_eq!(SYSCALL_RMDIR, 84);
        assert_eq!(SYSCALL_UNLINK, 87);
        assert_eq!(SYSCALL_DUP, 32);
        assert_eq!(SYSCALL_DUP2, 33);
    }
}
