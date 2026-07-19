// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! Konsole terminal-context resolver (Linux/KDE only).
//!
//! Mirrors the kitty integration, but konsole exposes no in-terminal plugin API, so
//! detection lives here in the daemon instead of an installed watcher. It reuses the
//! window-focus events the daemon already receives from KWin as its trigger, then
//! asks konsole over D-Bus which tab is active and what foreground program runs in
//! it, publishing that as the `context`/`git` variables.
//!
//! Enabled by the global `konsole_context` config flag — a plain behaviour setting,
//! not a file-installing integration. Portable: the flag deserializes on every OS
//! but only this Linux code ever acts on it.

use crate::event::DeviceEvent;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

/// Default programs of interest, matching the kitty watcher's spirit. Only these are
/// reported as `context`; anything else (including the bare shell) reports empty.
pub fn default_apps() -> Vec<String> {
    ["claude", "codex", "opencode", "mc"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Handle to the background konsole resolver. `trigger()` asks it to re-resolve the
/// focused konsole and publish `context`/`git` if they changed. On non-Linux
/// platforms it is an inert stub (no thread; `trigger()` does nothing).
pub struct KonsoleResolver {
    tx: Option<Sender<()>>,
    apps: Arc<RwLock<Vec<String>>>,
}

impl KonsoleResolver {
    /// Spawns the resolver thread (Linux only). The thread stays idle until the first
    /// `trigger()`, connecting to the session bus lazily, so it costs nothing on
    /// machines where konsole is never focused or the flag is off.
    pub fn spawn(event_tx: Sender<DeviceEvent>, apps: Vec<String>) -> Self {
        let apps = Arc::new(RwLock::new(apps));
        #[cfg(target_os = "linux")]
        {
            let (tx, rx) = std::sync::mpsc::channel::<()>();
            let apps_for_thread = apps.clone();
            std::thread::spawn(move || linux::run(rx, event_tx, apps_for_thread));
            Self {
                tx: Some(tx),
                apps,
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = event_tx;
            Self { tx: None, apps }
        }
    }

    /// Asks the resolver to re-read the focused konsole. Coalesced with any pending
    /// triggers, so a burst of focus/caption events resolves once.
    pub fn trigger(&self) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(());
        }
    }

    /// Replaces the programs-of-interest list (on config reload).
    pub fn set_apps(&self, apps: Vec<String>) {
        if let Ok(mut guard) = self.apps.write() {
            *guard = apps;
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use crate::event::{send, DeviceEvent};
    use crate::{error_log, verbose_log};
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::{Arc, RwLock};
    use zbus::blocking::{Connection, Proxy};

    pub fn run(rx: Receiver<()>, event_tx: Sender<DeviceEvent>, apps: Arc<RwLock<Vec<String>>>) {
        // zbus::blocking wants an ambient tokio runtime on this thread (same pattern
        // the KWin client uses). Both the runtime and the connection are created
        // lazily on the first trigger, so an off/idle integration costs nothing.
        let mut runtime: Option<tokio::runtime::Runtime> = None;
        let mut conn: Option<Connection> = None;
        let mut last_context: Option<String> = None;
        let mut last_git: Option<String> = None;

        while rx.recv().is_ok() {
            // Coalesce a burst of focus/caption events into a single resolve.
            while rx.try_recv().is_ok() {}

            if runtime.is_none() {
                runtime = tokio::runtime::Runtime::new()
                    .map_err(|e| error_log!("konsole: failed to create runtime: {}", e))
                    .ok();
            }
            let Some(rt) = runtime.as_ref() else { continue };
            let _guard = rt.enter();

            if conn.is_none() {
                conn = Connection::session()
                    .map_err(|e| error_log!("konsole: failed to connect to session bus: {}", e))
                    .ok();
            }
            let Some(conn) = conn.as_ref() else { continue };

            let apps_snapshot = apps.read().map(|g| g.clone()).unwrap_or_default();
            if let Some((context, git)) = resolve(conn, &apps_snapshot) {
                if last_context.as_deref() != Some(context.as_str()) {
                    verbose_log!("konsole context = '{}'", context);
                    send(
                        &event_tx,
                        DeviceEvent::SetContextVar {
                            key: "context".to_string(),
                            value: if context.is_empty() {
                                None
                            } else {
                                Some(context.clone())
                            },
                        },
                    );
                    last_context = Some(context);
                }
                if last_git.as_deref() != Some(git.as_str()) {
                    send(
                        &event_tx,
                        DeviceEvent::SetContextVar {
                            key: "git".to_string(),
                            value: if git.is_empty() { None } else { Some(git.clone()) },
                        },
                    );
                    last_git = Some(git);
                }
            }
        }
    }

    /// Finds the active konsole window's foreground program + git state. Returns
    /// `None` when no active konsole window can be resolved (e.g. a transient focus
    /// event or a D-Bus hiccup), in which case the last-published values are kept.
    fn resolve(conn: &Connection, apps: &[String]) -> Option<(String, String)> {
        for svc in konsole_services(conn) {
            for n in main_window_indices(conn, &svc) {
                if is_active_window(conn, &svc, n) != Some(true) {
                    continue;
                }
                let session = current_session(conn, &svc, n)?;
                let fg = foreground_pid(conn, &svc, session)?;
                let prog = proc_cmd_basename(fg);
                let context = if apps.iter().any(|a| a == &prog) {
                    prog
                } else {
                    String::new()
                };
                let git = if is_git_repo(&proc_cwd(fg)) { "1" } else { "" }.to_string();
                return Some((context, git));
            }
        }
        None
    }

    fn proxy<'a>(conn: &'a Connection, dest: &str, path: String, iface: &str) -> Option<Proxy<'a>> {
        Proxy::new(conn, dest.to_string(), path, iface.to_string()).ok()
    }

    /// Well-known konsole bus names: `org.kde.konsole` and `org.kde.konsole-<pid>`.
    fn konsole_services(conn: &Connection) -> Vec<String> {
        let Some(p) = proxy(
            conn,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus".to_string(),
            "org.freedesktop.DBus",
        ) else {
            return Vec::new();
        };
        let names: Vec<String> = match p.call("ListNames", &()) {
            Ok(n) => n,
            Err(_) => return Vec::new(),
        };
        names.into_iter().filter(|n| is_konsole_service(n)).collect()
    }

    fn is_konsole_service(name: &str) -> bool {
        name == "org.kde.konsole"
            || name
                .strip_prefix("org.kde.konsole-")
                .map_or(false, |s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
    }

    /// Reads the `MainWindow_N` child object indices under `/konsole` by
    /// introspection — title-free window enumeration (`MainWindow_N` ↔ `/Windows/N`).
    fn main_window_indices(conn: &Connection, svc: &str) -> Vec<u32> {
        let Some(p) = proxy(
            conn,
            svc,
            "/konsole".to_string(),
            "org.freedesktop.DBus.Introspectable",
        ) else {
            return Vec::new();
        };
        let xml: String = match p.call("Introspect", &()) {
            Ok(x) => x,
            Err(_) => return Vec::new(),
        };
        let mut out = Vec::new();
        for part in xml.split("name=\"MainWindow_").skip(1) {
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse::<u32>() {
                out.push(n);
            }
        }
        out
    }

    /// konsole-authoritative "which window has focus" — no title parsing.
    fn is_active_window(conn: &Connection, svc: &str, n: u32) -> Option<bool> {
        let p = proxy(
            conn,
            svc,
            format!("/konsole/MainWindow_{n}"),
            "org.qtproject.Qt.QWidget",
        )?;
        p.get_property::<bool>("isActiveWindow").ok()
    }

    fn current_session(conn: &Connection, svc: &str, n: u32) -> Option<i32> {
        let p = proxy(conn, svc, format!("/Windows/{n}"), "org.kde.konsole.Window")?;
        p.call::<_, _, i32>("currentSession", &()).ok()
    }

    /// Innermost foreground process group leader in the tab (ignores Ctrl-Z jobs).
    fn foreground_pid(conn: &Connection, svc: &str, session: i32) -> Option<i32> {
        let p = proxy(
            conn,
            svc,
            format!("/Sessions/{session}"),
            "org.kde.konsole.Session",
        )?;
        p.call::<_, _, i32>("foregroundProcessId", &()).ok()
    }

    /// `argv[0]` basename of a process (matches what the user typed), like the kitty
    /// watcher — so e.g. `claude` is reported rather than a truncated `comm`.
    fn proc_cmd_basename(pid: i32) -> String {
        let Ok(data) = std::fs::read(format!("/proc/{pid}/cmdline")) else {
            return String::new();
        };
        let first = data.split(|b| *b == 0).next().unwrap_or(&[]);
        if first.is_empty() {
            return String::new();
        }
        let s = String::from_utf8_lossy(first);
        std::path::Path::new(s.as_ref())
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn proc_cwd(pid: i32) -> String {
        std::fs::read_link(format!("/proc/{pid}/cwd"))
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn is_git_repo(cwd: &str) -> bool {
        if cwd.is_empty() {
            return false;
        }
        let mut path = std::path::Path::new(cwd);
        loop {
            if path.join(".git").is_dir() {
                return true;
            }
            match path.parent() {
                Some(parent) => path = parent,
                None => return false,
            }
        }
    }
}
