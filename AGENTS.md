# SentinelRS - Agent/Developer Notes

This document provides technical context for agents or developers maintaining the SentinelRS repository. It is intended to reflect the current Rust implementation in this repo, not an earlier prototype.

## Current Implementation Snapshot

- SentinelRS is a native macOS menu bar utility written in Rust.
- The crate currently targets Rust Edition 2024.
- The binary crate name is `sentinel-rs`, while the packaged app bundle name is `SentinelRS`.
- The app monitors macOS memory pressure through the kernel sysctl `kern.memorystatus_level`.
- `cargo check` currently succeeds.
- There are currently dead-code warnings for `ProcessManager::get_suspended_consumers` and `ProcessManager::resume_process`.

## Architecture

SentinelRS is designed as a lightweight menu bar app that tries to intervene before the system becomes unresponsive under memory pressure.

### Core Behavior

The app reads `kern.memorystatus_level`, which is effectively a free-memory score from `0..=100`:

- `100` means plenty of free memory
- `0` means critically low memory

The UI converts that value into a "used/load" percentage as:

- `used = 100 - level`

This means the implementation reasons about both values:

- `level` = free-memory score
- `used` = user-facing memory load percentage

### Thresholds

Current behavior in code:

- Normal:
  - `level >= 20`
  - equivalent to `used <= 80`
- Warning:
  - `10 <= level < 20`
  - equivalent to `80 < used < 90`
- Critical:
  - `level < 10`
  - equivalent to `used > 90`

### Panic Protocol

When the free-memory score drops below `10`, SentinelRS:

1. refreshes process state
2. finds the top non-whitelisted memory consumer
3. sends `SIGSTOP` to that process
4. posts a critical user notification on success

This is a freeze/suspend, not a kill.

## Concurrency Model

The app is currently split into two main execution contexts.

### 1. Main Thread / UI Event Loop

Handled by `tao`.

Responsibilities:

- creates the tray icon and menu
- receives user events from the watchdog thread via `EventLoopProxy`
- updates the tray title and informational menu text
- rebuilds the dynamic process action menu
- executes manual freeze/kill actions selected from the menu
- shows notifications
- launches the About dialog through `osascript`

### 2. Watchdog Thread

A background thread spawned from `main`.

Responsibilities:

- continuously polls memory pressure
- tracks recent load history for trend display
- refreshes process data via `ProcessManager`
- computes the top memory consumers
- sends status/menu update events back to the UI thread
- triggers the Panic Protocol under critical pressure

## UI and Event Flow

### Tray UI

The tray icon is built with a transparent 1x1 icon so the title text is the primary visible indicator.

Initial title:

- `MEM: --%`

At runtime the title becomes:

- `🟢 MEM: <used>% <trend>`
- `🟡 MEM: <used>% <trend>`
- `🔴 MEM: <used>% <trend>`

Color mapping is based on free-memory score:

- green: `level >= 20`
- yellow: `10 <= level < 20`
- red: `level < 10`

### Static Menu Items

The app currently creates these top-level menu items:

- status/info item
- separator
- `Kill/Freeze Top Offenders` submenu
- separator
- `Emergency Purge`
- separator
- `About SentinelRS...`
- `Quit SentinelRS`

### Dynamic Menu Items

The `Kill/Freeze Top Offenders` submenu is rebuilt only when the set of displayed process PIDs changes. If the same five processes are still the top consumers on the next watchdog tick, the rebuild is skipped. This prevents unnecessary UI churn every 500 ms during high-pressure polling.

For each top consumer, the UI creates two flat actions:

- `Freeze: <name> (<mb> MB)`
- `Kill: <name> (<mb> MB)`

These menu entries are mapped to `MenuAction::Freeze` and `MenuAction::Kill`.

### Custom User Events

The current `UserEvent` enum contains:

- `UpdateStatus { level, trend }`
- `UpdateTopConsumers(Vec<(Pid, String, u64)>)`
- `PanicTriggered { pid, name, memory_mb }`

