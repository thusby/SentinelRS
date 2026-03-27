mod memory;
mod process;

use std::collections::{VecDeque, HashMap};
use std::process::Command;
use std::thread;
use std::time::Duration;

use mac_notification_sys::Notification;
use muda::{Menu, MenuItem, Submenu, PredefinedMenuItem, MenuEvent, MenuId};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::event::Event;
use tray_icon::{TrayIconBuilder, Icon};
use sysinfo::Pid;

use crate::memory::get_memory_level;
use crate::process::ProcessManager;

#[derive(Debug)]
enum UserEvent {
    UpdateStatus { level: u32, trend: String },
    UpdateTopConsumers(Vec<(Pid, String, u64)>),
    PanicTriggered { pid: Pid, name: String, memory_mb: u64 },
}

#[derive(Debug, Clone)]
enum MenuAction {
    Freeze(Pid, String),
    Kill(Pid, String),
}

fn create_empty_icon() -> Icon {
    Icon::from_rgba(vec![0, 0, 0, 0], 1, 1).unwrap()
}

fn main() {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let tray_menu = Menu::new();

    let info_item = MenuItem::new("Initializing...", false, None);
    let _ = tray_menu.append(&info_item);
    let _ = tray_menu.append(&PredefinedMenuItem::separator());

    let kill_submenu = Submenu::new("Kill/Freeze Top Offenders", true);
    let _ = tray_menu.append(&kill_submenu);

    let _ = tray_menu.append(&PredefinedMenuItem::separator());
    let emergency_purge = MenuItem::new("Emergency Purge", true, None);
    let _ = tray_menu.append(&emergency_purge);
    let _ = tray_menu.append(&PredefinedMenuItem::separator());
    let about_item = MenuItem::new("About SentinelRS...", true, None);
    let _ = tray_menu.append(&about_item);
    let quit_item = MenuItem::new("Quit SentinelRS", true, None);
    let _ = tray_menu.append(&quit_item);

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu.clone()))
        .with_title("MEM: --%")
        .with_icon(create_empty_icon())
        .build()
        .unwrap();

    let proxy_clone = proxy.clone();
    thread::spawn(move || watchdog_thread(proxy_clone));

    let menu_channel = MenuEvent::receiver();
    let mut pm = ProcessManager::new();
    let mut menu_actions: HashMap<MenuId, MenuAction> = HashMap::new();
    let mut current_submenus: Vec<Submenu> = Vec::new();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Ok(menu_event) = menu_channel.try_recv() {
            if menu_event.id == quit_item.id() {
                *control_flow = ControlFlow::Exit;
            } else if menu_event.id == about_item.id() {
                Command::new("osascript")
                    .arg("-e")
                    .arg("display dialog \"SentinelRS is a proactive macOS memory guardian.\\n\\n• Green (>20% free): Normal\\n• Yellow (10-20% free): Warning\\n• Red (<10% free): Critical (Auto-Freezes heaviest process to prevent kernel panic)\\n\\nTrend arrows (↗↘→) show pressure changes over 5 mins.\" with title \"About SentinelRS\" buttons {\"OK\"} default button \"OK\"")
                    .spawn()
                    .ok();
            } else if menu_event.id == emergency_purge.id() {
                let _ = Notification::new()
                    .title("Emergency Purge")
                    .message("Run 'sudo purge' in the terminal to reclaim inactive memory.")
                    .send();
            } else if let Some(action) = menu_actions.get(&menu_event.id) {
                match action {
                    MenuAction::Freeze(pid, name) => {
                        if pm.freeze_process(*pid) {
                            let _ = Notification::new()
                                .title("Process Frozen")
                                .message(&format!("{} (PID: {}) has been suspended.", name, pid))
                                .send();
                        }
                    }
                    MenuAction::Kill(pid, name) => {
                        if pm.kill_process(*pid) {
                            let _ = Notification::new()
                                .title("Process Killed")
                                .message(&format!("{} (PID: {}) has been terminated.", name, pid))
                                .send();
                        }
                    }
                }
            }
        }

        match event {
            Event::UserEvent(UserEvent::UpdateStatus { level, trend }) => {
                let color = if level < 10 {
                    "🔴"
                } else if level < 20 {
                    "🟡"
                } else {
                    "🟢"
                };
                tray_icon.set_title(Some(format!("{} MEM: {}% {}", color, level, trend)));
                info_item.set_text(format!("Pressure: {}%", level));
            }
            Event::UserEvent(UserEvent::UpdateTopConsumers(consumers)) => {
                // Clear existing items by removing stored submenus
                for sub in &current_submenus {
                    let _ = kill_submenu.remove(sub);
                }
                current_submenus.clear();
                menu_actions.clear();

                for (pid, name, mem) in consumers {
                    let mb = mem / 1048576;
                    let process_menu = Submenu::new(format!("{} ({} MB)", name, mb), true);
                    let _ = kill_submenu.append(&process_menu);
                    current_submenus.push(process_menu.clone());

                    let freeze_item = MenuItem::new("Freeze (SIGSTOP)", true, None);
                    let kill_item = MenuItem::new("Force Quit (SIGKILL)", true, None);

                    let _ = process_menu.append(&freeze_item);
                    let _ = process_menu.append(&kill_item);

                    menu_actions.insert(freeze_item.id().clone(), MenuAction::Freeze(pid, name.clone()));
                    menu_actions.insert(kill_item.id().clone(), MenuAction::Kill(pid, name.clone()));
                }
            }
            Event::UserEvent(UserEvent::PanicTriggered { pid, name, memory_mb }) => {
                let _ = Notification::new()
                    .title("Memory Critical!")
                    .message(&format!("Auto-Frozen {} (PID {}) using {} MB", name, pid, memory_mb))
                    .sound("Basso")
                    .send();
            }
            _ => {}
        }
    });
}

fn watchdog_thread(proxy: EventLoopProxy<UserEvent>) {
    let mut pm = ProcessManager::new();
    let mut history: VecDeque<u32> = VecDeque::with_capacity(20);

    loop {
        let level = get_memory_level().unwrap_or(100);

        history.push_back(level);
        if history.len() > 20 {
            history.pop_front();
        }

        let trend = if history.len() >= 3 {
            let old = *history.front().unwrap();
            let new = *history.back().unwrap();
            if new > old + 3 { "↗" }
            else if new < old - 3 { "↘" }
            else { "→" }
        } else { "→" };

        let _ = proxy.send_event(UserEvent::UpdateStatus { level, trend: trend.to_string() });

        pm.refresh();
        let top = pm.get_top_consumers(5);
        let _ = proxy.send_event(UserEvent::UpdateTopConsumers(top.clone()));

        if level < 10 {
            if let Some((pid, name, mem)) = top.first() {
                if pm.freeze_process(*pid) {
                    let _ = proxy.send_event(UserEvent::PanicTriggered {
                        pid: *pid,
                        name: name.clone(),
                        memory_mb: mem / 1048576,
                    });
                }
            }
        }

        let sleep_duration = if level < 20 {
            Duration::from_millis(500)
        } else {
            Duration::from_secs(2)
        };
        thread::sleep(sleep_duration);
    }
}
