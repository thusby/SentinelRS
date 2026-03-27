# SentinelRS - Agent/Developer Notes

This document provides technical context for AI agents or developers maintaining the SentinelRS repository.

## Architecture

SentinelRS is a native macOS application written in Rust (Edition 2021). It replaces an older bash/SwiftBar script implementation to provide lower overhead and proactive memory management.

### Key Dependencies
- `sysinfo`: Used for enumerating active processes, getting memory usage, and sending signals (`SIGSTOP`, `SIGKILL`).
- `libc`: Used to bypass command-line tools and fetch memory pressure directly from the XNU kernel via `sysctlbyname("kern.memorystatus_level")`.
- `tao` & `muda`: Cross-platform windowing and native menu bar generation.
- `tray-icon`: Hooks the `muda` menu into the macOS status bar.
- `mac-notification-sys`: Native macOS Notification Center bindings.

### Concurrency Model
The application is strictly split into two threads:
1. **Main Thread (UI/Event Loop)**: Handled by `tao`. Receives events via a proxy and updates the `tray-icon` title or the `muda` submenus. Executes manual kill/freeze actions.
2. **Watchdog Thread**: Runs an infinite loop independent of UI blocking. Samples memory pressure. If load exceeds 80% (i.e. free drops below 20%), it ramps up polling to 500ms and extracts top processes via `ProcessManager`. If load exceeds 90%, it autonomously triggers the `Panic Protocol` and sends an event back to the UI.

### Project Structure
- `src/main.rs`: Entry point, event loop, tray icon setup, and the watchdog thread.
- `src/memory.rs`: Unsafe FFI wrapper for `kern.memorystatus_level`.
- `src/process.rs`: `ProcessManager` struct caching the `sysinfo::System` instance. Handles whitelisting, sorting top memory hogs, and signal dispatching.
- `build_app.sh`: Bash script to run `cargo build --release` and package the resulting binary into a valid `.app` bundle with an `Info.plist` (specifically injecting `LSUIElement=true` so the app doesn't show in the Dock).