## Watchdog Logic

The watchdog loop currently behaves as follows:

1. read `kern.memorystatus_level`
2. default to `100` if the sysctl call fails
3. convert free-memory score to used/load percentage
4. append the used/load value to a sliding history window
5. derive a trend arrow
6. send status update to the UI
7. refresh process state
8. fetch the top 5 consumers
9. send consumer list to the UI
10. if `level < 10`, freeze the top offender and emit a panic event
11. sleep based on current pressure

### Polling Frequency

Current polling intervals:

- `2s` when `level >= 20`
- `500ms` when `level < 20`

### Trend Calculation

A `VecDeque<u32>` with capacity `20` is used as a sliding history buffer.

Trend rules:

- `↗` if newest used value is more than `3` points above the oldest
- `↘` if newest used value is more than `3` points below the oldest (`saturating_sub` guards against u32 underflow when `old < 3`)
- `→` otherwise

Until enough history exists, the default trend is `→`.

## Process Management

`ProcessManager` lives in `src/process.rs` and wraps a cached `sysinfo::System` initialized with `System::new()` (not `System::new_all()`; unused subsystems such as disk and network are never loaded).

### Current Responsibilities

- refresh process and memory data
- maintain a process-name whitelist
- return top memory consumers
- identify suspended consumers
- freeze a process with `SIGSTOP`
- resume a process with `SIGCONT`
- kill a process with `SIGKILL`

### Whitelist

The current built-in whitelist is name-based and includes:

- `kernel_task`
- `launchd`
- `WindowServer`
- `SystemUIServer`
- `Dock`
- `Finder`
- `loginwindow`
- `sysmond`
- `SentinelRS`

### Top Consumer Filtering

`get_top_consumers()` excludes:

- whitelisted process names
- processes with PID `<= 100`
- processes already in `Stop` state
- zombie processes

Results are sorted descending by memory usage.

### Signal Behavior

Current manual and automatic actions:

- freeze uses `Signal::Stop`
- resume uses `Signal::Continue`
- kill uses `Signal::Kill`

## Dependency Notes

Direct dependencies currently declared in `Cargo.toml`:

- `libc`
- `mac-notification-sys`
- `muda`
- `sysinfo`
- `tao`
- `tray-icon`

### How They Are Used

- `libc`
  - used for the `sysctlbyname` FFI call in `memory.rs`
- `sysinfo`
  - used for process enumeration, process state, memory usage, and signal delivery
- `tao`
  - drives the main event loop and custom user-event channel
- `muda`
  - builds the native menu structure
- `tray-icon`
  - creates the macOS menu bar item and binds the menu
- `mac-notification-sys`
  - sends user notifications



## File-by-File Map

### `src/main.rs`

Primary entry point.

Contains:

- app bootstrap
- tray/menu construction
- transparent tray icon setup
- watchdog thread spawn
- UI event loop
- menu event handling
- dynamic process action menu rebuild
- notification dispatch
- About dialog launch through `osascript`

### `src/memory.rs`

Unsafe FFI wrapper around:

- `sysctlbyname("kern.memorystatus_level")`

Exports:

- `get_memory_level() -> Option<u32>`

Behavior:

- returns `Some(level)` on success
- returns `None` on sysctl failure

### `src/process.rs`

Defines:

- `ProcessManager`

Implements:

- whitelist initialization
- process refresh logic
- top consumer selection
- suspended consumer listing
- freeze/resume/kill helpers

### `build_app.sh`

Build-and-package helper for macOS app bundling.

Current responsibilities:

- runs `cargo build --release`
- creates `target/release/SentinelRS.app`
- copies the release binary to `Contents/MacOS/SentinelRS`
- copies `AppIcon.icns` into `Contents/Resources/`
- writes `Contents/Info.plist`

Important bundle settings currently written by the script:

