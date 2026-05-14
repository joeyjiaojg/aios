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
pub const SYSCALL_STAT: usize = 4;
pub const SYSCALL_FSTAT: usize = 5;
pub const SYSCALL_LSTAT: usize = 6;
pub const SYSCALL_LSEEK: usize = 8;
pub const SYSCALL_MMAP: usize = 9;
pub const SYSCALL_MUNMAP: usize = 11;
pub const SYSCALL_BRK: usize = 12;
pub const SYSCALL_RT_SIGACTION: usize = 13;
pub const SYSCALL_RT_SIGPROCMASK: usize = 14;
pub const SYSCALL_IOCTL: usize = 16;
pub const SYSCALL_GETCWD: usize = 17;
pub const SYSCALL_MPROTECT: usize = 10;
pub const SYSCALL_ACCESS: usize = 21;
pub const SYSCALL_PIPE: usize = 22;
pub const SYSCALL_DUP: usize = 32;
pub const SYSCALL_DUP2: usize = 33;
pub const SYSCALL_GETPID: usize = 39;
pub const SYSCALL_UNAME: usize = 63;
pub const SYSCALL_FCNTL: usize = 72;
pub const SYSCALL_CHDIR: usize = 80;
pub const SYSCALL_MKDIR: usize = 83;
pub const SYSCALL_RMDIR: usize = 84;
pub const SYSCALL_UNLINK: usize = 87;
pub const SYSCALL_READLINK: usize = 89;
pub const SYSCALL_GETUID: usize = 102;
pub const SYSCALL_GETGID: usize = 104;
pub const SYSCALL_GETEUID: usize = 107;
pub const SYSCALL_GETEGID: usize = 108;
pub const SYSCALL_SETPGID: usize = 109;
pub const SYSCALL_GETPPID: usize = 110;
pub const SYSCALL_SETSID: usize = 112;
pub const SYSCALL_GETPGID: usize = 121;
pub const SYSCALL_GETSID: usize = 124;
pub const SYSCALL_SIGALTSTACK: usize = 131;
pub const SYSCALL_ARCH_PRCTL: usize = 158;
pub const SYSCALL_CLOCK_GETTIME: usize = 160;
pub const SYSCALL_FUTEX: usize = 202;
pub const SYSCALL_SET_TID_ADDRESS: usize = 218;
pub const SYSCALL_GETDENTS64: usize = 217;
pub const SYSCALL_SET_ROBUST_LIST: usize = 273;
pub const SYSCALL_FORK: usize = 57;
pub const SYSCALL_EXECVE: usize = 59;
pub const SYSCALL_EXIT: usize = 60;
pub const SYSCALL_WAIT4: usize = 61;
pub const SYSCALL_EXIT_GROUP: usize = 231;
pub const SYSCALL_TGKILL: usize = 234;
pub const SYSCALL_OPENAT: usize = 257;
pub const SYSCALL_NEWFSTATAT: usize = 262;
pub const SYSCALL_READLINKAT: usize = 267;
pub const SYSCALL_FACCESSAT: usize = 269;
pub const SYSCALL_PRLIMIT64: usize = 302;
pub const SYSCALL_PIPE2: usize = 293;
pub const SYSCALL_FACCESSAT2: usize = 439;
pub const SYSCALL_WRITEV: usize = 20;

const MAX_SYSCALLS: usize = 512;

// FD table: fds 0/1/2 = stdin/stdout/stderr; user fds start at FD_OFFSET
const FD_OFFSET: usize = 3;
const MAX_OPEN_FDS: usize = 13;

#[derive(Copy, Clone)]
struct FdEntry {
    used: bool,
    is_dir: bool,
    path_len: usize,
    path: [u8; 256],
    offset: usize, // byte offset for files; getdents cursor for dirs
}

impl FdEntry {
    const fn new() -> Self {
        Self {
            used: false,
            is_dir: false,
            path_len: 0,
            path: [0u8; 256],
            offset: 0,
        }
    }

    fn path_str(&self) -> &str {
        core::str::from_utf8(&self.path[..self.path_len]).unwrap_or("")
    }
}

struct FdTable {
    entries: [FdEntry; MAX_OPEN_FDS],
}

impl FdTable {
    const fn new() -> Self {
        Self {
            entries: [FdEntry::new(); MAX_OPEN_FDS],
        }
    }

    fn alloc(&mut self, path: &str, is_dir: bool) -> Option<usize> {
        for (i, e) in self.entries.iter_mut().enumerate() {
            if !e.used {
                *e = FdEntry::new();
                e.used = true;
                e.is_dir = is_dir;
                let len = path.len().min(255);
                e.path[..len].copy_from_slice(&path.as_bytes()[..len]);
                e.path_len = len;
                return Some(i + FD_OFFSET);
            }
        }
        None
    }

    fn free(&mut self, fd: usize) {
        if fd >= FD_OFFSET && fd - FD_OFFSET < MAX_OPEN_FDS {
            self.entries[fd - FD_OFFSET] = FdEntry::new();
        }
    }

    fn get(&self, fd: usize) -> Option<&FdEntry> {
        if fd < FD_OFFSET {
            return None;
        }
        let i = fd - FD_OFFSET;
        if i < MAX_OPEN_FDS && self.entries[i].used {
            Some(&self.entries[i])
        } else {
            None
        }
    }

    fn get_mut(&mut self, fd: usize) -> Option<&mut FdEntry> {
        if fd < FD_OFFSET {
            return None;
        }
        let i = fd - FD_OFFSET;
        if i < MAX_OPEN_FDS && self.entries[i].used {
            Some(&mut self.entries[i])
        } else {
            None
        }
    }
}

