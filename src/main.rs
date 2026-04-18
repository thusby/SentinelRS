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
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
use tray_icon::{Icon, TrayIconBuilder};

use crate::memory::get_memory_level;
use crate::process::{DecisionOutcome, DecisionResult, ProcessFeatureVector, ProcessManager};

/// Custom user events for the application's event loop.
#[derive(Debug)]
enum UserEvent {
    /// Updates the memory status displayed in the tray icon and info item.
    UpdateStatus { level: u32, trend: String },
    /// Updates the list of top memory-consuming processes in the "Kill/Freeze Top Offenders" submenu.
    UpdateTopConsumers(Vec<(Pid, String, u64)>),
    /// Triggers a notification when the Panic Protocol automatically freezes a process.
    PanicTriggered {
        pid: Pid,
        name: String,
        memory_mb: u64,
    },
}

/// Actions that can be performed from the menu on a specific process.
#[derive(Debug, Clone)]
enum MenuAction {
    Freeze(Pid, String),
    Kill(Pid, String),
}

/// Creates a transparent 1x1 icon for the tray icon, allowing the title text to be the primary visual indicator.
fn create_empty_icon() -> Icon {
    Icon::from_rgba(vec![0, 0, 0, 0], 1, 1).expect("failed to create 1x1 transparent tray icon")
}

