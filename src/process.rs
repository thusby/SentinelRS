use sysinfo::{Pid, Process, System, Signal};
use std::collections::HashSet;

pub struct ProcessManager {
    system: System,
    whitelist: HashSet<String>,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut whitelist = HashSet::new();
        let safe_apps = vec![
            "kernel_task", "launchd", "WindowServer", "SystemUIServer", "Dock", "Finder", "loginwindow", "sysmond"
        ];
        for app in safe_apps {
            whitelist.insert(app.to_string());
        }

        Self {
            system: System::new_all(),
            whitelist,
        }
    }

    pub fn refresh(&mut self) {
        self.system.refresh_all();
    }

    pub fn get_top_consumers(&self, count: usize) -> Vec<(Pid, String, u64)> {
        let mut processes: Vec<&Process> = self.system.processes().values().collect();
        // Sort descending by memory
        processes.sort_by(|a, b| b.memory().cmp(&a.memory()));

        processes.into_iter()
            .filter(|p| {
                let name = p.name().to_string_lossy().to_string();
                !self.whitelist.contains(&name) && p.pid().as_u32() > 100
            })
            .take(count)
            .map(|p| (p.pid(), p.name().to_string_lossy().to_string(), p.memory()))
            .collect()
    }

    pub fn freeze_process(&self, pid: Pid) -> bool {
        if let Some(process) = self.system.process(pid) {
            return process.kill_with(Signal::Stop).unwrap_or(false);
        }
        false
    }

    pub fn resume_process(&self, pid: Pid) -> bool {
        if let Some(process) = self.system.process(pid) {
            return process.kill_with(Signal::Continue).unwrap_or(false);
        }
        false
    }

    pub fn kill_process(&self, pid: Pid) -> bool {
        if let Some(process) = self.system.process(pid) {
            return process.kill_with(Signal::Kill).unwrap_or(false);
        }
        false
    }
}