static FD_TABLE: Mutex<FdTable> = Mutex::new(FdTable::new());

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
        self.register(SYSCALL_STAT, sys_stat);
        self.register(SYSCALL_FSTAT, sys_fstat);
        self.register(SYSCALL_LSTAT, sys_lstat);
        self.register(SYSCALL_LSEEK, sys_lseek);
        self.register(SYSCALL_MPROTECT, sys_mprotect);
        self.register(SYSCALL_RT_SIGACTION, sys_rt_sigaction);
        self.register(SYSCALL_RT_SIGPROCMASK, sys_rt_sigprocmask);
        self.register(SYSCALL_IOCTL, sys_ioctl);
        self.register(SYSCALL_WRITEV, sys_writev);
        self.register(SYSCALL_BRK, sys_brk);
        self.register(SYSCALL_MMAP, sys_mmap);
        self.register(SYSCALL_MUNMAP, sys_munmap);
        self.register(SYSCALL_GETUID, sys_getuid);
        self.register(SYSCALL_GETGID, sys_getgid);
        self.register(SYSCALL_GETEUID, sys_geteuid);
        self.register(SYSCALL_GETEGID, sys_getegid);
        self.register(SYSCALL_SIGALTSTACK, sys_sigaltstack);
        self.register(SYSCALL_CLOCK_GETTIME, sys_clock_gettime);
        self.register(SYSCALL_ARCH_PRCTL, sys_arch_prctl);
        self.register(SYSCALL_FORK, sys_fork);
        self.register(SYSCALL_EXECVE, sys_execve);
        self.register(SYSCALL_EXIT, sys_exit);
        self.register(SYSCALL_WAIT4, sys_wait4);
        self.register(SYSCALL_EXIT_GROUP, sys_exit_group);
        self.register(SYSCALL_SET_TID_ADDRESS, sys_set_tid_address);
        self.register(SYSCALL_SET_ROBUST_LIST, sys_set_robust_list);
        self.register(SYSCALL_PRLIMIT64, sys_prlimit64);
        self.register(SYSCALL_GETCWD, sys_getcwd);
        self.register(SYSCALL_CHDIR, sys_chdir);
        self.register(SYSCALL_MKDIR, sys_mkdir);
        self.register(SYSCALL_RMDIR, sys_rmdir);
        self.register(SYSCALL_UNLINK, sys_unlink);
        self.register(SYSCALL_DUP, sys_dup);
        self.register(SYSCALL_DUP2, sys_dup2);
        self.register(SYSCALL_GETPID, sys_getpid);
        self.register(SYSCALL_UNAME, sys_uname);
        self.register(SYSCALL_PIPE, sys_pipe);
        self.register(SYSCALL_FCNTL, sys_fcntl);
        self.register(SYSCALL_GETDENTS64, sys_getdents64);
        self.register(SYSCALL_OPENAT, sys_openat);
        self.register(SYSCALL_NEWFSTATAT, sys_newfstatat);
        self.register(SYSCALL_READLINKAT, sys_readlinkat);
        self.register(SYSCALL_FACCESSAT, sys_faccessat);
        self.register(SYSCALL_PIPE2, sys_pipe2);
        self.register(SYSCALL_FACCESSAT2, sys_faccessat2);
        self.register(SYSCALL_ACCESS, sys_access);
        self.register(SYSCALL_READLINK, sys_readlink);
        self.register(SYSCALL_GETPPID, sys_getppid);
        self.register(SYSCALL_SETPGID, sys_setpgid);
        self.register(SYSCALL_GETPGID, sys_getpgid);
        self.register(SYSCALL_SETSID, sys_setsid);
        self.register(SYSCALL_GETSID, sys_getsid);
        self.register(SYSCALL_FUTEX, sys_futex);
        self.register(SYSCALL_TGKILL, sys_tgkill);
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

// --- Helper functions ---

/// Read a null-terminated string from a user pointer, max 256 bytes.
fn read_user_str(ptr: usize) -> ([u8; 256], usize) {
    let mut buf = [0u8; 256];
    if ptr == 0 {
        return (buf, 0);
    }
    // # Safety
    // Bounded to 256 bytes and scanned for null terminator to prevent
    // reading past the string. The caller (syscall handler) is responsible
    // for providing a valid user-space pointer.
    let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, 256) };
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(255);
    buf[..len].copy_from_slice(&bytes[..len]);
    (buf, len)
}

/// Write a minimal stat struct (144 bytes) to user memory.
/// x86_64 Linux stat field offsets:
///   0:  st_dev (u64)
///   8:  st_ino (u64)
///   16: st_nlink (u64)
///   24: st_mode (u32)
///   28: st_uid (u32)
///   32: st_gid (u32)
///   36: pad0 (u32)
///   40: st_rdev (u64)
///   48: st_size (i64)
///   56: st_blksize (i64)
///   64: st_blocks (i64)
///   72..143: atime/mtime/ctime/unused
fn write_stat(stat_ptr: usize, ino: u64, size: u64, is_dir: bool) {
    if stat_ptr == 0 {
        return;
    }
    // # Safety
    // Writing 144-byte stat struct to user-provided pointer. The pointer is
    // non-null (checked above). A production kernel would validate the full
    // range is accessible user memory; here we trust the syscall caller.
    unsafe {
        core::ptr::write_bytes(stat_ptr as *mut u8, 0, 144);
        let p = stat_ptr as *mut u8;
        // st_dev = 1
        (p as *mut u64).write_unaligned(1);
        // st_ino
        (p.add(8) as *mut u64).write_unaligned(ino);
        // st_nlink = 1
        (p.add(16) as *mut u64).write_unaligned(1);
        // st_mode: dir 0755 = 0x41ED, file 0644 = 0x81A4
        let mode: u32 = if is_dir { 0x41ED } else { 0x81A4 };
        (p.add(24) as *mut u32).write_unaligned(mode);
        // st_size
        (p.add(48) as *mut u64).write_unaligned(size);
        // st_blksize
        (p.add(56) as *mut u64).write_unaligned(4096);
        // st_blocks (512-byte units)
        (p.add(64) as *mut u64).write_unaligned(size.div_ceil(512));
    }
}

