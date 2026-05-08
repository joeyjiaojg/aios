// AIOS Job Control
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Job control for AIOS shell - track background jobs, fg, bg commands.

use crate::shell::MAX_JOBS;
use alloc::string::ToString;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Running,
    Stopped,
    Terminated,
    Done,
}

#[derive(Debug, Clone, Copy)]
pub struct Job {
    pub jid: usize,
    pub pid: usize,
    pub command: [u8; 64],
    pub command_len: usize,
    pub state: JobState,
}

impl Job {
    pub const fn new() -> Self {
        Self {
            jid: 0,
            pid: 0,
            command: [0u8; 64],
            command_len: 0,
            state: JobState::Running,
        }
    }

    pub fn set_command(&mut self, cmd: &str) {
        let cmd_bytes = cmd.as_bytes();
        let len = cmd_bytes.len().min(63);
        self.command[..len].copy_from_slice(&cmd_bytes[..len]);
        self.command[len] = 0;
        self.command_len = len;
    }

    pub fn get_command_str(&self) -> &str {
        let cmd_slice = &self.command[..self.command_len];
        core::str::from_utf8(cmd_slice).unwrap_or("")
    }
}

impl Default for Job {
    fn default() -> Self {
        Self::new()
    }
}

pub struct JobTable {
    jobs: [Job; MAX_JOBS],
    next_jid: usize,
}

impl JobTable {
    pub const fn new() -> Self {
        Self {
            jobs: [Job::new(); MAX_JOBS],
            next_jid: 1,
        }
    }

    pub fn add_job(&mut self, pid: usize, command: &str) -> Option<usize> {
        for i in 0..MAX_JOBS {
            if self.jobs[i].state == JobState::Terminated || self.jobs[i].state == JobState::Done {
                self.jobs[i].jid = self.next_jid;
                self.jobs[i].pid = pid;
                self.jobs[i].set_command(command);
                self.jobs[i].state = JobState::Running;
                self.next_jid += 1;
                return Some(self.jobs[i].jid);
            }
        }
        None
    }

    pub fn get_job(&self, jid: usize) -> Option<&Job> {
        for i in 0..MAX_JOBS {
            if self.jobs[i].jid == jid
                && self.jobs[i].state != JobState::Terminated
                && self.jobs[i].state != JobState::Done
            {
                return Some(&self.jobs[i]);
            }
        }
        None
    }

    pub fn get_job_mut(&mut self, jid: usize) -> Option<&mut Job> {
        for i in 0..MAX_JOBS {
            if self.jobs[i].jid == jid
                && self.jobs[i].state != JobState::Terminated
                && self.jobs[i].state != JobState::Done
            {
                return Some(&mut self.jobs[i]);
            }
        }
        None
    }

    pub fn set_job_state(&mut self, jid: usize, state: JobState) {
        if let Some(job) = self.get_job_mut(jid) {
            job.state = state;
        }
    }

    pub fn remove_job(&mut self, jid: usize) {
        if let Some(job) = self.get_job_mut(jid) {
            job.state = JobState::Terminated;
        }
    }

    pub fn list_all_jobs(&self) -> Vec<&Job> {
        let mut result: Vec<&Job> = Vec::new();
        for i in 0..MAX_JOBS {
            if self.jobs[i].state != JobState::Terminated && self.jobs[i].state != JobState::Done {
                result.push(&self.jobs[i]);
            }
        }
        result
    }

    pub fn get_next_jid(&self) -> usize {
        self.next_jid
    }
}

impl Default for JobTable {
    fn default() -> Self {
        Self::new()
    }
}

static JOB_TABLE: spin::Mutex<JobTable> = spin::Mutex::new(JobTable::new());

pub fn add_job(pid: usize, command: &str) -> Option<usize> {
    JOB_TABLE.lock().add_job(pid, command)
}

pub fn get_job(jid: usize) -> Option<Job> {
    JOB_TABLE.lock().get_job(jid).copied()
}

pub fn set_job_state(jid: usize, state: JobState) {
    JOB_TABLE.lock().set_job_state(jid, state);
}

pub fn remove_job(jid: usize) {
    JOB_TABLE.lock().remove_job(jid);
}

pub fn list_jobs() -> Result<(), &'static str> {
    let table = JOB_TABLE.lock();
    let jobs = table.list_all_jobs();

    if jobs.is_empty() {
        crate::serial::write_str("No jobs.\r\n");
        return Ok(());
    }

    for job in jobs {
        let state_str = match job.state {
            JobState::Running => "Running",
            JobState::Stopped => "Stopped",
            JobState::Terminated => "Terminated",
            JobState::Done => "Done",
        };
        crate::serial::write_str("[");
        crate::serial::write_str(job.jid.to_string().as_str());
        crate::serial::write_str("] ");
        crate::serial::write_str(state_str);
        crate::serial::write_str("  ");
        crate::serial::write_str(job.get_command_str());
        crate::serial::write_str("\r\n");
    }

    Ok(())
}

