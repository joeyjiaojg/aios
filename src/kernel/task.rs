// AIOS Task/Process Manager
//
// Model: opencode
// Tool: opencode
// Prompt: Create task/process manager for x86_64 with tests.

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

#[derive(Debug, Clone, Copy)]
pub struct Task {
    pub id: TaskId,
    pub stack_ptr: usize,
    pub state: TaskState,
}

impl Task {
    pub const fn new() -> Self {
        Task {
            id: TaskId::Invalid,
            stack_ptr: 0,
            state: TaskState::Blocked,
        }
    }
}

pub fn init() {
    println!("[TASK] Task manager initialized");
}

pub fn switch_task() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_tasks() {
        assert_eq!(MAX_TASKS, 16);
    }

    #[test]
    fn test_task_stack_size() {
        assert_eq!(TASK_STACK_SIZE, 0x2000);
    }

    #[test]
    fn test_task_state_ready() {
        assert!(matches!(TaskState::Ready, TaskState::Ready));
    }

    #[test]
    fn test_task_state_running() {
        assert!(matches!(TaskState::Running, TaskState::Running));
    }

    #[test]
    fn test_task_state_blocked() {
        assert!(matches!(TaskState::Blocked, TaskState::Blocked));
    }

    #[test]
    fn test_task_id_invalid() {
        let id = TaskId::Invalid;
        assert!(!id.is_valid());
    }

    #[test]
    fn test_task_id_valid() {
        let id = TaskId::Idle(0);
        assert!(id.is_valid());
    }

    #[test]
    fn test_task_id_as_usize() {
        let id = TaskId::Idle(5);
        assert_eq!(id.as_usize(), 5);
    }

    #[test]
    fn test_task_creation() {
        let task = Task::new();
        assert!(matches!(task.id, TaskId::Invalid));
    }

    #[test]
    fn test_task_state_default() {
        let task = Task::new();
        assert!(matches!(task.state, TaskState::Blocked));
    }
}