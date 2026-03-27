use std::collections::HashSet;
use sysinfo::{Pid, Process, ProcessStatus, ProcessesToUpdate, Signal, System};

/// Manages system processes, including whitelisting,
/// identifying top memory consumers, and sending signals.
pub struct ProcessManager {
    system: System,
    whitelist: HashSet<String>,
}

impl ProcessManager {
    /// Creates a new `ProcessManager` instance with an initialized `sysinfo::System`
    /// and a predefined whitelist of critical macOS components and SentinelRS itself.
    pub fn new() -> Self {
        let mut whitelist = HashSet::new();
        // Extended whitelist based on critical macOS components
        let safe_apps = vec![
            "kernel_task",
            "launchd",
            "WindowServer",
            "SystemUIServer",
            "Dock",
            "Finder",
            "loginwindow",
            "sysmond",
            "SentinelRS", // Ensure SentinelRS doesn't freeze itself
        ];
        for app in safe_apps {
            whitelist.insert(app.to_string());
        }

        Self {
            system: System::new_all(),
            whitelist,
        }
    }

    /// Refreshes the system's process list and memory information.
    /// This should be called periodically to get up-to-date process data.
    pub fn refresh(&mut self) {
        // Updates all processes and removes those that have exited.
        self.system.refresh_processes(ProcessesToUpdate::All, true);
        self.system.refresh_memory();
    }

    /// Retrieves the top memory-consuming processes,
    /// excluding whitelisted and already suspended processes.
    ///
    /// The processes are sorted in descending order of memory usage.
    ///
    /// # Arguments
    ///
    /// * `count` - The maximum number of top processes to return.
    ///
    /// # Returns
    ///
    /// A vector of tuples, where each tuple contains the process PID, name, and memory usage in bytes.
    pub fn get_top_consumers(&self, count: usize) -> Vec<(Pid, String, u64)> {
        let mut processes: Vec<&Process> = self
            .system
            .processes()
            .values()
            .filter(|p| {
                let name = p.name().to_string_lossy();
                // 1. Not in whitelist
                // 2. Not a system PID (typically < 100)
                // 3. Process must be active (not already stopped/zombie)
                !self.whitelist.contains(name.as_ref())
                    && p.pid().as_u32() > 100
                    && !matches!(p.status(), ProcessStatus::Stop | ProcessStatus::Zombie)
            })
            .collect();

        // Sort in descending order by memory usage
        processes.sort_by(|a, b| b.memory().cmp(&a.memory()));

        processes
            .into_iter()
            .take(count)
            .map(|p| (p.pid(), p.name().to_string_lossy().to_string(), p.memory()))
            .collect()
    }

    /// Identifies suspended (SIGSTOP'd) processes that are still consuming memory.
    /// This can be useful for escalating from SIGSTOP to SIGKILL if memory pressure persists.
    ///
    /// # Returns
    ///
    /// A vector of tuples, where each tuple contains the process PID, name, and memory usage in bytes.
    pub fn get_suspended_consumers(&self) -> Vec<(Pid, String, u64)> {
        let mut suspended: Vec<&Process> = self
            .system
            .processes()
            .values()
            .filter(|p| matches!(p.status(), ProcessStatus::Stop) && p.memory() > 0)
            .collect();

        suspended.sort_by(|a, b| b.memory().cmp(&a.memory()));

        suspended
            .into_iter()
            .map(|p| (p.pid(), p.name().to_string_lossy().to_string(), p.memory()))
            .collect()
    }

    /// Sends a `SIGSTOP` signal to a process, pausing its execution.
    /// This prevents further memory allocation by the process.
    ///
    /// # Arguments
    ///
    /// * `pid` - The PID of the process to freeze.
    ///
    /// # Returns
    ///
    /// `true` if the signal was successfully sent, `false` otherwise.
    pub fn freeze_process(&self, pid: Pid) -> bool {
        if let Some(process) = self.system.process(pid) {
            // SIGSTOP pauses the process and prevents further allocation
            return process.kill_with(Signal::Stop).is_some();
        }
        false
    }

    /// Sends a `SIGCONT` signal to a suspended process, resuming its execution.
    ///
    /// # Arguments
    ///
    /// * `pid` - The PID of the process to resume.
    ///
    /// # Returns
    ///
    /// `true` if the signal was successfully sent, `false` otherwise.
    pub fn resume_process(&self, pid: Pid) -> bool {
        if let Some(process) = self.system.process(pid) {
            // SIGCONT resumes a stopped process
            return process.kill_with(Signal::Continue).is_some();
        }
        false
    }

    /// Sends a `SIGKILL` signal to a process, forcing its immediate termination.
    ///
    /// # Arguments
    ///
    /// * `pid` - The PID of the process to kill.
    ///
    /// # Returns
    ///
    /// `true` if the signal was successfully sent, `false` otherwise.
    pub fn kill_process(&self, pid: Pid) -> bool {
        if let Some(process) = self.system.process(pid) {
            // SIGKILL forces immediate termination
            return process.kill_with(Signal::Kill).is_some();
        }
        false
    }
}