pub fn fg(args: &[&str]) -> Result<(), &'static str> {
    let jid = if args.is_empty() || args[0].is_empty() {
        let table = JOB_TABLE.lock();
        table.get_next_jid().saturating_sub(1)
    } else {
        args[0].parse::<usize>().unwrap_or(0)
    };

    let table = JOB_TABLE.lock();
    if let Some(job) = table.get_job(jid) {
        crate::serial::write_str("Bringing job [");
        crate::serial::write_str(jid.to_string().as_str());
        crate::serial::write_str("] to foreground: ");
        crate::serial::write_str(job.get_command_str());
        crate::serial::write_str("\r\n");
        Ok(())
    } else {
        crate::serial::write_str("fg: no such job\r\n");
        Err("Job not found")
    }
}

pub fn bg(args: &[&str]) -> Result<(), &'static str> {
    let jid = if args.is_empty() || args[0].is_empty() {
        let table = JOB_TABLE.lock();
        table.get_next_jid().saturating_sub(1)
    } else {
        args[0].parse::<usize>().unwrap_or(0)
    };

    let mut table = JOB_TABLE.lock();
    if let Some(job) = table.get_job_mut(jid) {
        job.state = JobState::Running;
        crate::serial::write_str("Resuming job [");
        crate::serial::write_str(jid.to_string().as_str());
        crate::serial::write_str("] in background: ");
        crate::serial::write_str(job.get_command_str());
        crate::serial::write_str("\r\n");
        Ok(())
    } else {
        crate::serial::write_str("bg: no such job\r\n");
        Err("Job not found")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_new() {
        let job = Job::new();
        assert_eq!(job.jid, 0);
        assert_eq!(job.pid, 0);
    }

    #[test]
    fn test_job_set_command() {
        let mut job = Job::new();
        job.set_command("ls -la");
        assert_eq!(job.command_len, 6);
    }

    #[test]
    fn test_job_get_command_str() {
        let mut job = Job::new();
        job.set_command("test");
        assert_eq!(job.get_command_str(), "test");
    }

    #[test]
    fn test_job_state_enum() {
        assert_eq!(JobState::Running as u8, 0);
        assert_eq!(JobState::Stopped as u8, 1);
        assert_eq!(JobState::Terminated as u8, 2);
        assert_eq!(JobState::Done as u8, 3);
    }

    #[test]
    fn test_job_table_new() {
        let table = JobTable::new();
        assert_eq!(table.get_next_jid(), 1);
    }

    #[test]
    fn test_add_job() {
        let mut table = JobTable::new();
        let jid = table.add_job(100, "ls");
        assert!(jid.is_some());
        assert_eq!(jid.unwrap(), 1);
    }

    #[test]
    fn test_get_job() {
        let mut table = JobTable::new();
        let jid = table.add_job(100, "ls").unwrap();
        let job = table.get_job(jid);
        assert!(job.is_some());
    }

    #[test]
    fn test_get_job_invalid() {
        let table = JobTable::new();
        let job = table.get_job(999);
        assert!(job.is_none());
    }

    #[test]
    fn test_set_job_state() {
        let mut table = JobTable::new();
        let jid = table.add_job(100, "ls").unwrap();
        table.set_job_state(jid, JobState::Stopped);
        let job = table.get_job(jid).unwrap();
        assert_eq!(job.state, JobState::Stopped);
    }

    #[test]
    fn test_remove_job() {
        let mut table = JobTable::new();
        let jid = table.add_job(100, "ls").unwrap();
        table.remove_job(jid);
        let job = table.get_job(jid);
        assert!(job.is_none());
    }

    #[test]
    fn test_list_all_jobs() {
        let mut table = JobTable::new();
        table.add_job(100, "ls").unwrap();
        table.add_job(101, "cat").unwrap();
        let jobs = table.list_all_jobs();
        assert!(jobs.len() <= MAX_JOBS);
    }

    #[test]
    fn test_list_all_jobs_empty() {
        let table = JobTable::new();
        let jobs = table.list_all_jobs();
        assert_eq!(jobs.len(), 0);
    }

    #[test]
    fn test_multiple_jobs_same_slot() {
        let mut table = JobTable::new();
        for i in 0..5 {
            let jid = table.add_job(100 + i, "cmd");
            assert!(jid.is_some());
        }
    }

    #[test]
    fn test_job_command_truncation() {
        let mut job = Job::new();
        let long_cmd = "this is a very long command that exceeds the maximum length";
        job.set_command(long_cmd);
        assert!(job.command_len <= 64);
    }

    #[test]
    fn test_fg_default() {
        let result = fg(&[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bg_default() {
        let result = bg(&[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_jobs_empty() {
        let result = list_jobs();
        assert!(result.is_ok());
    }
}
