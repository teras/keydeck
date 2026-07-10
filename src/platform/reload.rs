// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! Config-reload and exit signalling for the Windows and macOS backends.
//!
//! Linux uses UNIX signals (SIGHUP = reload, SIGINT/SIGTERM = exit). Those do
//! not exist on Windows and are awkward on macOS GUI processes, so instead:
//!
//! * **Reload** is driven by watching `config.yaml` for modifications with the
//!   `notify` crate (equivalent to a manual SIGHUP whenever the file changes).
//! * **Exit** is driven by a `ctrlc` handler covering Ctrl-C / termination.

use crate::event::{send, DeviceEvent};
use crate::{error_log, verbose_log};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Spawns both the config watcher (reload) and the exit handler.
pub fn spawn_control_listener(tx: Sender<DeviceEvent>, active: Arc<AtomicBool>) {
    spawn_config_watcher(tx.clone(), active);
    spawn_exit_handler(tx);
}

/// Watches the configuration file and emits [`DeviceEvent::Reload`] on change.
fn spawn_config_watcher(tx: Sender<DeviceEvent>, active: Arc<AtomicBool>) {
    let config_path = keydeck::get_config_path();
    let watch_dir = config_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| config_path.clone());

    thread::spawn(move || {
        let (watch_tx, watch_rx) = channel::<notify::Result<Event>>();
        let mut watcher = match RecommendedWatcher::new(watch_tx, notify::Config::default()) {
            Ok(w) => w,
            Err(e) => {
                error_log!("Failed to create config file watcher: {}", e);
                return;
            }
        };

        // Watch the containing directory: editors and the config UI often
        // replace the file atomically (write temp + rename), which would break
        // a watch bound to the file inode itself.
        if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::NonRecursive) {
            error_log!("Failed to watch config directory {:?}: {}", watch_dir, e);
            return;
        }

        verbose_log!("Watching {:?} for configuration changes", config_path);

        // Debounce: editors emit bursts of events for a single save.
        let mut last_reload = Instant::now() - Duration::from_secs(10);
        let debounce = Duration::from_millis(300);

        while active.load(Ordering::Relaxed) {
            match watch_rx.recv_timeout(Duration::from_millis(500)) {
                Ok(Ok(event)) => {
                    let touches_config = event.paths.iter().any(|p| p == &config_path);
                    let is_change = matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                    );
                    if touches_config
                        && is_change
                        && last_reload.elapsed() >= debounce
                        && config_path.exists()
                    {
                        last_reload = Instant::now();
                        verbose_log!("Configuration file changed, triggering reload");
                        send(&tx, DeviceEvent::Reload);
                    }
                }
                Ok(Err(e)) => error_log!("Config watch error: {}", e),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        verbose_log!("Config watcher thread exiting");
    });
}

/// Installs a Ctrl-C / termination handler that emits [`DeviceEvent::Exit`].
fn spawn_exit_handler(tx: Sender<DeviceEvent>) {
    if let Err(e) = ctrlc::set_handler(move || {
        let _ = tx.send(DeviceEvent::Exit);
    }) {
        error_log!("Failed to install exit handler: {}", e);
    }
}
