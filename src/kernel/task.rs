// AIOS Task Manager
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Replace task manager stub with proper implementation

use core::ptr::null_mut;

const MAX_TASKS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TaskState {
    Unused = 0,
    Ready = 1,
    Running = 2,
    Waiting = 3,
    Finished = 4,
}

#[derive(Debug, Copy, Clone)]
pub struct Task {
    pub id: usize,
    pub state: TaskState,
    pub stack_ptr: *mut u8,
    pub stack_base: *mut u8,
    pub stack_size: usize,
    pub priority: u8,
    pub time_slice: u16,
}

impl Task {
    pub const fn new() -> Self {
        Self {
            id: 0,
            state: TaskState::Unused,
            stack_ptr: null_mut(),
            stack_base: null_mut(),
            stack_size: 0,
            priority: 0,
            time_slice: 0,
        }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn init(
        &mut self,
        id: usize,
        stack_base: *mut u8,
        stack_size: usize,
    ) -> Result<(), &'static str> {
        if id >= MAX_TASKS {
            return Err("Task ID exceeds maximum");
        }

        self.id = id;
        self.state = TaskState::Ready;
        self.stack_base = stack_base;
        self.stack_size = stack_size;
        self.stack_ptr = unsafe { stack_base.add(stack_size) };
        self.priority = 1;
        self.time_slice = 10;

        Ok(())
    }
}