- `CFBundleIdentifier = com.thusby.sentinelrs`
- `LSUIElement = true`
- `LSMinimumSystemVersion = 10.15`

## Notifications and User Messaging

Current notification behavior:

- `Emergency Purge`
  - does not run a purge command
  - only tells the user to run `sudo purge` manually in Terminal
- manual freeze
  - sends a "Process Frozen" notification
- manual kill
  - sends a "Process Killed" notification
- panic protocol
  - sends a "Memory Critical!" notification with `Basso` sound

The About dialog is currently implemented by spawning:

- `osascript -e 'display dialog ...'`

## Maintenance Notes and Caveats

### 1. Dock Icon Visible on macOS 26 (Tahoe Beta)

On macOS 26 (Tahoe beta), the app's icon appears in the Dock while running, despite the following suppression attempts all being in place:

- `LSUIElement = true` in `Info.plist`
- `event_loop.set_activation_policy(ActivationPolicy::Accessory)` pre-run
- `event_loop.set_dock_visibility(false)` pre-run

Both `Accessory` and `Prohibited` activation policies were tested — neither suppresses the Dock icon on this OS version. This appears to be a macOS 26 behaviour change that is not yet handled by `tao 0.35.0`. The app is otherwise fully functional. Revisit when a stable macOS 26 release or a newer `tao` version is available.

### 2. Manual-action ProcessManager Is Refreshed Before Every Signal ✅

The UI thread owns a separate `ProcessManager` (`mut pm`) used exclusively for manual freeze/kill actions. Before dispatching any signal, the event handler now calls `pm.refresh()` to get a current process snapshot. This means a menu entry built from watchdog data will not try to signal a PID that has since exited or been replaced.

If the refresh fails to find the PID, `freeze_process`/`kill_process` return `false` and no notification is sent — a safe no-op.

### 3. Unused Capabilities Already Exist

`ProcessManager` already has support for:

- listing suspended processes
- resuming suspended processes

Those capabilities are not currently wired into the UI.

### 4. Process Filtering Is Conservative but Name-Based

Whitelisting is based on process name strings, not bundle ID, executable path, code signature, or parent/child relationships.

If the whitelist ever needs to become more robust, this is a likely upgrade area.

### 5. The App Is macOS-Specific in Practice

Although some UI crates are cross-platform, the current app behavior depends on macOS-specific pieces:

- `kern.memorystatus_level`
- macOS notifications
- AppleScript dialog handling
- `.app` bundle packaging

Treat the project as macOS-native.

## Recommended Guidance for Future Agents

When making changes:

1. preserve the two-thread model unless there is a strong reason to change it
2. keep the distinction between `level` (free-memory score) and `used` (UI load percentage) explicit
3. be careful with signal semantics:
   - `SIGSTOP` suspends
   - `SIGKILL` terminates
   - `SIGCONT` resumes
4. prefer validating menu behavior against `src/main.rs` rather than relying on older documentation
5. if updating thresholds or panic behavior, update:
   - tray color logic
   - About dialog text
   - README wording
   - this file
6. if cleaning warnings, inspect:
   - dead-code methods in `ProcessManager` (`get_suspended_consumers`, `resume_process`)

## Quick Reality Check

As of the current codebase:

- the Rust edition is 2024, not 2021
- the watchdog polls every 2s normally and 500ms under pressure
- the top-consumer submenu is a flat list of Freeze/Kill actions, not nested per-process menus
- the Panic Protocol freezes the single top offender when free-memory score drops below 10
- packaging is handled by `build_app.sh`, which creates a background-only macOS app bundle
- `crossbeam-channel` has been removed; it was declared but never referenced in `src/`
- `ProcessManager` is initialized with `System::new()`, not `System::new_all()`
- manual freeze/kill actions call `pm.refresh()` immediately before dispatching the signal
- the menu submenu is rebuilt only when the PID set changes, not on every watchdog tick
- `old.saturating_sub(3)` prevents u32 underflow in the trend calculation