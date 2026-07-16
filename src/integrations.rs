// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! External terminal/app integrations that feed keydeck context variables.
//!
//! An integration is an optional add-on installed into a third-party app (e.g. a
//! kitty watcher) that calls `keydeck --set context=…`. Managed via the CLI verb
//! `keydeck --integration <name> <install|uninstall|status>` and, on top of that,
//! a toggle in the config UI. All install/file logic lives here (the single source
//! of truth), mirroring how `platform::lifecycle` owns the daemon-service logic.

#[cfg(target_os = "linux")]
pub mod kitty;

/// Available integration names, for help/error text.
pub const NAMES: &str = "kitty";

/// The actions accepted after an integration name.
pub const ACTIONS: &str = "install, uninstall, status";

/// Runs `--integration <name> <action>`, returning the process exit code.
pub fn run(name: &str, action: &str) -> i32 {
    #[cfg(target_os = "linux")]
    {
        match name {
            "kitty" => kitty::run(action),
            other => {
                crate::error_log!("Unknown integration '{}'. Available: {}", other, NAMES);
                1
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (name, action);
        crate::error_log!("Integrations are currently only supported on Linux");
        1
    }
}

/// Refreshes any installed integration's payload to match this binary (so the
/// watcher script can never drift behind the daemon's socket protocol). Called
/// once at daemon startup; a no-op when nothing is installed.
pub fn refresh_installed() {
    #[cfg(target_os = "linux")]
    kitty::refresh_if_installed();
}
