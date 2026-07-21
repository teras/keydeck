// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::event::DeviceEvent;
use crate::kwin_script::KWinScriptClient;
use crate::{error_log, verbose_log};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Try to run the Wayland/KWin focus listener. Returns true if it ran successfully for a while,
/// false if it failed to start.
pub fn try_wayland_listener(tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>) -> bool {
    // Create KWin script client
    let mut client = match KWinScriptClient::new() {
        Ok(c) => c,
        Err(e) => {
            error_log!("Failed to create KWin script client: {}", e);
            return false;
        }
    };

    // Start the event listener. Retry a few times before giving up: a transient
    // proof-of-life miss (e.g. a busy compositor at boot) must not make us return
    // false, which would send the orchestrator to the X11 fallback that stalls under
    // Wayland. A genuine "no KWin" failure still errors out quickly each attempt.
    let receiver = {
        let mut attempt = 0;
        loop {
            match client.start_focus_listener() {
                Ok(r) => break r,
                Err(e) => {
                    attempt += 1;
                    if attempt >= 3 {
                        error_log!("Failed to start KWin focus listener after {} attempts: {}", attempt, e);
                        let _ = client.stop_focus_listener();
                        return false;
                    }
                    verbose_log!("KWin focus listener start attempt {} failed ({}), retrying...", attempt, e);
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        }
    };

    verbose_log!("KWin focus listener started");
    let mut last_event = Instant::now();
    let mut last_health_check = Instant::now();

    // Event loop - blocks until events arrive (no polling!)
    while active.load(Ordering::Relaxed) {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(window) => {
                last_event = Instant::now();
                // Only send events for valid windows
                if !window.class.is_empty() && window.class != "<no class>" {
                    verbose_log!("Focus changed: {} - {}", window.class, window.title);

                    if let Err(e) = tx.send(DeviceEvent::FocusChanges {
                        class: window.class,
                        title: window.title,
                    }) {
                        error_log!("Failed to send focus change event: {}", e);
                        break;
                    }
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Timeout is normal - allows checking 'active' flag.
                // Health check: a KWin restart silently drops all dynamically-loaded
                // scripts, so our listener would go quiet forever. Poll cheaply every
                // 10s and re-install promptly (via the orchestrator retry) if our
                // script vanished, instead of waiting for the hour-long watchdog.
                if last_health_check.elapsed() > Duration::from_secs(10) {
                    last_health_check = Instant::now();
                    if !client.is_listener_loaded() {
                        error_log!("KWin focus listener script vanished (KWin restart?) - restarting listener");
                        break;
                    }
                }
                // Backstop watchdog for the pathological case where the script is still
                // loaded but silently delivering nothing.
                if last_event.elapsed() > Duration::from_secs(3600) {
                    error_log!("No focus events received for 1 hour - restarting listener");
                    break;
                }
                continue;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                error_log!("KWin focus listener disconnected");
                break;
            }
        }
    }

    // Explicitly stop and clean up the KWin script
    if let Err(e) = client.stop_focus_listener() {
        error_log!("Failed to stop KWin focus listener: {}", e);
    }

    verbose_log!("KWin focus listener stopped");
    true
}
