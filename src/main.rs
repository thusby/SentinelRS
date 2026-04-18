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
    Icon::from_rgba(vec![0, 0, 0, 0], 1, 1).expect("failed to create 1x1 transparent tray icon")
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

    // ProcessManager for manual menu actions. Declared mut so refresh() can be called
    // immediately before each signal dispatch to avoid operating on a stale process list.
    let mut pm = ProcessManager::new();
    let mut menu_actions: HashMap<MenuId, MenuAction> = HashMap::new();
    let mut current_dynamic_menu_items: Vec<MenuItem> = Vec::new();
    // Track PIDs from the last menu build to skip unnecessary rebuilds.
    let mut prev_consumer_pids: Vec<Pid> = Vec::new();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait; // Keep the application running and waiting for events

        // Handle menu events (clicks on tray icon menu items)
        if let Ok(menu_event) = menu_channel.try_recv() {
            if menu_event.id == quit_item.id() {
                *control_flow = ControlFlow::Exit; // Exit the application
            } else if menu_event.id == about_item.id() {
                // Display an "About" dialog using osascript.
                // Quotes and newlines must be correctly escaped for the AppleScript string.
                Command::new("osascript")
                    .arg("-e")
                    .arg("display dialog \"SentinelRS is a proactive macOS memory guardian.\\n\\n• Green (<80% load): Normal\\n• Yellow (80-90% load): Warning\\n• Red (>90% load): Critical (Auto-Freezes heaviest process to prevent kernel panic)\\n\\nTrend arrows (↗↘→) show load changes over ~40 seconds.\" with title \"About SentinelRS\" buttons {\"OK\"} default button \"OK\"")
                    .spawn()
                    .ok();
            } else if menu_event.id == emergency_purge.id() {
                // Notify user about "sudo purge" command
                let _ = Notification::new()
                    .title("Emergency Purge")
                    .message("Run 'sudo purge' in the terminal to reclaim inactive memory.")
                    .send();
            } else if let Some(action) = menu_actions.get(&menu_event.id).cloned() {
                // Handle dynamic process-specific actions (Freeze/Kill).
                // The action is cloned to release the borrow on menu_actions before calling
                // pm.refresh(), which requires &mut self on a separate variable.
                // Refresh the process snapshot immediately before signaling so we never
                // send SIGSTOP/SIGKILL based on a minutes-old process list.
                match action {
                    MenuAction::Freeze(pid, name) => {
                        pm.refresh();
                        if pm.freeze_process(pid) {
                            let _ = Notification::new()
                                .title("Process Frozen")
                                .message(&format!("{} (PID: {}) has been suspended.", name, pid))
                                .send();
                        }
                    }
                    MenuAction::Kill(pid, name) => {
                        pm.refresh();
                        if pm.kill_process(pid) {
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
                let color = if level < 10 {
                    "🔴" // Critical: less than 10% free
                } else if level < 20 {
                    "🟡" // Warning: 10-20% free
                } else {
                    "🟢" // Normal: more than 20% free
                };
                let used = 100_u32.saturating_sub(level); // Convert free level to used percentage
                tray_icon.set_title(Some(format!("{} MEM: {}% {}", color, used, trend)));
                info_item.set_text(format!("Memory Load: {}% ({}% Free)", used, level));
            }
            Event::UserEvent(UserEvent::UpdateTopConsumers(consumers)) => {
                // Only rebuild the menu when the set of displayed processes has changed.
                // This prevents unnecessary UI churn every 500ms during high-pressure polling.
                let new_pids: Vec<Pid> = consumers.iter().map(|(pid, _, _)| *pid).collect();
                if new_pids == prev_consumer_pids {
                    return;
                }
                prev_consumer_pids = new_pids;

                // Clear existing dynamic menu items.
                for item in &current_dynamic_menu_items {
                    let _ = kill_submenu.remove(item);
                }
                current_dynamic_menu_items.clear();
                menu_actions.clear();

                // Add new dynamic menu items for top memory consumers in a flat structure.
                for (pid, name, mem) in consumers {
                    let mb = mem / 1_048_576; // Convert bytes to megabytes

                    // Create "Freeze" menu item directly under kill_submenu
                    let freeze_text = format!("Freeze: {} ({} MB)", name, mb);
                    let freeze_item = MenuItem::new(freeze_text, true, None);
                    let _ = kill_submenu.append(&freeze_item);
                    current_dynamic_menu_items.push(freeze_item.clone());

                    // Create "Kill" menu item directly under kill_submenu
                    let kill_text = format!("Kill: {} ({} MB)", name, mb);
                    let kill_item = MenuItem::new(kill_text, true, None);
                    let _ = kill_submenu.append(&kill_item);
                    current_dynamic_menu_items.push(kill_item.clone());

                    // Store menu item IDs to map back to process actions
                    menu_actions.insert(freeze_item.id().clone(), MenuAction::Freeze(pid, name.clone()));
                    menu_actions.insert(kill_item.id().clone(), MenuAction::Kill(pid, name));
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
    let mut history: VecDeque<u32> = VecDeque::with_capacity(20); // Sliding window for trend calculation

    loop {
        let level = get_memory_level().unwrap_or(100); // Default to 100 (fully free) on sysctl error
        let used = 100_u32.saturating_sub(level); // Convert free-memory score to used percentage

        // Update memory usage history
        history.push_back(used);
        if history.len() > 20 {
            history.pop_front();
        }

        // Determine memory trend from the sliding window.
        // saturating_sub prevents u32 underflow when `old` is less than 3.
        let trend = if history.len() >= 3 {
            let old = *history.front().unwrap(); // Oldest recorded usage
            let new = *history.back().unwrap(); // Newest recorded usage
            if new > old + 3 {
                "↗" // Significant increase in used memory
            } else if new < old.saturating_sub(3) {
                "↘" // Significant decrease in used memory
            } else {
                "→" // Stable
            }
        } else {
            "→" // Default until enough history accumulates
        };

        // Send memory status update to the UI
        let _ = proxy.send_event(UserEvent::UpdateStatus {
            level,
            trend: trend.to_string(),
        });

        pm.refresh(); // Refresh process information
        let top = pm.get_top_consumers(5); // Get top 5 memory consumers

        // Capture the panic candidate before moving `top` into the event.
        // This avoids cloning the entire Vec when only the first entry is needed.
        let panic_candidate = if level < 10 {
            top.first().cloned()
        } else {
            None
        };

        let _ = proxy.send_event(UserEvent::UpdateTopConsumers(top));

        // --- Panic Protocol Logic ---
        // If the free-memory score drops below 10 (used > 90%), freeze the top consumer.
        if let Some((pid, name, mem)) = panic_candidate {
            if pm.freeze_process(pid) {
                let _ = proxy.send_event(UserEvent::PanicTriggered {
                    pid,
                    name,
                    memory_mb: mem / 1_048_576,
                });
            }
        }

        // Adjust polling frequency based on current memory pressure
        let sleep_duration = if level < 20 {
            Duration::from_millis(500) // High pressure: poll every 500ms
        } else {
            Duration::from_secs(2) // Normal: poll every 2s
        };
        thread::sleep(sleep_duration);
    }
}