impl Default for Task {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TaskManager {
    tasks: [Task; MAX_TASKS],
    current_task: Option<usize>,
    next_task_id: usize,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: [Task {
                id: 0,
                state: TaskState::Unused,
                stack_ptr: core::ptr::null_mut(),
                stack_base: core::ptr::null_mut(),
                stack_size: 0,
                priority: 0,
                time_slice: 0,
            }; MAX_TASKS],
            current_task: None,
            next_task_id: 0,
        }
    }

    pub fn create_task(
        &mut self,
        stack_base: *mut u8,
        stack_size: usize,
    ) -> Result<usize, &'static str> {
        if self.next_task_id >= MAX_TASKS {
            return Err("Maximum number of tasks reached");
        }

        let task_id = self.next_task_id;
        self.next_task_id += 1;

        let task = &mut self.tasks[task_id];
        task.init(task_id, stack_base, stack_size)?;

        Ok(task_id)
    }

    pub fn get_task(&self, id: usize) -> Option<&Task> {
        if id < MAX_TASKS {
            Some(&self.tasks[id])
        } else {
            None
        }
    }

    pub fn get_task_mut(&mut self, id: usize) -> Option<&mut Task> {
        if id < MAX_TASKS {
            Some(&mut self.tasks[id])
        } else {
            None
        }
    }

    pub fn set_current_task(&mut self, id: usize) {
        if id < MAX_TASKS {
            self.current_task = Some(id);
        }
    }

    pub fn get_current_task(&self) -> Option<&Task> {
        self.current_task.and_then(|id| self.get_task(id))
    }

    pub fn get_current_task_mut(&mut self) -> Option<&mut Task> {
        self.current_task.and_then(|id| self.get_task_mut(id))
    }

    pub fn schedule_next(&mut self) -> Option<usize> {
        let mut next_id = self.current_task.unwrap_or(0);
        let mut checked = 0;

        while checked < MAX_TASKS {
            next_id = (next_id + 1) % MAX_TASKS;
            let task = &self.tasks[next_id];

            if task.state == TaskState::Ready {
                self.set_current_task(next_id);
                return Some(next_id);
            }

            checked += 1;
        }

        self.current_task
    }

    pub fn yield_current(&mut self) -> Option<usize> {
        if let Some(current) = self.current_task {
            if let Some(task) = self.get_task_mut(current) {
                if task.state == TaskState::Running {
                    task.state = TaskState::Ready;
                }
            }
        }

        self.schedule_next()
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn switch_to(task: &Task) {
    core::hint::black_box(task);
}

static mut TASK_MANAGER: Option<TaskManager> = None;

pub fn init_scheduler() {
    // # Safety
    // Task manager is initialized once during kernel boot.
    // Single-core kernel - no data races during initialization.
    unsafe {
        TASK_MANAGER = Some(TaskManager::new());
    }
}

pub fn run_scheduler() {
    // # Safety
    // Accessing TASK_MANAGER is safe during scheduler execution.
    // The timer tick check and yield operation are atomic w.r.t. the scheduler.
    unsafe {
        if let Some(ref mut tm) = TASK_MANAGER {
            if crate::interrupts::is_timer_tick() {
                tm.yield_current();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_manager_creation() {
        let tm = TaskManager::new();
        assert_eq!(tm.next_task_id, 0);
        assert_eq!(tm.current_task, None);
    }

    #[test]
    fn test_task_creation() {
        let mut tm = TaskManager::new();

        let mut stack = [0u8; 4096];
        let stack_ptr = stack.as_mut_ptr();

        let result = tm.create_task(stack_ptr, stack.len());
        assert!(result.is_ok());
        let task_id = result.unwrap();

        assert_eq!(task_id, 0);
        let task = tm.get_task(task_id).unwrap();
        assert_eq!(task.id, 0);
        assert_eq!(task.state, TaskState::Ready);
        assert_eq!(task.stack_base, stack_ptr);
        assert_eq!(task.stack_size, 4096);
        assert_eq!(task.priority, 1);
        assert_eq!(task.time_slice, 10);
    }

    #[test]
    fn test_task_manager_max_tasks() {
        let mut tm = TaskManager::new();

        for i in 0..MAX_TASKS {
            let mut stack = [0u8; 1024];
            let stack_ptr = stack.as_mut_ptr();
            let result = tm.create_task(stack_ptr, stack.len());
            assert!(result.is_ok(), "Failed to create task {}", i);
        }

        let mut stack = [0u8; 1024];
        let stack_ptr = stack.as_mut_ptr();
        let result = tm.create_task(stack_ptr, stack.len());
        assert!(result.is_err());
    }

    #[test]
    fn test_task_switching() {
        let mut tm = TaskManager::new();

        let mut stack1 = [0u8; 1024];
        let stack_ptr1 = stack1.as_mut_ptr();
        let id1 = tm.create_task(stack_ptr1, stack1.len()).unwrap();

        let mut stack2 = [0u8; 1024];
        let stack_ptr2 = stack2.as_mut_ptr();
        let id2 = tm.create_task(stack_ptr2, stack2.len()).unwrap();

        tm.set_current_task(id1);
        assert_eq!(tm.get_current_task().unwrap().id, id1);

        let next_id = tm.schedule_next().unwrap();
        assert_eq!(next_id, id2);
        assert_eq!(tm.get_current_task().unwrap().id, id2);

        let next_id2 = tm.schedule_next().unwrap();
        assert_eq!(next_id2, id1);
        assert_eq!(tm.get_current_task().unwrap().id, id1);
    }

    #[test]
    fn test_task_yield() {
        let mut tm = TaskManager::new();

        let mut stack1 = [0u8; 1024];
        let stack_ptr1 = stack1.as_mut_ptr();
        let id1 = tm.create_task(stack_ptr1, stack1.len()).unwrap();

        let mut stack2 = [0u8; 1024];
        let stack_ptr2 = stack2.as_mut_ptr();
        let id2 = tm.create_task(stack_ptr2, stack2.len()).unwrap();

        tm.set_current_task(id1);
        {
            let task = tm.get_task_mut(id1).unwrap();
            task.state = TaskState::Running;
        }

        let next_id = tm.yield_current().unwrap();
        assert_eq!(next_id, id2);

        let task1 = tm.get_task(id1).unwrap();
        assert_eq!(task1.state, TaskState::Ready);

        assert_eq!(tm.get_current_task().unwrap().id, id2);
    }

    #[test]
    fn test_invalid_task_id() {
        let tm = TaskManager::new();
        assert!(tm.get_task(MAX_TASKS).is_none());
        assert!(tm.get_task_mut(MAX_TASKS).is_none());
    }

    #[test]
    fn test_task_state_enum() {
        assert_eq!(TaskState::Unused as u8, 0);
        assert_eq!(TaskState::Ready as u8, 1);
        assert_eq!(TaskState::Running as u8, 2);
        assert_eq!(TaskState::Waiting as u8, 3);
        assert_eq!(TaskState::Finished as u8, 4);
    }

    #[test]
    fn test_task_default() {
        let task = Task::default();
        assert_eq!(task.id, 0);
        assert_eq!(task.state, TaskState::Unused);
    }

    #[test]
    fn test_task_manager_default() {
        let tm = TaskManager::default();
        assert_eq!(tm.next_task_id, 0);
        assert_eq!(tm.current_task, None);
    }

    #[test]
    fn test_schedule_no_tasks() {
        let mut tm = TaskManager::new();
        let result = tm.schedule_next();
        assert!(result.is_none());
    }

    #[test]
    fn test_set_current_task_invalid() {
        let mut tm = TaskManager::new();
        tm.set_current_task(MAX_TASKS + 1);
        assert!(tm.get_current_task().is_none());
    }

    #[test]
    fn test_scheduler_init() {
        init_scheduler();
    }

    #[test]
    fn test_run_scheduler_no_panic() {
        run_scheduler();
    }

    #[test]
    fn test_task_manager_preserved() {
        init_scheduler();
        run_scheduler();
        // Verify task manager still accessible after scheduler runs
        unsafe {
            assert!(TASK_MANAGER.is_some());
        }
    }
}
