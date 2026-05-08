// AIOS Process Management
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Implement process management for AIOS kernel - process table, fork/execve/wait syscalls.

pub const MAX_PROCESSES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ProcessState {
    Unused = 0,
    Running = 1,
    Waiting = 2,
    Exited = 3,
}

pub const CWD_SIZE: usize = 256;

#[derive(Clone, Copy)]
pub struct Process {
    pub pid: usize,
    pub ppid: usize,
    pub state: ProcessState,
    pub exit_code: i32,
    pub cwd: [u8; CWD_SIZE],
    pub cwd_len: usize,
}

impl Process {
    pub const fn new() -> Self {
        Self {
            pid: 0,
            ppid: 0,
            state: ProcessState::Unused,
            exit_code: 0,
            cwd: [0u8; CWD_SIZE],
            cwd_len: 0,
        }
    }

    pub fn set_cwd(&mut self, path: &[u8]) {
        let len = path.len().min(CWD_SIZE - 1);
        self.cwd[..len].copy_from_slice(&path[..len]);
        self.cwd[len] = 0;
        self.cwd_len = len;
    }

    pub fn get_cwd_str(&self) -> &str {
        let cstr = &self.cwd[..self.cwd_len];
        core::str::from_utf8(cstr).unwrap_or("/")
    }
}

impl Default for Process {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ProcessTable {
    processes: [Process; MAX_PROCESSES],
    next_pid: usize,
}

impl ProcessTable {
    pub const fn new() -> Self {
        Self {
            processes: [Process::new(); MAX_PROCESSES],
            next_pid: 1,
        }
    }

    pub fn alloc_process(&mut self, parent_pid: usize) -> Option<usize> {
        for i in 0..MAX_PROCESSES {
            if self.processes[i].state == ProcessState::Unused {
                self.processes[i].pid = self.next_pid;
                self.processes[i].ppid = parent_pid;
                self.processes[i].state = ProcessState::Running;
                self.processes[i].exit_code = 0;
                self.processes[i].cwd_len = 1;
                self.processes[i].cwd = [0u8; CWD_SIZE];
                self.processes[i].cwd[0] = b'/';
                self.next_pid += 1;
                return Some(self.processes[i].pid);
            }
        }
        None
    }

    pub fn get_process(&self, pid: usize) -> Option<&Process> {
        for i in 0..MAX_PROCESSES {
            if self.processes[i].pid == pid && self.processes[i].state != ProcessState::Unused {
                return Some(&self.processes[i]);
            }
        }
        None
    }

    pub fn get_process_mut(&mut self, pid: usize) -> Option<&mut Process> {
        for i in 0..MAX_PROCESSES {
            if self.processes[i].pid == pid && self.processes[i].state != ProcessState::Unused {
                return Some(&mut self.processes[i]);
            }
        }
        None
    }

    pub fn get_parent(&self, pid: usize) -> Option<&Process> {
        if let Some(proc) = self.get_process(pid) {
            if proc.ppid == 0 {
                return None;
            }
            return self.get_process(proc.ppid);
        }
        None
    }

    pub fn get_process_by_ppid(&self, ppid: usize) -> Option<(usize, &Process)> {
        for i in 0..MAX_PROCESSES {
            if self.processes[i].ppid == ppid && self.processes[i].state == ProcessState::Running {
                return Some((i, &self.processes[i]));
            }
        }
        None
    }

    pub fn get_process_by_ppid_mut(&mut self, ppid: usize) -> Option<&mut Process> {
        for i in 0..MAX_PROCESSES {
            if self.processes[i].ppid == ppid && self.processes[i].state == ProcessState::Running {
                return Some(&mut self.processes[i]);
            }
        }
        None
    }

    pub fn free_process(&mut self, pid: usize) {
        if let Some(proc) = self.get_process_mut(pid) {
            proc.state = ProcessState::Unused;
            proc.pid = 0;
        }
    }

    pub fn set_exit_status(&mut self, pid: usize, code: i32) {
        if let Some(proc) = self.get_process_mut(pid) {
            proc.exit_code = code;
            proc.state = ProcessState::Exited;
        }
    }

    pub fn wait_for_child(&mut self, parent_pid: usize) -> Option<(usize, i32)> {
        for i in 0..MAX_PROCESSES {
            if self.processes[i].ppid == parent_pid
                && self.processes[i].state == ProcessState::Exited
            {
                let pid = self.processes[i].pid;
                let code = self.processes[i].exit_code;
                self.processes[i].state = ProcessState::Unused;
                self.processes[i].pid = 0;
                return Some((pid, code));
            }
        }
        None
    }

    pub fn find_unused_index(&self) -> Option<usize> {
        (0..MAX_PROCESSES).find(|&i| self.processes[i].state == ProcessState::Unused)
    }

    pub fn copy_process(&mut self, src_pid: usize, child_pid: usize) -> Option<usize> {
        let src_idx = (0..MAX_PROCESSES).find(|&i| self.processes[i].pid == src_pid)?;
        let dst_idx =
            (0..MAX_PROCESSES).find(|&i| self.processes[i].state == ProcessState::Unused)?;

        self.processes[dst_idx] = self.processes[src_idx];
        self.processes[dst_idx].pid = child_pid;
        self.processes[dst_idx].state = ProcessState::Running;
        Some(dst_idx)
    }

