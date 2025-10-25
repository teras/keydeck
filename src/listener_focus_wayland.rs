use crate::event::DeviceEvent;
use crate::kwin_script::KWinScriptClient;
use crate::{error_log, verbose_log};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Event-driven window focus listener for KDE Plasma/Wayland using native KWin D-Bus API
pub fn listener_focus_wayland(tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>) {
    let tx = tx.clone();
    let active = active.clone();

    thread::spawn(move || {
        verbose_log!("Starting KDE Wayland focus listener with auto-restart (event-driven via KWin D-Bus)");

        let mut restart_count = 0;
        const MAX_RESTARTS: u32 = 10;
        const RESTART_COOLDOWN: Duration = Duration::from_secs(5);
        let mut last_restart = Instant::now();

        // Main restart loop
        while active.load(Ordering::Relaxed) {
            // Create KWin script client
            let client = match KWinScriptClient::new() {
                Ok(c) => c,
                Err(e) => {
                    error_log!("Failed to create KWin script client: {}", e);
                    error_log!("Make sure you're running on KDE Plasma Wayland");
                    error_log!("Will retry in 30 seconds...");
                    thread::sleep(Duration::from_secs(30));
                    continue;
                }
            };

            // Start the event listener
            let receiver = match client.start_focus_listener() {
                Ok(r) => r,
                Err(e) => {
                    error_log!("Failed to start KWin focus listener: {}", e);
                    error_log!("Will retry in 30 seconds...");
                    thread::sleep(Duration::from_secs(30));
                    continue;
                }
            };

            verbose_log!("KWin focus listener started (restart count: {})", restart_count);
            let mut last_event = Instant::now();

            // Event loop - blocks until events arrive (no polling!)
            let mut should_restart = false;
            while active.load(Ordering::Relaxed) {
                match receiver.recv_timeout(Duration::from_millis(100)) {
                    Ok(window) => {
                        last_event = Instant::now();
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
                        // Also check if we've gone too long without events (watchdog)
                        // This helps detect if KWin script silently stopped working
                        if last_event.elapsed() > Duration::from_secs(3600) {
                            error_log!("No focus events received for 1 hour - restarting listener");
                            should_restart = true;
                            break;
                        }
                        continue;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        error_log!("KWin focus listener disconnected");
                        should_restart = true;
                        break;
                    }
                }
            }

            // Explicitly stop and clean up the KWin script before restarting
            if let Err(e) = client.stop_focus_listener() {
                error_log!("Failed to stop KWin focus listener: {}", e);
            }

            // Check if we should restart or exit
            if !active.load(Ordering::Relaxed) {
                verbose_log!("KWin focus listener stopped (shutdown requested)");
                break;
            }

            if should_restart {
                // Implement restart backoff to avoid tight restart loop
                let time_since_last_restart = last_restart.elapsed();
                if time_since_last_restart < RESTART_COOLDOWN {
                    restart_count += 1;
                    if restart_count >= MAX_RESTARTS {
                        error_log!("KWin focus listener failed too many times ({} restarts in {}s), giving up",
                                 restart_count, time_since_last_restart.as_secs());
                        break;
                    }
                    verbose_log!("Waiting {} seconds before restart attempt {}...",
                               RESTART_COOLDOWN.as_secs(), restart_count + 1);
                    thread::sleep(RESTART_COOLDOWN);
                } else {
                    // Reset restart count if enough time has passed
                    restart_count = 0;
                }
                last_restart = Instant::now();

                verbose_log!("Restarting KWin focus listener...");
                continue;
            }

            break;
        }

        verbose_log!("KWin focus listener thread exiting");
    });
}