/// Main function that sets up the UI event loop, tray icon, and spawns the watchdog thread.
fn main() {
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    // Attempt to suppress the Dock icon (known limitation on macOS 26)
    event_loop.set_activation_policy(ActivationPolicy::Accessory);
    event_loop.set_dock_visibility(false);
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
    let proxy_clone = proxy.clone();
    thread::spawn(move || watchdog_thread(proxy_clone));

    // --- UI Event Loop (Main Thread) ---
    let menu_channel = MenuEvent::receiver();

    // ProcessManager for manual menu actions and ML decision making.
    let mut pm = ProcessManager::new();
    let mut menu_actions: HashMap<MenuId, MenuAction> = HashMap::new();
    let mut current_dynamic_menu_items: Vec<MenuItem> = Vec::new();
    // Track PIDs from the last menu build to skip unnecessary rebuilds.
    let mut prev_consumer_pids: Vec<Pid> = Vec::new();

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait; // Keep the application running and waiting for events

        // Handle menu events (clicks on tray icon menu items)
        if let Ok(menu_event) = menu_channel.try_recv() {
            if menu_event.id == quit_item.id() {
                *control_flow = ControlFlow::Exit; // Exit the application
            } else if menu_event.id == about_item.id() {
                // Display an "About" dialog using osascript.
                Command::new("osascript")
                    .arg("-e")
                    .arg("display dialog \"SentinelRS is a proactive macOS memory guardian.\\n\\n• Green (<80% load): Normal\\n• Yellow (80-90% load): Warning\\n• Red (>90% load): Critical (Auto-Freezes heaviest process to prevent kernel panic)\\n\\nTrend arrows (↗↘→) show load changes over ~40 seconds.\" with title \"About SentinelRS\" buttons {\"OK\"} default button \"OK\"")
                    .spawn()
                    .ok();
            } else if menu_event.id == emergency_purge.id() {
                let _ = Notification::new()
                    .title("Emergency Purge")
                    .message("Run 'sudo purge' in the terminal to reclaim inactive memory.")
                    .send();
            } else if let Some(action) = menu_actions.get(&menu_event.id).cloned() {
                // Handle dynamic process-specific actions (Freeze/Kill).
                match action {
                    MenuAction::Freeze(pid, name) => {
                        pm.refresh(); // Refresh before action!
                        if pm.freeze_process(pid) {
                            let _ = Notification::new()
                                .title("Process Frozen")
                                .message(&format!("{} (PID: {}) has been suspended.", name, pid))
                                .send();
                        }
                    }
                    MenuAction::Kill(pid, name) => {
                        pm.refresh(); // Refresh before action!
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
                let color = if level < 10 {
                    "🔴" // Critical: less than 10% free
                } else if level < 20 {
                    "🟡" // Warning: 10-20% free
                } else {
                    "🟢" // Normal: more than 20% free
                };
                let used = 100_u32.saturating_sub(level);
                tray_icon.set_title(Some(format!("{} MEM: {}% {}", color, used, trend)));
                info_item.set_text(format!("Memory Load: {}% ({}% Free)", used, level));
            }
            Event::UserEvent(UserEvent::UpdateTopConsumers(consumers)) => {
                let new_pids: Vec<Pid> = consumers.iter().map(|(pid, _, _)| *pid).collect();
                if new_pids == prev_consumer_pids {
                    return; // Skip rebuild if the PID set hasn't changed
                }
                prev_consumer_pids = new_pids;

                // Clear existing dynamic menu items.
                for item in &current_dynamic_menu_items {
                    let _ = kill_submenu.remove(item);
                }
                current_dynamic_menu_items.clear();
                menu_actions.clear();

                // Add new dynamic menu items for top memory consumers (Flat list).
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

/// The watchdog thread continuously monitors memory pressure, identifies top consumers,
/// and sends updates and panic triggers to the main (UI) thread via an `EventLoopProxy`.
fn watchdog_thread(proxy: EventLoopProxy<UserEvent>) {
    let mut pm = ProcessManager::new();
    let mut history: VecDeque<u32> = VecDeque::with_capacity(20); // Sliding window for trend calculation

    loop {
        // 1. Read System State (Memory)
        let level = get_memory_level().unwrap_or(100);
        let used = 100_u32.saturating_sub(level);

        // Update memory usage history and trend
        history.push_back(used);
        if history.len() > 20 {
            history.pop_front();
        }

        let trend = if history.len() >= 3 {
            let old = *history.front().unwrap();
            let new = *history.back().unwrap();
            if new > old + 3 {
                "↗"
            } else if new < old.saturating_sub(3) {
                "↘"
            } else {
                "→"
            }
        } else {
            "→"
        };

        // Send memory status update to the UI
        let _ = proxy.send_event(UserEvent::UpdateStatus {
            level,
            trend: trend.to_string(),
        });

        // 2. Refresh and Analyze Processes
        pm.refresh(); // Update process snapshot
        let top = pm.get_top_consumers(5);

        let system_memory = pm.system.total_memory();
        let mut panic_candidate: Option<(Pid, String, u64)> = None;

        // 3. Decision Cycle Loop (ML Inference -> Fallbacks)
        for &(pid, name, mem) in top.iter() {
            // A. Feature Engineering
            let features = pm.extract_features(
                pm.system.process(pid).unwrap(),
                system_memory,
                pm.system.processes().len(),
            );

            // B. Get Decision from ML Engine (The core AI call)
            let decision: DecisionResult = pm.run_inference(&features);

            // C. Execution Logic & Fallbacks (THE DECISION ROUTER)
            let mut action_to_take: Option<&str> = None;

            // 1. High Confidence ML Prediction takes precedence
            if decision.confidence >= 0.8 {
                match decision.outcome {
                    DecisionOutcome::PredictFreeze => action_to_take = Some("FREEZE"),
                    DecisionOutcome::PredictKill => action_to_take = Some("KILL"),
                    _ => {} // Ignore if model predicts ignore, regardless of confidence
                }
            } else if level < 10 && !top.is_empty() {
                // 2. Fallback: Critical Panic Protocol (Always run this, regardless of low ML confidence)
                action_to_take = Some("FREEZE"); // Always freeze the top offender when critically low memory
            } else {
                // 3. No action required based on current heuristics or model output.
            }

            if let Some(action) = action_to_take {
                // Execute the chosen action
                match action {
                    "FREEZE" => {
                        if pm.freeze_process(pid) {
                            // Trigger a panic notification only if it was the top offender AND critically low memory (redundancy check for better logging)
                            let _ = proxy.send_event(UserEvent::PanicTriggered {
                                pid: pid,
                                name: name.clone(),
                                memory_mb: mem / 1_048_576,
                            });
                        }
                    }
                    "KILL" => {
                        if pm.kill_process(pid) {} // Log success/failure here (UI notification if required)
                    }
                    _ => {}
                }
            }

            // Store the potential panic candidate for notification purposes (always use the top consumer if critical)
            panic_candidate = Some((pid, name, mem));
        }

        let _ = proxy.send_event(UserEvent::UpdateTopConsumers(top));

        // 4. Sleep based on current memory pressure
        let sleep_duration = if level < 20 {
            Duration::from_millis(500) // High pressure: poll every 500ms
        } else {
            Duration::from_secs(2) // Normal: poll every 2s
        };
        thread::sleep(sleep_duration);
    }
}