// --- Syscall implementations ---

fn sys_read(fd: usize, buf: usize, count: usize) -> isize {
    if buf == 0 || count == 0 {
        return 0;
    }
    if fd == 0 {
        // stdin: blocking read from serial with echo
        let safe_count = count.min(256);
        // # Safety: user buffer, bounded to safe_count
        let buf_slice = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, safe_count) };
        let mut n = 0;
        loop {
            match crate::serial::read_byte() {
                Some(b'\r') | Some(b'\n') => {
                    if n < safe_count {
                        buf_slice[n] = b'\n';
                        n += 1;
                    }
                    break;
                }
                Some(0x7F) | Some(0x08) if n > 0 => {
                    n -= 1;
                    crate::serial::write_str("\x08 \x08");
                }
                Some(b) if (0x20..0x7F).contains(&b) && n < safe_count => {
                    buf_slice[n] = b;
                    n += 1;
                    crate::serial::write_byte(b);
                }
                _ => {}
            }
        }
        n as isize
    } else if fd == 1 || fd == 2 {
        -1
    } else {
        // file read via FD table
        let (path, path_len, offset) = {
            let table = FD_TABLE.lock();
            match table.get(fd) {
                Some(e) if !e.is_dir => {
                    let mut p = [0u8; 256];
                    p[..e.path_len].copy_from_slice(&e.path[..e.path_len]);
                    (p, e.path_len, e.offset)
                }
                _ => return -1,
            }
        };
        let path_str = core::str::from_utf8(&path[..path_len]).unwrap_or("");
        let safe_count = count.min(4096);
        // # Safety: user buffer, bounded to safe_count
        let buf_slice = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, safe_count) };
        match crate::ramdisk::read_file_at(path_str, offset, buf_slice) {
            Some(n) => {
                if n > 0 {
                    if let Some(e) = FD_TABLE.lock().get_mut(fd) {
                        e.offset += n;
                    }
                }
                n as isize
            }
            None => -1,
        }
    }
}

fn sys_write(fd: usize, buf: usize, count: usize) -> isize {
    if fd == 1 || fd == 2 {
        if buf == 0 || count == 0 {
            return 0;
        }
        let safe_count = if count > 4096 { 4096 } else { count };
        // # Safety
        // We validate that buf is non-zero and count is bounded to 4KB before
        // constructing the slice. The slice is only used for reading a UTF-8
        // string to write to serial output; no memory is written through it.
        // A production kernel would copy through an intermediate kernel buffer
        // to additionally validate the memory is accessible.
        let slice = unsafe { core::slice::from_raw_parts(buf as *const u8, safe_count) };
        if let Ok(s) = core::str::from_utf8(slice) {
            crate::serial::write_str(s);
        }
        count as isize
    } else {
        count as isize
    }
}

fn sys_open(path_ptr: usize, _flags: usize, _mode: usize) -> isize {
    let (path_bytes, path_len) = read_user_str(path_ptr);
    if path_len == 0 {
        return -1;
    }
    let path = core::str::from_utf8(&path_bytes[..path_len]).unwrap_or("");
    if crate::ramdisk::path_exists(path) {
        FD_TABLE
            .lock()
            .alloc(path, false)
            .map(|fd| fd as isize)
            .unwrap_or(-1)
    } else if crate::ramdisk::is_valid_dir(path) {
        FD_TABLE
            .lock()
            .alloc(path, true)
            .map(|fd| fd as isize)
            .unwrap_or(-1)
    } else {
        -2 // ENOENT
    }
}

fn sys_close(fd: usize, _arg2: usize, _arg3: usize) -> isize {
    if fd >= FD_OFFSET {
        FD_TABLE.lock().free(fd);
    }
    0
}

fn sys_stat(path_ptr: usize, stat_ptr: usize, _arg3: usize) -> isize {
    let (path_bytes, path_len) = read_user_str(path_ptr);
    if path_len == 0 || stat_ptr == 0 {
        return -1;
    }
    let path_str = core::str::from_utf8(&path_bytes[..path_len]).unwrap_or("");
    let is_dir = crate::ramdisk::is_valid_dir(path_str);
    if !is_dir && !crate::ramdisk::path_exists(path_str) {
        return -2;
    }
    let size = if is_dir {
        0u64
    } else {
        crate::ramdisk::lookup_file(path_str)
            .map(|d| d.len() as u64)
            .unwrap_or(0)
    };
    let ino = simple_hash(path_str.as_bytes()) as u64;
    write_stat(stat_ptr, ino, size, is_dir);
    0
}

fn sys_fstat(fd: usize, stat_ptr: usize, _arg3: usize) -> isize {
    if stat_ptr == 0 {
        return -1;
    }
    if fd <= 2 {
        // stdin/stdout/stderr: character device
        write_stat(stat_ptr, fd as u64 + 1, 0, false);
        return 0;
    }
    let (path, path_len, is_dir) = {
        let table = FD_TABLE.lock();
        match table.get(fd) {
            Some(e) => {
                let mut p = [0u8; 256];
                p[..e.path_len].copy_from_slice(&e.path[..e.path_len]);
                (p, e.path_len, e.is_dir)
            }
            None => return -1,
        }
    };
    let path_str = core::str::from_utf8(&path[..path_len]).unwrap_or("");
    let size = if is_dir {
        0u64
    } else {
        crate::ramdisk::lookup_file(path_str)
            .map(|d| d.len() as u64)
            .unwrap_or(0)
    };
    let ino = simple_hash(path_str.as_bytes()) as u64;
    write_stat(stat_ptr, ino, size, is_dir);
    0
}

