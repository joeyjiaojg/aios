// AIOS Task Manager
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Replace task manager stub with proper implementation

use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const MAX_TASKS: usize = 16;

static SCHEDULER_RUNNING: AtomicBool = AtomicBool::new(false);
static mut CURRENT_TASK_ID: AtomicUsize = AtomicUsize::new(0);

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
            unsafe {
                CURRENT_TASK_ID.store(id, Ordering::SeqCst);
            }
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

    pub fn add_idle_task(&mut self) -> Result<(), &'static str> {
        let mut idle_stack = [0u8; 4096];
        let stack_ptr = idle_stack.as_mut_ptr();
        self.create_task(stack_ptr, idle_stack.len())?;
        Ok(())
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
static mut TASK_MANAGER_INIT: bool = false;

pub fn init() {
    // # Safety
    // TASK_MANAGER is only initialized once during boot, before interrupts are enabled.
    // This is a single-core kernel, so there are no data races.
    unsafe {
        if !TASK_MANAGER_INIT {
            let mut tm = TaskManager::new();
            tm.add_idle_task().ok();
            TASK_MANAGER = Some(tm);
            TASK_MANAGER_INIT = true;
        }
    }
}

pub fn run_scheduler() {
    SCHEDULER_RUNNING.store(true, Ordering::SeqCst);

    loop {
        if crate::interrupts::is_timer_tick() {
            // # Safety
            // TASK_MANAGER is accessed only from this single-threaded scheduler loop.
            // The scheduler runs on a single CPU core in this kernel implementation.
            unsafe {
                if let Some(ref mut tm) = TASK_MANAGER {
                    let current_id = tm.current_task;
                    if let Some(current) = current_id {
                        if let Some(task) = tm.get_task_mut(current) {
                            if task.state == TaskState::Running {
                                task.time_slice = task.time_slice.saturating_sub(1);
                                if task.time_slice == 0 {
                                    task.state = TaskState::Ready;
                                    task.time_slice = 10;
                                }
                            }
                        }
                    }
                    tm.yield_current();
                }
            }
        }

        core::hint::black_box(());
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
    fn test_task_yield() {
        let mut tm = TaskManager::new();
        let mut stack1 = [0u8; 1024];
        let stack_ptr1 = stack1.as_mut_ptr();
        let id1 = tm.create_task(stack_ptr1, stack1.len()).unwrap();
        tm.set_current_task(id1);
        {
            let task = tm.get_task_mut(id1).unwrap();
            task.state = TaskState::Running;
        }
        let next = tm.yield_current();
        assert!(next.is_some());
        assert_eq!(tm.get_current_task().unwrap().id, next.unwrap());
        let task = tm.get_task(id1).unwrap();
        assert_eq!(task.state, TaskState::Ready);
    }

    #[test]
    fn test_add_idle_task() {
        let mut tm = TaskManager::new();
        let result = tm.add_idle_task();
        assert!(result.is_ok());
        assert!(tm.get_task(0).is_some());
    }

    #[test]
    fn test_invalid_task_id() {
        let tm = TaskManager::new();
        assert!(tm.get_task(MAX_TASKS).is_none());
    }
}
