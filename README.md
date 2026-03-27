# SentinelRS 🛡️

**A proactive, high-performance macOS memory guardian written in Rust.**

SentinelRS lives in your macOS menu bar and actively monitors your system's memory pressure using native `sysctl` kernel hooks. Unlike passive monitors that only show you when you're already out of memory, SentinelRS intervenes *before* your Mac freezes or kernel panics by automatically suspending (`SIGSTOP`) runaway memory hogs.

## Features

- **Blazing Fast**: Written in pure Rust. Uses direct `libc` bindings for `kern.memorystatus_level`, completely avoiding the overhead of spawning shell processes like `ps` or `awk`.
- **Proactive Panic Protocol**: When the memory load exceeds 90% (Critical), SentinelRS instantly identifies the heaviest non-system process and sends a `SIGSTOP` to freeze it in its tracks, giving macOS room to breathe.
- **Native Menu UI**: Built with `tao` and `tray-icon` for a lightweight, native macOS tray icon and dropdown menu. No Electron, no web views.
- **Real-time Trend Tracking**: Analyzes memory pressure over a sliding window and displays a trend arrow (↗↘→) directly in your menu bar so you can spot leaks early.
- **One-Click Process Management**: The menu dynamically lists the top memory consumers, allowing you to manually Freeze (`SIGSTOP`) or Force Quit (`SIGKILL`) them with a single click.

## Screenshots

Here's a glimpse of SentinelRS in action, showing the menu bar icon and the dropdown menu with top memory consumers:

![SentinelRS Menu Bar UI](/images/screenshot.png)

## Installation

### Option 1: Download Ready-to-Use (Recommended)
1. Go to the [Releases](https://github.com/thusby/SentinelRS/releases) page.
2. Download and unzip `SentinelRS-macOS.zip`.
3. Drag `SentinelRS.app` to your `/Applications` folder.

> **Note on macOS Security:** Since this is an unsigned community project, macOS may say the app is "damaged" or from an "unidentified developer".
> 
> **To fix this, run the following command in Terminal:**
> ```bash
> xattr -cr /Applications/SentinelRS.app
> ```
> *Then right-click the app and select **Open**.*

### Option 2: Build from Source
You must have [Rust and Cargo](https://rustup.rs/) installed.

```bash
git clone [https://github.com/thusby/SentinelRS.git](https://github.com/thusby/SentinelRS.git)
cd SentinelRS
chmod +x build_app.sh
./build_app.sh
