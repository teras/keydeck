// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::event::DeviceEvent;
use crate::{error_log, verbose_log};
use dbus::arg;
use dbus::blocking::Connection;
use dbus::channel::MatchingReceiver;
use dbus::message::SignalArgs;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
struct PrepareForSleep {
    pub start: bool,
}
impl arg::AppendAll for PrepareForSleep {
    fn append(&self, i: &mut arg::IterAppend) {
        arg::RefArg::append(&self.start, i);
    }
}

impl arg::ReadAll for PrepareForSleep {
    fn read(i: &mut arg::Iter) -> Result<Self, arg::TypeMismatchError> {
        Ok(PrepareForSleep {
            start: i.read()?,
        })
    }
}

impl SignalArgs for PrepareForSleep {
    const NAME: &'static str = "PrepareForSleep";
    const INTERFACE: &'static str = "org.freedesktop.login1.Manager";
}
pub fn listener_sleep(tx: &Sender<DeviceEvent>, still_active: &Arc<AtomicBool>, should_reset: &Arc<AtomicBool>) {
    let tx = tx.clone();
    let still_active = still_active.clone();
    let should_reset = should_reset.clone();
    thread::spawn(move || {
        let conn = match Connection::new_system() {
            Ok(conn) => conn,
            Err(e) => {
                error_log!("Failed to connect to D-Bus: {}", e);
                return;
            }
        };
        let proxy = conn.with_proxy(
            "org.freedesktop.login1", // Destination
            "/org/freedesktop/login1", // Object path
            Duration::from_millis(5000),
        );

        // Register signal handler and store token for cleanup
        let signal_token = match proxy.match_signal(move |p: PrepareForSleep, _: &Connection, _: &dbus::Message| {
            tx.send(DeviceEvent::Sleep { sleep: p.start }).expect("Error sending sleep event");
            if p.start {
                should_reset.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            true
        }) {
            Ok(token) => token,
            Err(e) => {
                error_log!("Failed to register sleep signal handler: {}", e);
                return;
            }
        };

        // Main event loop
        while still_active.load(std::sync::atomic::Ordering::Relaxed) {
            conn.process(Duration::from_millis(1000)).unwrap_or_else(|e| {
                error_log!("Failed to process D-Bus messages: {}", e);
                false
            });
        }

        // Clean up signal handler before exiting
        if let Some(_) = conn.stop_receive(signal_token) {
            verbose_log!("Sleep listener signal handler cleaned up");
        }
    });
}
