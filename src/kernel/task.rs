// AIOS Task/Process Manager
//
// Model: opencode
// Tool: opencode
// Prompt: Create task/process manager for AIOS x86_64 kernel in Rust no_std.
//         Define Task struct with id, stack pointer, state (Ready/Running/Blocked).
//         Implement TaskManager with array of up to 16 tasks.
//         Add functions: spawn_task, switch_task, get_current_task.

#![no_std]

use spin::Mutex;

/// Maximum number of tasks supported
pub const MAX_TASKS: usize = 16;

/// Default stack size for a task (8KB)
pub const TASK_STACK_SIZE: usize = 0x2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskId {
    Idle(usize),
    Invalid,
}

impl TaskId {
    pub fn as_usize(self) -> usize {
        match self {
            TaskId::Idle(id) => id,
            TaskId::Invalid => MAX_TASKS,
        }
    }

    pub fn is_valid(self) -> bool {
        !matches!(self, TaskId::Invalid)
    }
}

pub struct Task {
    pub id: TaskId,
    pub stack_ptr: usize,
    pub state: TaskState,
    entry_point: Option<fn()>,
}

impl Task {
    pub const fn new() -> Self {
        Task {
            id: TaskId::Invalid,
            stack_ptr: 0,
            state: TaskState::Blocked,
            entry_point: None,
        }
    }

    pub fn init(&mut self, id: TaskId, stack_ptr: usize, entry_point: fn()) {
        self.id = id;
        self.stack_ptr = stack_ptr;
        self.state = TaskState::Ready;
        self.entry_point = Some(entry_point);
    }

    pub fn set_state(&mut self, state: TaskState) {
        self.state = state;
    }

    pub fn set_running(&mut self) {
        self.state = TaskState::Running;
    }

    pub fn set_ready(&mut self) {
        self.state = TaskState::Ready;
    }

    pub fn set_blocked(&mut self) {
        self.state = TaskState::Blocked;
    }
}

pub struct TaskManager {
    tasks: [Task; MAX_TASKS],
    current_task: usize,
    task_count: usize,
}

impl TaskManager {
    pub const fn new() -> Self {
        TaskManager {
            tasks: [Task::new(); MAX_TASKS],
            current_task: 0,
            task_count: 0,
        }
    }

    pub fn spawn_task(&mut self, entry_point: fn()) -> Option<TaskId> {
        if self.task_count >= MAX_TASKS {
            return None;
        }

        let task_id = TaskId::Idle(self.task_count);

        let stack_top = self.allocate_stack()?;

        self.tasks[self.task_count].init(task_id, stack_top, entry_point);
        self.task_count += 1;

        Some(task_id)
    }

    fn allocate_stack(&self) -> Option<usize> {
        let base_addr = 0xFFFF_FF00_0000_0000u64
            + (self.task_count as u64 * TASK_STACK_SIZE as u64);
        Some(base_addr as usize + TASK_STACK_SIZE)
    }

    pub fn switch_task(&mut self) {
        let current_idx = self.current_task;

        if self.task_count == 0 {
            return;
        }

        self.tasks[current_idx].set_ready();

        let next_idx = (current_idx + 1) % self.task_count;
        self.current_task = next_idx;

        self.tasks[next_idx].set_running();
    }

    pub fn get_current_task(&self) -> Option<&Task> {
        if self.current_task < self.task_count {
            Some(&self.tasks[self.current_task])
        } else {
            None
        }
    }

    pub fn get_current_task_mut(&mut self) -> Option<&mut Task> {
        if self.current_task < self.task_count {
            Some(&mut self.tasks[self.current_task])
        } else {
            None
        }
    }

    pub fn get_task(&self, id: TaskId) -> Option<&Task> {
        let idx = id.as_usize();
        if idx < self.task_count {
            Some(&self.tasks[idx])
        } else {
            None
        }
    }

    pub fn block_current_task(&mut self) {
        if let Some(task) = self.get_current_task_mut() {
            task.set_blocked();
        }
    }

    pub fn wake_task(&mut self, id: TaskId) {
        if let Some(task) = self.get_task(id) {
            let idx = task.id.as_usize();
            if idx < self.task_count {
                self.tasks[idx].set_ready();
            }
        }
    }

    pub fn task_count(&self) -> usize {
        self.task_count
    }

    pub fn current_task_id(&self) -> Option<TaskId> {
        if self.task_count > 0 {
            Some(self.tasks[self.current_task].id)
        } else {
            None
        }
    }
}

pub static TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());

pub fn init() {
    let mut manager = TASK_MANAGER.lock();
    *manager = TaskManager::new();
}

pub fn spawn_task(entry_point: fn()) -> Option<TaskId> {
    let mut manager = TASK_MANAGER.lock();
    manager.spawn_task(entry_point)
}

pub fn switch_task() {
    let mut manager = TASK_MANAGER.lock();
    manager.switch_task();
}

pub fn get_current_task() -> Option<&'static Task> {
    let manager = TASK_MANAGER.lock();
    manager.get_current_task()
}

pub fn get_current_task_id() -> Option<TaskId> {
    let manager = TASK_MANAGER.lock();
    manager.current_task_id()
}

#[inline(always)]
pub fn save_context(_sp: usize) {
    unsafe {
        core::arch::asm!("nop", options(nomem, nostack));
    }
}

#[inline(always)]
pub fn restore_context(_sp: usize) {
    unsafe {
        core::arch::asm!("nop", options(nomem, nostack));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let mut task = Task::new();
        assert_eq!(task.state, TaskState::Blocked);
        assert!(!task.id.is_valid());
    }

    fn dummy_entry() {}

    #[test]
    fn test_task_manager_init() {
        let manager = TaskManager::new();
        assert_eq!(manager.task_count(), 0);
    }

    #[test]
    fn test_spawn_task() {
        let mut manager = TaskManager::new();
        let id = manager.spawn_task(dummy_entry);
        assert!(id.is_some());
        assert_eq!(manager.task_count(), 1);
    }

    #[test]
    fn test_max_tasks() {
        let mut manager = TaskManager::new();
        for _ in 0..MAX_TASKS {
            let result = manager.spawn_task(dummy_entry);
            assert!(result.is_some(), "Failed to spawn task");
        }

        let overflow = manager.spawn_task(dummy_entry);
        assert!(overflow.is_none(), "Should not exceed MAX_TASKS");
    }

    #[test]
    fn test_switch_task() {
        let mut manager = TaskManager::new();

        manager.spawn_task(dummy_entry).unwrap();
        manager.spawn_task(dummy_entry).unwrap();

        assert_eq!(manager.current_task_id().unwrap().as_usize(), 0);

        manager.switch_task();
        assert_eq!(manager.current_task_id().unwrap().as_usize(), 1);

        manager.switch_task();
        assert_eq!(manager.current_task_id().unwrap().as_usize(), 0);
    }

    #[test]
    fn test_task_state_transitions() {
        let mut task = Task::new();
        assert_eq!(task.state, TaskState::Blocked);

        task.set_ready();
        assert_eq!(task.state, TaskState::Ready);

        task.set_running();
        assert_eq!(task.state, TaskState::Running);

        task.set_blocked();
        assert_eq!(task.state, TaskState::Blocked);
    }
}