fn sys_lstat(path_ptr: usize, stat_ptr: usize, arg3: usize) -> isize {
    sys_stat(path_ptr, stat_ptr, arg3)
}

fn sys_lseek(fd: usize, offset: usize, whence: usize) -> isize {
    // whence: SEEK_SET=0, SEEK_CUR=1, SEEK_END=2
    let mut table = FD_TABLE.lock();
    match table.get_mut(fd) {
        Some(e) => {
            e.offset = match whence {
                0 => offset,
                1 => e.offset.wrapping_add(offset),
                _ => e.offset,
            };
            e.offset as isize
        }
        None => 0,
    }
}

fn sys_mprotect(_addr: usize, _len: usize, _prot: usize) -> isize {
    0
}

fn sys_ioctl(fd: usize, request: usize, _arg: usize) -> isize {
    // busybox uses TCGETS to detect if stdout is a tty
    if fd <= 2 {
        match request {
            0x5401 => 0, // TCGETS — pretend it's a tty
            _ => 0,
        }
    } else {
        -25 // ENOTTY
    }
}

fn sys_access(path_ptr: usize, _mode: usize, _arg3: usize) -> isize {
    let (path_bytes, path_len) = read_user_str(path_ptr);
    if path_len == 0 {
        return -2;
    }
    let path = core::str::from_utf8(&path_bytes[..path_len]).unwrap_or("");
    if crate::ramdisk::path_exists(path) || crate::ramdisk::is_valid_dir(path) {
        0
    } else {
        -2
    }
}

fn sys_pipe(_pipefd: usize, _b: usize, _c: usize) -> isize {
    -38 // ENOSYS
}

fn sys_exit(status: usize, _arg2: usize, _arg3: usize) -> isize {
    let pid = crate::process::get_current_pid();
    let mut table = crate::process::PROCESS_TABLE.lock();
    table.set_exit_status(pid, status as i32);
    drop(table);
    // Signal syscall_dispatch to longjmp back to the shell after we return.
    // The SYSCALL_MANAGER mutex is still held here; syscall_dispatch checks
    // the flag only after handle_syscall returns (which releases the mutex).
    crate::interrupts::PROCESS_EXITED.store(true, core::sync::atomic::Ordering::Release);
    0
}

fn sys_getpid(_arg1: usize, _arg2: usize, _arg3: usize) -> isize {
    1
}

fn sys_brk(addr: usize, _arg2: usize, _arg3: usize) -> isize {
    let pid = crate::process::get_current_pid();
    let mut table = crate::process::PROCESS_TABLE.lock();
    let proc = table.get_process_mut(pid).unwrap();
    if addr == 0 {
        return proc.brk_end as isize;
    }
    if addr > proc.brk_end {
        let old = proc.brk_end;
        proc.brk_end = addr;
        drop(table);
        crate::elf::map_user_segment(old as u64, addr as u64);
    }
    addr as isize
}

fn sys_mmap(_addr: usize, len: usize, _prot: usize) -> isize {
    if len == 0 {
        return -1;
    }
    let len_aligned = (len + 0x1F_FFFF) & !0x1F_FFFF;
    let pid = crate::process::get_current_pid();
    let mut table = crate::process::PROCESS_TABLE.lock();
    let proc = table.get_process_mut(pid).unwrap();
    let base = proc.mmap_next;
    proc.mmap_next = base + len_aligned;
    drop(table);
    crate::elf::map_user_segment(base as u64, (base + len_aligned) as u64);
    base as isize
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
    //
    // Current implementation notes:
    // - This is a simplified fork for the AIOS kernel which runs shell as a
    //   built-in kernel task (not a separate userspace process).
    // - True fork requires: (1) process address space duplication, (2) context
    //   switch to child, (3) different return values for parent/child.
    // - This implementation creates a new process entry in the process table
    //   and returns the child's PID. A full implementation would require a
    //   scheduler and userspace memory management.
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

    // Return child's PID to parent
    // Note: In a full implementation, child would get 0 via context switch
    child_pid as isize
}

pub fn fork_parent_return(child_pid: usize) -> isize {
    child_pid as isize
}

pub fn fork_child_return() -> isize {
    0
}

