---
name: Feature: Global Hotkey for Freezing Top App
about: Implement a global hotkey (e.g., Cmd+Shift+Escape) to immediately trigger a freeze of the top memory-consuming application.
title: 'Feature: Global Hotkey'
labels: 'enhancement, good first issue'
assignees: ''

---

## Feature Request: Global Hotkey for Freezing Top Application

**Is your feature request related to a problem? Please describe.**
Currently, users need to click the tray icon, navigate the menu, and then select a process to freeze. This can be cumbersome when an application is rapidly consuming memory and immediate action is required. A global hotkey would provide a much faster and more responsive way to mitigate runaway processes.

**Describe the solution you'd like**
I'd like to implement a global hotkey, such as `Cmd+Shift+Escape`, that when pressed, immediately identifies the top memory-consuming application (excluding whitelisted and system processes) and sends a `SIGSTOP` signal to pause it.

**Describe alternatives you've considered**
The current method of using the menu bar icon is the only alternative, but it's not as quick or convenient as a hotkey.

**Pointers for possible solution:**

*   **`tao` for Global Hotkey:** The `tao` library, already in use for windowing and event loops, has capabilities for registering global hotkeys. You would need to investigate its API for `GlobalHotKey` or similar functionality.
*   **Integrating with Watchdog Thread Logic:** When the hotkey is triggered, the main thread would need to communicate with the `ProcessManager` (likely via the existing event proxy mechanism) to request identification and freezing of the top process.
*   **Process Identification:** The logic for identifying the "top app" is already present in the `ProcessManager` for the Panic Protocol. This can be reused, potentially with minor adjustments to ensure it targets the actively focused application or simply the absolute top memory hog at that moment.
*   **Error Handling/User Feedback:** Consider how to handle cases where no suitable process can be frozen or if the hotkey fails. A subtle notification via `mac-notification-sys` could inform the user of the action taken or any issues.
*   **Configuration (Future):** While not part of this initial feature, consider how the hotkey might be customizable in a future `config.toml`. For now, a hardcoded hotkey is acceptable.