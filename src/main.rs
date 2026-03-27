mod memory;
mod process;

use std::collections::{HashMap, VecDeque};
use std::process::Command;
use std::thread;
use std::time::Duration;

use mac_notification_sys::Notification;
use muda::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use sysinfo::Pid;
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tray_icon::{Icon, TrayIconBuilder};

use crate::memory::get_memory_level;
use crate::process::ProcessManager;

/// Custom user events for the application's event loop.
/// These events are sent from the watchdog thread to the main (UI) thread.
#[derive(Debug)]
enum UserEvent {
    /// Updates the memory status displayed in the tray icon and info item.
    ///
    /// # Fields
    /// * `level` - The current memory pressure level (0-100, where 0 is critical, 100 is free).
    /// * `trend` - A string representing the memory trend (e.g., "↗", "↘", "→").
    UpdateStatus { level: u32, trend: String },
    /// Updates the list of top memory-consuming processes in the "Kill/Freeze Top Offenders" submenu.
    ///
    /// # Fields
    /// * `Vec<(Pid, String, u64)>` - A vector of tuples containing process PID, name, and memory usage in bytes.
    UpdateTopConsumers(Vec<(Pid, String, u64)>),
    /// Triggers a notification when the Panic Protocol automatically freezes a process.
    ///
    /// # Fields
    /// * `pid` - The PID of the process that was frozen.
    /// * `name` - The name of the process that was frozen.
    /// * `memory_mb` - The memory usage of the frozen process in MB.
    PanicTriggered {
        pid: Pid,
        name: String,
        memory_mb: u64,
    },
}

/// Actions that can be performed from the menu on a specific process.
#[derive(Debug, Clone)]
enum MenuAction {
    /// Freezes a process by sending a `SIGSTOP` signal.
    Freeze(Pid, String),
    /// Kills a process by sending a `SIGKILL` signal.
    Kill(Pid, String),
}

/// Creates a transparent 1x1 icon for the tray icon,
/// allowing the title text to be the primary visual indicator.
fn create_empty_icon() -> Icon {
    Icon::from_rgba(vec![0, 0, 0, 0], 1, 1).unwrap()
}

/// Main function that sets up the UI event loop, tray icon, and spawns the watchdog thread.
fn main() {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // --- Tray Menu Setup ---
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

    // --- Tray Icon Setup ---
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu.clone()))
        .with_title("MEM: --%") // Initial title
        .with_icon(create_empty_icon())
        .build()
        .unwrap();

    // --- Spawn Watchdog Thread ---
    // The watchdog thread runs in the background, monitoring memory and sending updates to the UI.
    let proxy_clone = proxy.clone();
    thread::spawn(move || watchdog_thread(proxy_clone));

    // --- UI Event Loop (Main Thread) ---
    // This loop handles all UI-related events and updates.
    let menu_channel = MenuEvent::receiver();
    let pm = ProcessManager::new(); // ProcessManager for handling manual menu actions
    let mut menu_actions: HashMap<MenuId, MenuAction> = HashMap::new();
    let mut current_submenus: Vec<Submenu> = Vec::new(); // To manage dynamic submenus for processes

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait; // Keep the application running and waiting for events

        // Handle menu events (clicks on tray icon menu items)
        if let Ok(menu_event) = menu_channel.try_recv() {
            if menu_event.id == quit_item.id() {
                *control_flow = ControlFlow::Exit; // Exit the application
            } else if menu_event.id == about_item.id() {
                // Display an "About" dialog using osascript
                Command::new("osascript")
                    .arg("-e")
                    .arg("display dialog \\\"SentinelRS is a proactive macOS memory guardian.\\\\n\\\\n• Green (<80% load): Normal\\\\n• Yellow (80-90% load): Warning\\\\n• Red (>90% load): Critical (Auto-Freezes heaviest process to prevent kernel panic)\\\\n\\\\nTrend arrows (↗↘→) show load changes over 5 mins.\\\" with title \\\"About SentinelRS\\\" buttons {\\\"OK\\\"} default button \\\"OK\\\"")
                    .spawn()
                    .ok();
            } else if menu_event.id == emergency_purge.id() {
                // Notify user about "sudo purge" command
                let _ = Notification::new()
                    .title("Emergency Purge")
                    .message("Run 'sudo purge' in the terminal to reclaim inactive memory.")
                    .send();
            } else if let Some(action) = menu_actions.get(&menu_event.id) {
                // Handle dynamic process-specific actions (Freeze/Kill)
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

        // Handle custom user events sent from the watchdog thread
        match event {
            Event::UserEvent(UserEvent::UpdateStatus { level, trend }) => {
                // Update tray icon title and info item text based on memory level and trend
                let color = if level < 10 { // Critical (less than 10% free)
                    "🔴"
                } else if level < 20 { // Warning (10-20% free)
                    "🟡"
                } else { // Normal (more than 20% free)
                    "🟢"
                };
                let used = 100_u32.saturating_sub(level); // Convert free level to used percentage
                tray_icon.set_title(Some(format!("{} MEM: {}% {}", color, used, trend)));
                info_item.set_text(format!("Memory Load: {}% ({}% Free)", used, level));
            }
            Event::UserEvent(UserEvent::UpdateTopConsumers(consumers)) => {
                // Clear existing dynamic process submenus
                for sub in &current_submenus {
                    let _ = kill_submenu.remove(sub);
                }
                current_submenus.clear();
                menu_actions.clear();

                // Add new dynamic submenus for top memory consumers
                for (pid, name, mem) in consumers {
                    let mb = mem / 1048576; // Convert bytes to megabytes
                    let process_menu = Submenu::new(format!("{} ({} MB)", name, mb), true);
                    let _ = kill_submenu.append(&process_menu);
                    current_submenus.push(process_menu.clone());

                    let freeze_item = MenuItem::new("Freeze (SIGSTOP)", true, None);
                    let kill_item = MenuItem::new("Force Quit (SIGKILL)", true, None);

                    let _ = process_menu.append(&freeze_item);
                    let _ = process_menu.append(&kill_item);

                    // Store menu item IDs to map back to process actions
                    menu_actions.insert(freeze_item.id().clone(), MenuAction::Freeze(pid, name.clone()));
                    menu_actions.insert(kill_item.id().clone(), MenuAction::Kill(pid, name.clone()));
                }
            }
            Event::UserEvent(UserEvent::PanicTriggered { pid, name, memory_mb }) => {
                // Send a notification when the Panic Protocol activates
                let _ = Notification::new()
                    .title("Memory Critical!")
                    .message(&format!("Auto-Frozen {} (PID {}) using {} MB", name, pid, memory_mb))
                    .sound("Basso") // Distinct sound for critical alerts
                    .send();
            }
            _ => {} // Ignore other events
        }
    });
}

