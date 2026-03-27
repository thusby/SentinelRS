use std::collections::HashSet;
use sysinfo::{Pid, Process, ProcessStatus, Signal, System};

pub struct ProcessManager {
    system: System,
    whitelist: HashSet<String>,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut whitelist = HashSet::new();
        // Utvidet whitelist basert på kritiske macOS komponenter
        let safe_apps = vec![
            "kernel_task",
            "launchd",
            "WindowServer",
            "SystemUIServer",
            "Dock",
            "Finder",
            "loginwindow",
            "sysmond",
            "SentinelRS",
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
        // Vi oppdaterer kun det vi trenger for å spare ressurser
        self.system.refresh_processes();
        self.system.refresh_memory();
    }

    /// Henter de største synderne, men filtrerer bort suspenderte prosesser
    /// slik at vi ikke prøver å "fryse" noe som allerede er fryst.
    pub fn get_top_consumers(&self, count: usize) -> Vec<(Pid, String, u64)> {
        let mut processes: Vec<&Process> = self
            .system
            .processes()
            .values()
            .filter(|p| {
                let name = p.name().to_string_lossy();
                // 1. Ikke i whitelist
                // 2. Ikke en system-PID (< 100)
                // 3. Prosessen må være aktiv (ikke allerede stoppet/zombie)
                !self.whitelist.contains(name.as_ref())
                    && p.pid().as_u32() > 100
                    && !matches!(p.status(), ProcessStatus::Stop | ProcessStatus::Zombie)
            })
            .collect();

        // Sorter synkende etter minnebruk
        processes.sort_by(|a, b| b.memory().cmp(&a.memory()));

        processes
            .into_iter()
            .take(count)
            .map(|p| (p.pid(), p.name().to_string_lossy().to_string(), p.memory()))
            .collect()
    }

    /// Ny funksjon for å identifisere "Zombies" eller suspenderte tunge prosesser.
    /// Nyttig for eskalering fra SIGSTOP til SIGKILL hvis minnepresset vedvarer.
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

    pub fn freeze_process(&self, pid: Pid) -> bool {
        if let Some(process) = self.system.process(pid) {
            // SIGSTOP pauser prosessen og hindrer videre allokering
            return process.kill_with(Signal::Stop).is_some();
        }
        false
    }

    pub fn resume_process(&self, pid: Pid) -> bool {
        if let Some(process) = self.system.process(pid) {
            // SIGCONT fortsetter en stoppet prosess
            return process.kill_with(Signal::Continue).is_some();
        }
        false
    }

    pub fn kill_process(&self, pid: Pid) -> bool {
        if let Some(process) = self.system.process(pid) {
            // SIGKILL tvinger avslutning umiddelbart
            return process.kill_with(Signal::Kill).is_some();
        }
        false
    }
}
