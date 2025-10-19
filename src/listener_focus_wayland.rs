use crate::event::DeviceEvent;
use crate::kwin_script::KWinScriptClient;
use crate::{error_log, verbose_log};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

/// Event-driven window focus listener for KDE Plasma/Wayland using native KWin D-Bus API
pub fn listener_focus_wayland(tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>) {
    let tx = tx.clone();
    let active = active.clone();

    thread::spawn(move || {
        verbose_log!("Starting KDE Wayland focus listener (event-driven via KWin D-Bus)");

    // Create KWin script client
    let client = match KWinScriptClient::new() {
        Ok(c) => c,
        Err(e) => {
            error_log!("Failed to create KWin script client: {}", e);
            error_log!("Make sure you're running on KDE Plasma Wayland");
            return;
        }
    };

    // Start the event listener
    let receiver = match client.start_focus_listener() {
        Ok(r) => r,
        Err(e) => {
            error_log!("Failed to start KWin focus listener: {}", e);
            return;
        }
    };

    verbose_log!("KWin focus listener started (event-driven - zero CPU when idle)");

    // Event loop - blocks until events arrive (no polling!)
    while active.load(Ordering::Relaxed) {
        match receiver.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(window) => {
                // Only send events for valid windows
                if !window.class.is_empty() && window.class != "<no class>" {
                    verbose_log!("Focus changed: {} - {}", window.class, window.title);

                    if let Err(e) =
                        tx.send(DeviceEvent::FocusChanges { class: window.class, title: window.title })
                    {
                        error_log!("Failed to send focus change event: {}", e);
                        break;
                    }
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Timeout is normal - allows checking 'active' flag
                continue;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                error_log!("KWin focus listener disconnected");
                break;
            }
        }
    }

        verbose_log!("KWin focus listener stopped");
    });
}