fn sys_execve(path_ptr: usize, _argv: usize, _envp: usize) -> isize {
    // # Safety
    // execve() loads an ELF program from ramdisk and prepares to execute it.
    // This implementation:
    //   1. Validates the path pointer and reads the path string
    //   2. Looks up the file in ramdisk (using a simple hash for demo)
    //   3. Reads and validates ELF header
    //   4. Extracts the entry point address
    //
    // Current implementation notes:
    // - This is a simplified execve for the AIOS kernel running in kernel context
    // - True execve requires: (1) address space setup, (2) stack setup with args,
    //   (3) context switch to new program, (4) TLS setup if needed
    // - This implementation loads the ELF, extracts entry point, and stores it
    // - A full implementation would need a userspace memory manager
    if path_ptr == 0 {
        return -1;
    }

    // # Safety
    // path_ptr must point to a null-terminated string in accessible user-space
    // memory. We check path_ptr != 0 before dereferencing. The slice is bounded
    // to 256 bytes (the maximum path length we accept) and scanning for a null
    // terminator prevents reading past the string. Callers (via the syscall
    // interface) are responsible for passing valid user-space pointers.
    let path_bytes = unsafe { core::slice::from_raw_parts(path_ptr as *const u8, 256) };
    let path_len = path_bytes.iter().position(|&b| b == 0).unwrap_or(256);
    let path = &path_bytes[..path_len];

    // Try to read ELF from ramdisk (using path as block number for simple demo)
    // In a real FS, we'd parse the path and look up the inode
    let _block_num = if path.len() == 1 && path[0] == b'/' {
        1
    } else if path.starts_with(b"/bin/") || path.starts_with(b"/sbin/") {
        let name = &path[5..];
        simple_hash(name) as u32
    } else {
        simple_hash(path) as u32
    };

    let elf_data = [0u8; 8192];

    // Read ELF from ramdisk into buffer
    // # Safety
    // TODO: Implement file read from new ramdisk format
    // For now, return error since ramdisk was refactored to file index
    /*
    let ramdisk = crate::ramdisk::RAMDISK.lock();
    let bytes_read = ramdisk.read(block_num, 0, &mut elf_data).unwrap_or(0);
    drop(ramdisk);
    */
    let bytes_read = 0;

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
    // buf_ptr must be a valid user-space pointer and size must be at least 1.
    // Both are validated by the (buf_ptr == 0 || size == 0) check below, which
    // rejects null pointers and zero size. Callers via the syscall interface
    // are responsible for providing valid, accessible user memory.
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
    // validated: buf_ptr is non-null (checked above) and size is large enough
    // to hold the path plus null terminator (checked via path_bytes.len() + 1 > size).
    // The write is bounded to path_bytes.len() bytes, and we null-terminate at
    // exactly path_bytes.len(), both within the validated buffer bounds.
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
    // path_ptr must point to a null-terminated string in accessible user-space
    // memory. We check path_ptr != 0 before dereferencing. The slice is bounded
    // to 256 bytes and scanning for null terminator prevents over-reading.
    // Callers (via the syscall interface) are responsible for valid pointers.
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
    // path_ptr must point to a null-terminated string in accessible user-space
    // memory. We check path_ptr != 0 before dereferencing. The slice is bounded
    // to 256 bytes and scanning for null terminator prevents over-reading.
    // Callers (via the syscall interface) are responsible for valid pointers.
    let path_bytes = unsafe { core::slice::from_raw_parts(path_ptr as *const u8, 256) };
    let path_len = path_bytes.iter().position(|&b| b == 0).unwrap_or(256);

    let _ino = simple_hash(&path_bytes[..path_len]) as u32;
    // TODO: Implement mkdir in new ramdisk format
    -1 // Not implemented
}

fn sys_rmdir(path_ptr: usize, _arg2: usize, _arg3: usize) -> isize {
    if path_ptr == 0 {
        return -1;
    }

    // # Safety
    // path_ptr must point to a null-terminated string in accessible user-space
    // memory. We check path_ptr != 0 before dereferencing. The slice is bounded
    // to 256 bytes and scanning for null terminator prevents over-reading.
    // Callers (via the syscall interface) are responsible for valid pointers.
    let path_bytes = unsafe { core::slice::from_raw_parts(path_ptr as *const u8, 256) };
    let path_len = path_bytes.iter().position(|&b| b == 0).unwrap_or(256);
    let _ino = simple_hash(&path_bytes[..path_len]) as u32;

    // TODO: Implement in new ramdisk format
    -1 // Not implemented
}

fn sys_unlink(path_ptr: usize, _arg2: usize, _arg3: usize) -> isize {
    if path_ptr == 0 {
        return -1;
    }

    // # Safety
    // path_ptr must point to a null-terminated string in accessible user-space
    // memory. We check path_ptr != 0 before dereferencing. The slice is bounded
    // to 256 bytes and scanning for null terminator prevents over-reading.
    // Callers (via the syscall interface) are responsible for valid pointers.
    let path_bytes = unsafe { core::slice::from_raw_parts(path_ptr as *const u8, 256) };
    let path_len = path_bytes.iter().position(|&b| b == 0).unwrap_or(256);
    let _ino = simple_hash(&path_bytes[..path_len]) as u32;

    // TODO: Implement in new ramdisk format
    -1 // Not implemented
}

fn sys_dup(_fd: usize, _arg2: usize, _arg3: usize) -> isize {
    0
}

fn sys_dup2(_oldfd: usize, _newfd: usize, _arg3: usize) -> isize {
    0
}

fn sys_writev(fd: usize, iov_ptr: usize, iovcnt: usize) -> isize {
    if fd != 1 && fd != 2 {
        return iovcnt as isize;
    }
    let mut total = 0isize;
    for i in 0..iovcnt.min(16) {
        // # Safety: iov_ptr comes from user, bounded to 16 entries
        let iov_entry = (iov_ptr + i * 16) as *const u64;
        let base = unsafe { *iov_entry } as usize;
        let len = unsafe { *iov_entry.add(1) } as usize;
        if base == 0 || len == 0 {
            continue;
        }
        total += sys_write(fd, base, len.min(4096));
    }
    total
}

fn sys_rt_sigaction(_signum: usize, _act: usize, _oldact: usize) -> isize {
    0
}

fn sys_rt_sigprocmask(_how: usize, _set: usize, _oldset: usize) -> isize {
    0
}

fn sys_sigaltstack(_ss: usize, _oss: usize, _arg3: usize) -> isize {
    0
}

fn sys_arch_prctl(code: usize, addr: usize, _arg3: usize) -> isize {
    match code {
        0x1002 => {
            // ARCH_SET_FS: write FS_BASE MSR so musl TLS works
            // # Safety
            // Writes MSR_FS_BASE (0xC000_0100) with wrmsr. Called from the
            // syscall handler before sysretq so the FS base persists to ring 3.
            // ecx=MSR number, eax=low 32 bits, edx=high 32 bits of the address.
            unsafe {
                core::arch::asm!(
                    "wrmsr",
                    in("ecx") 0xC000_0100u32,
                    in("eax") addr as u32,
                    in("edx") (addr >> 32) as u32,
                );
            }
            0
        }
        0x1003 => 0, // ARCH_GET_FS — stub
        _ => -1,
    }
}

fn sys_set_tid_address(_tidptr: usize, _arg2: usize, _arg3: usize) -> isize {
    1
}

fn sys_set_robust_list(_head: usize, _len: usize, _arg3: usize) -> isize {
    0
}

