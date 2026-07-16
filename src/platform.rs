// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! Platform abstraction layer.
//!
//! Exposes a uniform API for all OS-specific operations the daemon needs:
//! keyboard injection, window focus (query + request), sleep/resume
//! notification, config-reload signalling, process-exit cleanup and shell
//! selection for the `Exec` action.
//!
//! * **Linux** reuses the mature native X11 / Wayland (RemoteDesktop portal +
//!   KWin) implementations that already live in the crate root.
//! * **Windows** and **macOS** use dedicated native backends under
//!   `platform/windows` and `platform/macos`, built on the `enigo`, `windows`,
//!   `objc2` and `accessibility` crates.

// ---------------------------------------------------------------------------
// Shared, platform-independent helpers
// ---------------------------------------------------------------------------

/// Processes escape sequences in a string and returns the actual characters.
///
/// Supported escape sequences: `\n` (Enter), `\t` (Tab), `\r` (carriage
/// return), `\\` (backslash), `\e` (Escape). Used both for keyboard input and
/// for text rendering, so it lives in the platform-independent layer.
pub fn process_escape_sequences(text: &str) -> Vec<char> {
    let mut result = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next_ch) = chars.peek() {
                match next_ch {
                    'n' => {
                        chars.next();
                        result.push('\n');
                    }
                    't' => {
                        chars.next();
                        result.push('\t');
                    }
                    'r' => {
                        chars.next();
                        result.push('\r');
                    }
                    '\\' => {
                        chars.next();
                        result.push('\\');
                    }
                    'e' => {
                        chars.next();
                        result.push('\x1b');
                    }
                    _ => result.push(ch),
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

// Daemon lifecycle management (install/uninstall/start/stop/restart/status/reload).
pub mod lifecycle;

/// External context-variable control socket. Unix (Linux + macOS) only; a no-op on
/// Windows, where the socket channel is not yet implemented.
#[cfg(unix)]
pub fn spawn_context_listener(
    tx: &std::sync::mpsc::Sender<crate::event::DeviceEvent>,
    active: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    crate::listener_context::spawn_context_listener(tx, active);
}

#[cfg(not(unix))]
pub fn spawn_context_listener(
    _tx: &std::sync::mpsc::Sender<crate::event::DeviceEvent>,
    _active: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
}

// ---------------------------------------------------------------------------
// Linux backend — delegate to the existing native implementations
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod linux_glue {
    use crate::event::DeviceEvent;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc::Sender;
    use std::sync::Arc;

    pub use crate::focus_property::set_focus;
    pub use crate::keyboard::{send_key_combination, send_string};

    pub fn spawn_focus_listener(tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>) {
        crate::listener_focus::listener_focus(tx, active);
    }

    pub fn spawn_sleep_listener(
        tx: &Sender<DeviceEvent>,
        active: &Arc<AtomicBool>,
        should_reset: &Arc<AtomicBool>,
    ) {
        crate::listener_sleep::listener_sleep(tx, active, should_reset);
    }

    /// Reload (SIGHUP) + exit (SIGINT/SIGTERM) signalling.
    pub fn spawn_control_listener(tx: &Sender<DeviceEvent>, _active: &Arc<AtomicBool>) {
        crate::listener_signal::listener_signal(tx);
    }

    /// Cleanup performed right before the event loop exits.
    pub fn on_exit_cleanup() {
        crate::kwin_script::KWinScriptClient::cleanup_stale_scripts_static();
    }

    /// Shell used to run `Exec` actions: `(program, single-argument flag)`.
    pub fn exec_shell() -> (&'static str, &'static str) {
        ("bash", "-c")
    }
}

#[cfg(target_os = "linux")]
pub use linux_glue::*;

// ---------------------------------------------------------------------------
// Windows / macOS backends
//
// Keyboard injection (`keymap`) and the config-reload / exit control listener
// (`reload`) are identical across both and live in shared modules. Each OS
// module only provides what genuinely differs: window focus (query + set),
// sleep/resume notification, exit cleanup and the `Exec` shell.
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "linux"))]
pub mod keymap;
#[cfg(not(target_os = "linux"))]
mod reload;

#[cfg(not(target_os = "linux"))]
pub use keymap::{send_key_combination, send_string};

#[cfg(not(target_os = "linux"))]
pub fn spawn_control_listener(
    tx: &std::sync::mpsc::Sender<crate::event::DeviceEvent>,
    active: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    reload::spawn_control_listener(tx.clone(), active.clone());
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::{
    exec_shell, on_exit_cleanup, set_focus, spawn_focus_listener, spawn_sleep_listener,
};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{
    exec_shell, on_exit_cleanup, run_main_thread, set_focus, spawn_focus_listener,
    spawn_sleep_listener,
};
