# SentinelRS 🛡️

**A proactive macOS memory guardian written in Rust.**

SentinelRS lives in your menu bar and monitors memory pressure using native `sysctl` kernel hooks. When memory gets critical, it automatically suspends the heaviest runaway process before your Mac becomes unresponsive.

## Features

- **Native & Lightweight** — Pure Rust, no Electron, no web views. Direct `libc` bindings to `kern.memorystatus_level`, no shell processes spawned.
- **Panic Protocol** — At >90% memory load, automatically freezes (`SIGSTOP`) the top non-system memory consumer.
- **Trend Tracking** — Displays a trend arrow (↗↘→) in the menu bar based on a recent sliding window.
- **One-Click Actions** — Manually Freeze or Force Quit any of the top 5 memory consumers from the menu.

## Screenshots

![SentinelRS Menu Bar UI](/images/screenshot.png)

## Memory Thresholds

| Status | Free Memory | Menu Bar |
|--------|-------------|----------|
| Normal | ≥ 20% free | 🟢 MEM: XX% → |
| Warning | 10–20% free | 🟡 MEM: XX% → |
| Critical | < 10% free | 🔴 MEM: XX% ↗ |

At **Critical**, SentinelRS automatically sends `SIGSTOP` to the heaviest non-system process and posts a notification.

## Installation

### Option 1: Download (Recommended)

1. Go to the [Releases](https://github.com/thusby/SentinelRS/releases) page.
2. Download and unzip `SentinelRS-macOS.zip`.
3. Drag `SentinelRS.app` to `/Applications`.

> **macOS Security Note:** Since this app is unsigned, macOS may block it. Run this once in Terminal, then right-click the app and select **Open**:
> ```bash
> xattr -cr /Applications/SentinelRS.app
> ```

### Option 2: Build from Source

Requires [Rust and Cargo](https://rustup.rs/).

```bash
git clone https://github.com/thusby/SentinelRS.git
cd SentinelRS
chmod +x build_app.sh
./build_app.sh
```

The app bundle is created at `target/release/SentinelRS.app`. Drag it to `/Applications`.

## How It Works

1. A background watchdog polls `kern.memorystatus_level` every **2 seconds** (every **500 ms** under pressure).
2. The top 5 non-system memory consumers are listed in the **Kill/Freeze Top Offenders** submenu.
3. If memory load exceeds 90%, the Panic Protocol fires — the top offender is frozen with `SIGSTOP`, and you get a notification.
4. You can resume or manually kill processes from the menu at any time.

System processes (`kernel_task`, `WindowServer`, `Dock`, `Finder`, etc.) are always excluded from automated and manual actions.

## License

MIT — see [LICENSE](LICENSE).