// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::event::{send, DeviceEvent};
use crate::session::{detect_session_type, SessionType};
use crate::{error_log, verbose_log};
use std::error::Error;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use x11rb::atom_manager;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, EventMask, PropertyNotifyEvent, Window};
use x11rb::rust_connection::RustConnection;

pub fn listener_focus(tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>) {
    let session_type = detect_session_type();

    match session_type {
        SessionType::X11 => {
            verbose_log!("Detected X11 session, using X11 focus listener");
            listener_focus_x11(tx, active);
        }
        SessionType::Wayland => {
            verbose_log!("Detected Wayland session, using KWin D-Bus event-driven focus listener");
            // Import and call the Wayland listener
            crate::listener_focus_wayland::listener_focus_wayland(tx, active);
        }
    }
}

fn listener_focus_x11(tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>) {
    let active = active.clone();
    let tx = tx.clone();
    thread::spawn(move || {
        verbose_log!("Starting X11 focus listener with auto-restart");

        let mut restart_count = 0;
        const MAX_RESTARTS: u32 = 10;
        const RESTART_COOLDOWN: std::time::Duration = std::time::Duration::from_secs(5);
        let mut last_restart = std::time::Instant::now();

        // Main restart loop
        while active.load(std::sync::atomic::Ordering::Relaxed) {
            let mut listener = match X11FocusListener::new() {
                Ok(l) => l,
                Err(e) => {
                    error_log!("Error while creating X11 focus listener: {}", e);
                    error_log!("Will retry in 30 seconds...");
                    std::thread::sleep(std::time::Duration::from_secs(30));
                    continue;
                }
            };

            verbose_log!("X11 focus listener started (restart count: {})", restart_count);
            let mut should_restart = false;

            // Event loop
            while active.load(std::sync::atomic::Ordering::Relaxed) {
                match listener.get_next_focus_change() {
                    Ok((class, title)) => {
                        send(&tx, DeviceEvent::FocusChanges { class, title });
                    }
                    Err(e) => {
                        error_log!("Error while getting next focus change: {}", e);
                        should_restart = true;
                        break;
                    }
                }
            }

            // Check if we should restart or exit
            if !active.load(std::sync::atomic::Ordering::Relaxed) {
                verbose_log!("X11 focus listener stopped (shutdown requested)");
                break;
            }

            if should_restart {
                // Implement restart backoff to avoid tight restart loop
                let time_since_last_restart = last_restart.elapsed();
                if time_since_last_restart < RESTART_COOLDOWN {
                    restart_count += 1;
                    if restart_count >= MAX_RESTARTS {
                        error_log!("X11 focus listener failed too many times ({} restarts in {}s), giving up",
                                 restart_count, time_since_last_restart.as_secs());
                        break;
                    }
                    verbose_log!("Waiting {} seconds before restart attempt {}...",
                               RESTART_COOLDOWN.as_secs(), restart_count + 1);
                    std::thread::sleep(RESTART_COOLDOWN);
                } else {
                    // Reset restart count if enough time has passed
                    restart_count = 0;
                }
                last_restart = std::time::Instant::now();

                verbose_log!("Restarting X11 focus listener...");
                continue;
            }

            break;
        }

        verbose_log!("X11 focus listener thread exiting");
    });
}

atom_manager! {
    pub Atoms: AtomsCookie {
        _NET_ACTIVE_WINDOW,
        WM_CLASS,
        _NET_WM_NAME,
        WM_NAME,
        UTF8_STRING,
    }
}

struct X11FocusListener {
    conn: RustConnection,
    root: Window,
    atoms: Atoms,
    last_active_window: Option<Window>,
}

impl X11FocusListener {
    fn new() -> Result<Self, Box<dyn Error>> {
        let (conn, screen_num) = RustConnection::connect(None)?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;

        let atoms = Atoms::new(&conn)?.reply()?;

        conn.change_window_attributes(root, &x11rb::protocol::xproto::ChangeWindowAttributesAux::new()
            .event_mask(EventMask::PROPERTY_CHANGE))?;

        // Flush to ensure the event mask is set
        conn.flush()?;

        Ok(Self {
            conn,
            root,
            atoms,
            last_active_window: None,
        })
    }

    fn get_next_focus_change(&mut self) -> Result<(String, String), Box<dyn Error>> {
        loop {
            let event = self.conn.wait_for_event()?;
            if let x11rb::protocol::Event::PropertyNotify(PropertyNotifyEvent { atom, .. }) = event {
                if atom == self.atoms._NET_ACTIVE_WINDOW {
                    if let Ok(reply) = self.conn.get_property(false, self.root, self.atoms._NET_ACTIVE_WINDOW, AtomEnum::WINDOW, 0, 1)?.reply() {
                        if let Some(window_id) = reply.value32().and_then(|mut v| v.next()) {
                            if self.last_active_window != Some(window_id) {
                                self.last_active_window = Some(window_id);
                                let wm_class = self.get_window_class(window_id)?;
                                let wm_title = self.get_window_title(window_id)?;
                                return Ok((wm_class.unwrap_or_default(), wm_title.unwrap_or_default()));
                            }
                        }
                    }
                }
            }
        }
    }

    fn get_window_class(&self, window: Window) -> Result<Option<String>, Box<dyn Error>> {
        if let Ok(reply) = self.conn.get_property(false, window, self.atoms.WM_CLASS, AtomEnum::STRING, 0, 1024)?.reply() {
            if let Some(value) = reply.value8() {
                return Ok(Some(String::from_utf8_lossy(&value.collect::<Vec<u8>>()).split('\0').filter(|s| !s.is_empty()).collect::<Vec<_>>().join(".")));
            }
        }
        Ok(None)
    }

    fn get_window_title(&self, window: Window) -> Result<Option<String>, Box<dyn Error>> {
        if let Ok(reply) = self.conn.get_property(false, window, self.atoms._NET_WM_NAME, self.atoms.UTF8_STRING, 0, 1024)?.reply() {
            if let Some(value) = reply.value8() {
                return Ok(Some(String::from_utf8_lossy(&value.collect::<Vec<u8>>()).to_string()));
            }
        }
        if let Ok(reply) = self.conn.get_property(false, window, self.atoms.WM_NAME, AtomEnum::STRING, 0, 1024)?.reply() {
            if let Some(value) = reply.value8() {
                return Ok(Some(String::from_utf8_lossy(&value.collect::<Vec<u8>>()).to_string()));
            }
        }
        Ok(None)
    }
}
