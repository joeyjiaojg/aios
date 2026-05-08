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
        // SAFETY: We validate that buf is non-zero and count is bounded to 4KB.
        // This is a simple implementation - a production kernel would copy
        // through a kernel buffer to ensure the user pointer is valid.
        #[allow(clippy::undocumented_unsafe_blocks)]
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
}
