# SentinelRS 🛡️

**A proactive, high-performance macOS memory guardian written in Rust.**

SentinelRS lives in your macOS menu bar and actively monitors your system's memory pressure using native `sysctl` kernel hooks. Unlike passive monitors that only show you when you're already out of memory, SentinelRS intervenes *before* your Mac freezes or kernel panics by automatically suspending (`SIGSTOP`) runaway memory hogs.

## Features

- **Blazing Fast**: Written in pure Rust. Uses direct `libc` bindings for `kern.memorystatus_level`, completely avoiding the overhead of spawning shell processes like `ps` or `awk`.
- **Proactive Panic Protocol**: When the memory load exceeds 90% (Critical), SentinelRS instantly identifies the heaviest non-system process and sends a `SIGSTOP` to freeze it in its tracks, giving macOS room to breathe.
- **Native Menu UI**: Built with `tao` and `muda` for a lightweight, native macOS tray icon and dropdown menu. No Electron, no web views.
- **Real-time Trend Tracking**: Analyzes memory pressure over a sliding 5-minute window and displays a trend arrow (↗↘→) directly in your menu bar so you can spot leaks early.
- **One-Click Process Management**: The menu dynamically lists the top memory consumers, allowing you to manually Freeze (`SIGSTOP`) or Force Quit (`SIGKILL`) them with a single click.

## How It Works

SentinelRS uses an adaptive background watchdog thread:
* **Normal State (Green 🟢 <80% load)**: Checks memory pressure every 2 seconds.
* **Warning State (Yellow 🟡 80-90% load)**: Increases polling rate to every 500ms to monitor volatile situations closely.
* **Critical State (Red 🔴 >90% load)**: Triggers the Panic Protocol. Sends a critical macOS notification, finds the heaviest app (excluding whitelisted system tasks), and suspends it immediately.

## Installation

You must have [Rust and Cargo](https://rustup.rs/) installed to build the application.

```bash
git clone https://github.com/thusby/SentinelRS.git
cd SentinelRS
./build_app.sh
```

This will compile the release binary and bundle it into a native macOS Application (`SentinelRS.app`) in the `target/release/` directory.

### To install permanently:
1. Drag `target/release/SentinelRS.app` to your `/Applications` folder.
2. Open it! (It will run silently in your menu bar).
3. *Optional:* Add it to your **Login Items** in macOS System Settings so it starts automatically on boot.

## Safety & Whitelisting

SentinelRS includes a built-in safety whitelist. It will **never** attempt to auto-freeze or kill core macOS system processes. The current whitelist includes:
`kernel_task`, `launchd`, `WindowServer`, `SystemUIServer`, `Dock`, `Finder`, `loginwindow`, and `sysmond`.