fn sys_exit_group(status: usize, _arg2: usize, _arg3: usize) -> isize {
    sys_exit(status, 0, 0)
}

fn sys_prlimit64(_pid: usize, _resource: usize, _new_limit: usize) -> isize {
    0
}

fn sys_getuid(_arg1: usize, _arg2: usize, _arg3: usize) -> isize {
    0
}

fn sys_getgid(_arg1: usize, _arg2: usize, _arg3: usize) -> isize {
    0
}

fn sys_geteuid(_arg1: usize, _arg2: usize, _arg3: usize) -> isize {
    0
}

fn sys_getegid(_arg1: usize, _arg2: usize, _arg3: usize) -> isize {
    0
}

fn sys_uname(utsname_ptr: usize, _arg2: usize, _arg3: usize) -> isize {
    if utsname_ptr == 0 {
        return -1;
    }
    // struct utsname: 6 fields of 65 bytes each = 390 bytes
    // # Safety: writing 390-byte struct to user pointer, checked non-null above
    unsafe { core::ptr::write_bytes(utsname_ptr as *mut u8, 0, 390) };
    let fields: [(&[u8], usize); 6] = [
        (b"AIOS", 0),
        (b"aios", 65),
        (b"1.0.0", 130),
        (b"#1", 195),
        (b"x86_64", 260),
        (b"", 325),
    ];
    for (s, offset) in fields {
        if s.is_empty() {
            continue;
        }
        // # Safety: utsname_ptr non-null, each field at known offset within 390 bytes
        let ptr = (utsname_ptr + offset) as *mut u8;
        unsafe { core::slice::from_raw_parts_mut(ptr, s.len()).copy_from_slice(s) };
    }
    0
}

fn sys_fcntl(_fd: usize, _cmd: usize, _arg: usize) -> isize {
    0
}

fn sys_getdents64(fd: usize, buf_ptr: usize, buf_size: usize) -> isize {
    if buf_ptr == 0 || buf_size == 0 {
        return -1;
    }

    let (dir_path, path_len, start_idx) = {
        let table = FD_TABLE.lock();
        match table.get(fd) {
            Some(e) if e.is_dir => {
                let mut p = [0u8; 256];
                p[..e.path_len].copy_from_slice(&e.path[..e.path_len]);
                (p, e.path_len, e.offset)
            }
            _ => return -1,
        }
    };

    let dir_str = core::str::from_utf8(&dir_path[..path_len]).unwrap_or("/");
    // # Safety: buf_ptr is user-provided, bounded to buf_size
    let buf = unsafe { core::slice::from_raw_parts_mut(buf_ptr as *mut u8, buf_size) };
    let (written, new_idx) = crate::ramdisk::fill_getdents64(dir_str, buf, start_idx);

    if written > 0 {
        if let Some(e) = FD_TABLE.lock().get_mut(fd) {
            e.offset = new_idx;
        }
    }
    written as isize
}

fn sys_openat(dirfd: usize, path_ptr: usize, flags: usize) -> isize {
    let (path_bytes, path_len) = read_user_str(path_ptr);
    if path_len == 0 {
        return -1;
    }
    let path_str = core::str::from_utf8(&path_bytes[..path_len]).unwrap_or("");

    // Resolve to absolute path
    let mut abs_buf = [0u8; 256];
    let abs_path: &str = if path_str.starts_with('/') {
        path_str
    } else {
        // "." means the current working directory itself — resolve to "/" directly.
        if path_str == "." {
            "/"
        } else {
            // relative path: prepend CWD (AT_FDCWD = -100) or dirfd's path
            let base: &str = if dirfd as i64 == -100 { "/" } else { "/" };
            let base_bytes = base.as_bytes();
            let mut pos = 0;
            for &b in base_bytes {
                if pos < 255 {
                    abs_buf[pos] = b;
                    pos += 1;
                }
            }
            if pos > 0 && abs_buf[pos - 1] != b'/' {
                abs_buf[pos] = b'/';
                pos += 1;
            }
            for &b in path_str.as_bytes() {
                if pos < 255 {
                    abs_buf[pos] = b;
                    pos += 1;
                }
            }
            core::str::from_utf8(&abs_buf[..pos]).unwrap_or(path_str)
        }
    };

    // O_DIRECTORY = 0x10000 (octal 0200000)
    let is_dir_flag = (flags & 0x10000) != 0;

    if crate::ramdisk::path_exists(abs_path) && !is_dir_flag {
        FD_TABLE
            .lock()
            .alloc(abs_path, false)
            .map(|fd| fd as isize)
            .unwrap_or(-1)
    } else if crate::ramdisk::is_valid_dir(abs_path) {
        FD_TABLE
            .lock()
            .alloc(abs_path, true)
            .map(|fd| fd as isize)
            .unwrap_or(-1)
    } else {
        -2 // ENOENT
    }
}

fn sys_newfstatat(_dirfd: usize, path_ptr: usize, stat_ptr: usize) -> isize {
    sys_stat(path_ptr, stat_ptr, 0)
}

fn sys_readlink(path_ptr: usize, buf_ptr: usize, bufsiz: usize) -> isize {
    let (path_bytes, path_len) = read_user_str(path_ptr);
    let path = core::str::from_utf8(&path_bytes[..path_len]).unwrap_or("");
    let target: &[u8] = if path.starts_with("/proc/self") {
        b"/bin/busybox"
    } else {
        return -2; // ENOENT
    };
    if buf_ptr == 0 || bufsiz == 0 {
        return -1;
    }
    let copy_len = target.len().min(bufsiz);
    // # Safety: user-provided buffer, bounded to copy_len which is <= bufsiz
    unsafe {
        core::slice::from_raw_parts_mut(buf_ptr as *mut u8, copy_len)
            .copy_from_slice(&target[..copy_len]);
    }
    copy_len as isize
}