/// The watchdog thread continuously monitors memory pressure, identifies top consumers,
/// and sends updates and panic triggers to the main (UI) thread via an `EventLoopProxy`.
///
/// This thread runs in an infinite loop and is independent of UI blocking.
///
/// # Arguments
///
/// * `proxy` - An `EventLoopProxy` used to send `UserEvent`s to the main thread.
fn watchdog_thread(proxy: EventLoopProxy<UserEvent>) {
    let mut pm = ProcessManager::new();
    let mut history: VecDeque<u32> = VecDeque::with_capacity(20); // History of memory usage for trend calculation

    loop {
        let level = get_memory_level().unwrap_or(100); // Get memory free level (0-100), default to 100 if error
        let used = 100_u32.saturating_sub(level); // Calculate memory used percentage

        // Update memory usage history
        history.push_back(used);
        if history.len() > 20 {
            history.pop_front();
        }

        // Determine memory trend based on historical data
        let trend = if history.len() >= 3 {
            let old = *history.front().unwrap(); // Oldest recorded usage
            let new = *history.back().unwrap(); // Newest recorded usage
            if new > old + 3 {
                // Significant increase in used memory
                "↗"
            } else if new < old - 3 {
                // Significant decrease in used memory
                "↘"
            } else {
                // Stable
                "→"
            }
        } else {
            "→" // Default trend if not enough history
        };

        // Send memory status update to the UI
        let _ = proxy.send_event(UserEvent::UpdateStatus {
            level,
            trend: trend.to_string(),
        });

        pm.refresh(); // Refresh process information
        let top = pm.get_top_consumers(5); // Get top 5 memory consumers
        let _ = proxy.send_event(UserEvent::UpdateTopConsumers(top.clone())); // Send top consumers to UI

        // --- Panic Protocol Logic ---
        // If memory free level drops below 10% (i.e., used > 90%), trigger panic protocol.
        if level < 10 {
            if let Some((pid, name, mem)) = top.first() {
                // Attempt to freeze the top memory-consuming process
                if pm.freeze_process(*pid) {
                    let _ = proxy.send_event(UserEvent::PanicTriggered {
                        pid: *pid,
                        name: name.clone(),
                        memory_mb: mem / 1048576, // Convert bytes to megabytes
                    });
                }
            }
        }

        // Adjust sleep duration based on memory pressure
        let sleep_duration = if level < 20 {
            // If memory free is less than 20% (used > 80%), poll more frequently
            Duration::from_millis(500)
        } else {
            // Otherwise, poll less frequently
            Duration::from_secs(2)
        };
        thread::sleep(sleep_duration);
    }
}
