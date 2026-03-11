// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::event::{send, DeviceEvent};
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
    let tx = tx.clone();
    let active = active.clone();

    thread::spawn(move || {
        verbose_log!("Starting focus listener with auto-detection loop");

        while active.load(std::sync::atomic::Ordering::Relaxed) {
            // Try Wayland first
            verbose_log!("Trying Wayland focus listener...");
            if crate::listener_focus_wayland::try_wayland_listener(&tx, &active) {
                verbose_log!("Wayland listener exited, will retry...");
                thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }

            // Wayland failed, try X11
            verbose_log!("Wayland unavailable, trying X11 focus listener...");
            if try_x11_listener(&tx, &active) {
                verbose_log!("X11 listener exited, will retry...");
                thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }

            // Both failed - wait and retry
            verbose_log!("Both Wayland and X11 unavailable, retrying in 5 seconds...");
            thread::sleep(std::time::Duration::from_secs(5));
        }

        verbose_log!("Focus listener thread exiting");
    });
}

/// Try to run the X11 focus listener. Returns true if it ran successfully for a while,
/// false if it failed to start.
fn try_x11_listener(tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>) -> bool {
    let mut listener = match X11FocusListener::new() {
        Ok(l) => l,
        Err(e) => {
            error_log!("Error while creating X11 focus listener: {}", e);
            return false;
        }
    };

    verbose_log!("X11 focus listener started");

    // Event loop
    while active.load(std::sync::atomic::Ordering::Relaxed) {
        match listener.get_next_focus_change() {
            Ok((class, title)) => {
                send(&tx, DeviceEvent::FocusChanges { class, title });
            }
            Err(e) => {
                error_log!("X11 focus listener error: {}", e);
                return true; // Ran but disconnected
            }
        }
    }

    verbose_log!("X11 focus listener stopped (shutdown requested)");
    true
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

        conn.change_window_attributes(
            root,
            &x11rb::protocol::xproto::ChangeWindowAttributesAux::new()
                .event_mask(EventMask::PROPERTY_CHANGE),
        )?;

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
            if let x11rb::protocol::Event::PropertyNotify(PropertyNotifyEvent { atom, .. }) = event
            {
                if atom == self.atoms._NET_ACTIVE_WINDOW {
                    if let Ok(reply) = self
                        .conn
                        .get_property(
                            false,
                            self.root,
                            self.atoms._NET_ACTIVE_WINDOW,
                            AtomEnum::WINDOW,
                            0,
                            1,
                        )?
                        .reply()
                    {
                        if let Some(window_id) = reply.value32().and_then(|mut v| v.next()) {
                            if self.last_active_window != Some(window_id) {
                                self.last_active_window = Some(window_id);
                                let wm_class = self.get_window_class(window_id)?;
                                let wm_title = self.get_window_title(window_id)?;
                                return Ok((
                                    wm_class.unwrap_or_default(),
                                    wm_title.unwrap_or_default(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    fn get_window_class(&self, window: Window) -> Result<Option<String>, Box<dyn Error>> {
        if let Ok(reply) = self
            .conn
            .get_property(
                false,
                window,
                self.atoms.WM_CLASS,
                AtomEnum::STRING,
                0,
                1024,
            )?
            .reply()
        {
            if let Some(value) = reply.value8() {
                return Ok(Some(
                    String::from_utf8_lossy(&value.collect::<Vec<u8>>())
                        .split('\0')
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join("."),
                ));
            }
        }
        Ok(None)
    }

    fn get_window_title(&self, window: Window) -> Result<Option<String>, Box<dyn Error>> {
        if let Ok(reply) = self
            .conn
            .get_property(
                false,
                window,
                self.atoms._NET_WM_NAME,
                self.atoms.UTF8_STRING,
                0,
                1024,
            )?
            .reply()
        {
            if let Some(value) = reply.value8() {
                return Ok(Some(
                    String::from_utf8_lossy(&value.collect::<Vec<u8>>()).to_string(),
                ));
            }
        }
        if let Ok(reply) = self
            .conn
            .get_property(false, window, self.atoms.WM_NAME, AtomEnum::STRING, 0, 1024)?
            .reply()
        {
            if let Some(value) = reply.value8() {
                return Ok(Some(
                    String::from_utf8_lossy(&value.collect::<Vec<u8>>()).to_string(),
                ));
            }
        }
        Ok(None)
    }
}
