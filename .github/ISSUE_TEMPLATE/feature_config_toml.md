---
name: Feature: Config File for User Customization
about: Implement a `config.toml` file to allow users to customize their own whitelist and other application settings without recompiling.
title: 'Feature: Configurable Settings via config.toml'
labels: 'enhancement, good first issue'
assignees: ''

---

## Feature Request: Config File for User Customization

**Is your feature request related to a problem? Please describe.**
Currently, any customization to the application's behavior, such as whitelisting processes, requires modifying the source code and recompiling SentinelRS. This is not user-friendly and prevents non-developers from easily tailoring the application to their specific needs.

**Describe the solution you'd like**
I'd like to implement a `config.toml` file that allows users to define custom settings. Initially, this file should support:

*   **Whitelist Configuration:** A list of process names or PIDs that SentinelRS should ignore when identifying memory hogs. This would prevent critical applications from being unintentionally frozen or killed.
*   **Thresholds:** Potentially allow users to adjust the 80% and 90% memory load thresholds for ramping up polling and triggering the Panic Protocol, respectively. (Though whitelist is the primary goal for this issue).

**Describe alternatives you've considered**
Hardcoding values in `src/process.rs` or other files, but this requires recompilation, which is not a viable long-term solution for user customization.

**Pointers for possible solution:**

*   **`serde` and `toml` crates:** Rust has excellent ecosystem support for configuration files. The `serde` crate can be used for serializing/deserializing Rust structs to/from various formats, and the `toml` crate specifically handles TOML files.
*   **Configuration Struct:** Define a Rust `struct` that mirrors the desired configuration options (e.g., `Config { whitelist: Vec<String>, ... }`).
*   **Loading Mechanism:** At application startup (likely in `src/main.rs`), attempt to load `config.toml` from a known location (e.g., `~/.config/SentinelRS/config.toml` or `~/Library/Application Support/SentinelRS/config.toml` for macOS specific conventions). If the file doesn't exist, a default configuration should be used, and optionally, a default `config.toml` could be written to the expected location.
*   **Error Handling:** Gracefully handle cases where the `config.toml` file is missing, malformed, or has invalid values.
*   **Integration with `ProcessManager`:** The `ProcessManager` in `src/process.rs` will need to be updated to accept and utilize the loaded whitelist configuration.
*   **Watchdog Thread Access:** If thresholds are made configurable, the `Watchdog Thread` would also need access to these values.