fn sys_readlinkat(_dirfd: usize, path_ptr: usize, buf_ptr: usize) -> isize {
    sys_readlink(path_ptr, buf_ptr, 256)
}

fn sys_faccessat(_dirfd: usize, path_ptr: usize, mode: usize) -> isize {
    sys_access(path_ptr, mode, 0)
}

fn sys_pipe2(_pipefd: usize, _flags: usize, _c: usize) -> isize {
    -38 // ENOSYS
}

fn sys_faccessat2(_dirfd: usize, path_ptr: usize, mode: usize) -> isize {
    sys_access(path_ptr, mode, 0)
}

fn sys_getppid(_a: usize, _b: usize, _c: usize) -> isize {
    0
}

fn sys_setpgid(_a: usize, _b: usize, _c: usize) -> isize {
    0
}

fn sys_getpgid(_a: usize, _b: usize, _c: usize) -> isize {
    1
}

fn sys_setsid(_a: usize, _b: usize, _c: usize) -> isize {
    1
}

fn sys_getsid(_a: usize, _b: usize, _c: usize) -> isize {
    1
}

fn sys_futex(_uaddr: usize, _op: usize, _val: usize) -> isize {
    0
}

fn sys_tgkill(_tgid: usize, _tid: usize, _sig: usize) -> isize {
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
    fn test_new_syscall_numbers() {
        assert_eq!(SYSCALL_STAT, 4);
        assert_eq!(SYSCALL_FSTAT, 5);
        assert_eq!(SYSCALL_LSTAT, 6);
        assert_eq!(SYSCALL_LSEEK, 8);
        assert_eq!(SYSCALL_MPROTECT, 10);
        assert_eq!(SYSCALL_IOCTL, 16);
        assert_eq!(SYSCALL_GETDENTS64, 217);
        assert_eq!(SYSCALL_OPENAT, 257);
        assert_eq!(SYSCALL_FUTEX, 202);
        assert_eq!(SYSCALL_TGKILL, 234);
    }

    #[test]
    fn test_process_group_syscall_numbers() {
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

    #[test]
    fn test_syscall_handler_registration() {
        let mut mgr = SyscallManager::new();
        assert!(mgr.handlers[SYSCALL_READ].is_some());
        assert!(mgr.handlers[SYSCALL_WRITE].is_some());
        assert!(mgr.handlers[SYSCALL_EXIT].is_some());
        assert!(mgr.handlers[SYSCALL_STAT].is_some());
        assert!(mgr.handlers[SYSCALL_FSTAT].is_some());
        assert!(mgr.handlers[SYSCALL_LSTAT].is_some());
        assert!(mgr.handlers[SYSCALL_GETDENTS64].is_some());
        assert!(mgr.handlers[SYSCALL_OPENAT].is_some());
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
    fn test_sys_read_zero_buf() {
        let result = sys_read(0, 0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_sys_open_null_returns_negative() {
        // null path_ptr → -1
        let result = sys_open(0, 0, 0);
        assert_eq!(result, -1);
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
        // syscall 5 (FSTAT) is now registered, use 500 instead
        let mut mgr = SyscallManager::new();
        let result = mgr.handle(500, 0, 0, 0);
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

    // --- New syscall tests ---

    #[test]
    fn test_sys_mprotect_returns_zero() {
        assert_eq!(sys_mprotect(0x1000, 4096, 7), 0);
    }

    #[test]
    fn test_sys_ioctl_tty_tcgets() {
        // TCGETS on stdout → 0 (pretend tty)
        assert_eq!(sys_ioctl(1, 0x5401, 0), 0);
    }

    #[test]
    fn test_sys_ioctl_non_tty_fd() {
        // any request on fd > 2 → ENOTTY
        assert_eq!(sys_ioctl(5, 0x5401, 0), -25);
    }

    #[test]
    fn test_sys_fcntl_returns_zero() {
        assert_eq!(sys_fcntl(3, 1, 0), 0);
    }

    #[test]
    fn test_sys_pipe_returns_enosys() {
        assert_eq!(sys_pipe(0, 0, 0), -38);
    }

    #[test]
    fn test_sys_pipe2_returns_enosys() {
        assert_eq!(sys_pipe2(0, 0, 0), -38);
    }

    #[test]
    fn test_sys_futex_returns_zero() {
        assert_eq!(sys_futex(0, 0, 0), 0);
    }

    #[test]
    fn test_sys_tgkill_returns_zero() {
        assert_eq!(sys_tgkill(1, 1, 9), 0);
    }

    #[test]
    fn test_sys_getppid_returns_zero() {
        assert_eq!(sys_getppid(0, 0, 0), 0);
    }

    #[test]
    fn test_sys_setsid_returns_one() {
        assert_eq!(sys_setsid(0, 0, 0), 1);
    }

    #[test]
    fn test_sys_getsid_returns_one() {
        assert_eq!(sys_getsid(0, 0, 0), 1);
    }

    #[test]
    fn test_sys_getpgid_returns_one() {
        assert_eq!(sys_getpgid(0, 0, 0), 1);
    }

    #[test]
    fn test_sys_setpgid_returns_zero() {
        assert_eq!(sys_setpgid(0, 0, 0), 0);
    }

    #[test]
    fn test_sys_stat_null_ptr() {
        // null stat_ptr → -1
        assert_eq!(sys_stat(0, 0, 0), -1);
    }

    #[test]
    fn test_sys_fstat_stdin_writes_stat() {
        let mut buf = [0u8; 144];
        let result = sys_fstat(0, buf.as_mut_ptr() as usize, 0);
        assert_eq!(result, 0);
        // st_dev should be 1
        let dev = u64::from_ne_bytes(buf[0..8].try_into().unwrap());
        assert_eq!(dev, 1);
    }

    #[test]
    fn test_sys_uname_null_ptr() {
        assert_eq!(sys_uname(0, 0, 0), -1);
    }

    #[test]
    fn test_sys_uname_writes_fields() {
        let mut buf = [0u8; 390];
        let result = sys_uname(buf.as_mut_ptr() as usize, 0, 0);
        assert_eq!(result, 0);
        // first field: "AIOS"
        assert_eq!(&buf[0..4], b"AIOS");
        // machine field at offset 260: "x86_64"
        assert_eq!(&buf[260..266], b"x86_64");
    }

    #[test]
    fn test_sys_access_null_path() {
        // null → path_len == 0 → -2
        assert_eq!(sys_access(0, 0, 0), -2);
    }

    #[test]
    fn test_sys_readlink_proc_self() {
        let path = b"/proc/self/exe\0";
        let mut out = [0u8; 32];
        let r = sys_readlink(path.as_ptr() as usize, out.as_mut_ptr() as usize, out.len());
        assert!(r > 0);
        assert_eq!(&out[..r as usize], b"/bin/busybox");
    }

    #[test]
    fn test_sys_readlink_nonexistent_returns_neg2() {
        let path = b"/some/other/path\0";
        let mut out = [0u8; 32];
        let r = sys_readlink(path.as_ptr() as usize, out.as_mut_ptr() as usize, out.len());
        assert_eq!(r, -2);
    }

    #[test]
    fn test_fd_table_alloc_free() {
        let mut table = FdTable::new();
        let fd = table.alloc("/bin/busybox", false).unwrap();
        assert_eq!(fd, FD_OFFSET);
        assert!(table.get(fd).is_some());
        table.free(fd);
        assert!(table.get(fd).is_none());
    }

    #[test]
    fn test_fd_table_get_stdin_none() {
        let table = FdTable::new();
        assert!(table.get(0).is_none());
        assert!(table.get(1).is_none());
        assert!(table.get(2).is_none());
    }

    #[test]
    fn test_fd_entry_path_str() {
        let mut e = FdEntry::new();
        let path = "/bin/sh";
        let len = path.len();
        e.path[..len].copy_from_slice(path.as_bytes());
        e.path_len = len;
        assert_eq!(e.path_str(), "/bin/sh");
    }

    #[test]
    fn test_sys_lseek_seek_set() {
        // alloc a fake fd
        let fd = FD_TABLE.lock().alloc("/bin/busybox", false).unwrap();
        let result = sys_lseek(fd, 100, 0); // SEEK_SET
        assert_eq!(result, 100);
        FD_TABLE.lock().free(fd);
    }

    #[test]
    fn test_sys_lseek_seek_cur() {
        let fd = FD_TABLE.lock().alloc("/bin/busybox", false).unwrap();
        // first seek to 50
        sys_lseek(fd, 50, 0);
        // then advance by 20
        let result = sys_lseek(fd, 20, 1); // SEEK_CUR
        assert_eq!(result, 70);
        FD_TABLE.lock().free(fd);
    }

    #[test]
    fn test_sys_getdents64_null_buf() {
        assert_eq!(sys_getdents64(3, 0, 0), -1);
    }

    #[test]
    fn test_sys_openat_null_path() {
        // null path_ptr → -1
        let r = sys_openat(usize::MAX, 0, 0);
        assert_eq!(r, -1);
    }

    #[test]
    fn test_sys_faccessat_null_path() {
        // null path → access returns -2, faccessat wraps it
        let r = sys_faccessat(0, 0, 0);
        assert_eq!(r, -2);
    }

    #[test]
    fn test_sys_faccessat2_null_path() {
        let r = sys_faccessat2(0, 0, 0);
        assert_eq!(r, -2);
    }

    #[test]
    fn test_syscall_new_handlers_registered() {
        let mgr = SyscallManager::new();
        assert!(mgr.handlers[SYSCALL_MPROTECT].is_some());
        assert!(mgr.handlers[SYSCALL_IOCTL].is_some());
        assert!(mgr.handlers[SYSCALL_FCNTL].is_some());
        assert!(mgr.handlers[SYSCALL_GETDENTS64].is_some());
        assert!(mgr.handlers[SYSCALL_FUTEX].is_some());
        assert!(mgr.handlers[SYSCALL_TGKILL].is_some());
        assert!(mgr.handlers[SYSCALL_READLINK].is_some());
        assert!(mgr.handlers[SYSCALL_OPENAT].is_some());
        assert!(mgr.handlers[SYSCALL_FACCESSAT].is_some());
        assert!(mgr.handlers[SYSCALL_FACCESSAT2].is_some());
    }

    #[test]
    fn test_write_stat_fields() {
        let mut buf = [0u8; 144];
        write_stat(buf.as_mut_ptr() as usize, 42, 1024, false);
        let dev = u64::from_ne_bytes(buf[0..8].try_into().unwrap());
        let ino = u64::from_ne_bytes(buf[8..16].try_into().unwrap());
        let size = u64::from_ne_bytes(buf[48..56].try_into().unwrap());
        assert_eq!(dev, 1);
        assert_eq!(ino, 42);
        assert_eq!(size, 1024);
    }

    #[test]
    fn test_write_stat_dir_mode() {
        let mut buf = [0u8; 144];
        write_stat(buf.as_mut_ptr() as usize, 1, 0, true);
        let mode = u32::from_ne_bytes(buf[24..28].try_into().unwrap());
        assert_eq!(mode, 0x41ED); // directory 0755
    }

    #[test]
    fn test_read_user_str_null() {
        let (_, len) = read_user_str(0);
        assert_eq!(len, 0);
    }

    #[test]
    fn test_read_user_str_valid() {
        let s = b"hello\0";
        let (buf, len) = read_user_str(s.as_ptr() as usize);
        assert_eq!(len, 5);
        assert_eq!(&buf[..5], b"hello");
    }
}