    pub fn process_count(&self) -> usize {
        self.processes
            .iter()
            .filter(|p| p.state != ProcessState::Unused)
            .count()
    }
}

impl Default for ProcessTable {
    fn default() -> Self {
        Self::new()
    }
}

pub static PROCESS_TABLE: spin::Mutex<ProcessTable> = spin::Mutex::new(ProcessTable::new());

pub static mut CURRENT_PID: usize = 0;

pub fn init() {
    let mut table = PROCESS_TABLE.lock();
    let pid = table.alloc_process(0).unwrap_or(1);
    drop(table);
    unsafe { CURRENT_PID = pid };
}

pub fn alloc_process(parent_pid: usize) -> Option<usize> {
    PROCESS_TABLE.lock().alloc_process(parent_pid)
}

pub fn get_process(pid: usize) -> Option<Process> {
    PROCESS_TABLE.lock().get_process(pid).copied()
}

pub fn get_current_process() -> Option<Process> {
    unsafe { PROCESS_TABLE.lock().get_process(CURRENT_PID).copied() }
}

pub fn get_current_pid() -> usize {
    unsafe { CURRENT_PID }
}

pub fn set_current_pid(pid: usize) {
    unsafe { CURRENT_PID = pid };
}

pub fn get_current_cwd() -> Option<Process> {
    get_current_process()
}

pub fn process_table_lock() -> &'static spin::Mutex<ProcessTable> {
    &PROCESS_TABLE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_process() {
        let mut table = ProcessTable::new();
        let pid = table.alloc_process(0);
        assert!(pid.is_some());
        assert_eq!(1, pid.unwrap());
    }

    #[test]
    fn test_get_process() {
        let mut table = ProcessTable::new();
        let pid = table.alloc_process(0).unwrap();
        let proc = table.get_process(pid);
        assert!(proc.is_some());
    }

    #[test]
    fn test_max_processes() {
        let mut table = ProcessTable::new();
        for i in 0..MAX_PROCESSES {
            let result = table.alloc_process(0);
            assert!(result.is_some(), "Failed at iteration {}", i);
        }
        let result = table.alloc_process(0);
        assert!(result.is_none());
    }

    #[test]
    fn test_process_set_cwd() {
        let mut proc = Process::new();
        proc.set_cwd(b"/test/path");
        assert_eq!("/test/path", proc.get_cwd_str());
    }

    #[test]
    fn test_process_default() {
        let proc = Process::default();
        assert_eq!(proc.state, ProcessState::Unused);
    }

    #[test]
    fn test_wait_for_child_no_children() {
        let mut table = ProcessTable::new();
        let parent = table.alloc_process(0).unwrap();
        let result = table.wait_for_child(parent);
        assert!(result.is_none());
    }

    #[test]
    fn test_wait_for_child_with_exited() {
        let mut table = ProcessTable::new();
        let parent = table.alloc_process(0).unwrap();
        let child = table.alloc_process(parent).unwrap();
        table.set_exit_status(child, 42);
        let result = table.wait_for_child(parent);
        assert!(result.is_some());
        let (pid, code) = result.unwrap();
        assert_eq!(child, pid);
        assert_eq!(42, code);
    }

    #[test]
    fn test_free_process() {
        let mut table = ProcessTable::new();
        let pid = table.alloc_process(0).unwrap();
        table.free_process(pid);
        assert!(table.get_process(pid).is_none());
    }

    #[test]
    fn test_set_exit_status() {
        let mut table = ProcessTable::new();
        let pid = table.alloc_process(0).unwrap();
        table.set_exit_status(pid, 99);
        let proc = table.get_process(pid).unwrap();
        assert_eq!(99, proc.exit_code);
        assert_eq!(ProcessState::Exited, proc.state);
    }

    #[test]
    fn test_copy_process() {
        let mut table = ProcessTable::new();
        let src = table.alloc_process(0).unwrap();
        let child_pid = 99;
        let idx = table.copy_process(src, child_pid);
        assert!(idx.is_some());
        let proc = table.get_process(child_pid).unwrap();
        assert_eq!(child_pid, proc.pid);
        assert_eq!(src, proc.ppid);
    }

    #[test]
    fn test_process_count() {
        let mut table = ProcessTable::new();
        assert_eq!(0, table.process_count());
        table.alloc_process(0);
        assert_eq!(1, table.process_count());
    }

    #[test]
    fn test_get_parent() {
        let mut table = ProcessTable::new();
        let parent = table.alloc_process(0).unwrap();
        let child = table.alloc_process(parent).unwrap();
        let parent_proc = table.get_process(parent).unwrap();
        assert_eq!(0, parent_proc.ppid);
        let child_proc = table.get_process(child).unwrap();
        assert_eq!(parent, child_proc.ppid);
    }

    #[test]
    fn test_cwd_truncation() {
        let mut proc = Process::new();
        let long_path = [b'x'; 300];
        proc.set_cwd(&long_path);
        assert_eq!(CWD_SIZE - 1, proc.cwd_len);
    }

    #[test]
    fn test_process_state_enum() {
        assert_eq!(ProcessState::Unused as u8, 0);
        assert_eq!(ProcessState::Running as u8, 1);
        assert_eq!(ProcessState::Waiting as u8, 2);
        assert_eq!(ProcessState::Exited as u8, 3);
    }

    #[test]
    fn test_find_unused_index() {
        let mut table = ProcessTable::new();
        let idx = table.find_unused_index();
        assert_eq!(idx, Some(0));
    }
}
