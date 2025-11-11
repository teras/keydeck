// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::{event::DeviceEvent, error_log};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::sync::mpsc::Sender;
use std::thread;

pub fn listener_signal(tx: &Sender<DeviceEvent>) {
    let tx = tx.clone();
    thread::spawn(move || {
        let mut signals = match Signals::new(&[SIGINT, SIGTERM, SIGHUP]) {
            Ok(s) => s,
            Err(e) => {
                error_log!("Failed to initialize signal handler: {}", e);
                error_log!("Signal handling (Ctrl+C) will not work properly");
                return;
            }
        };

        for signal in signals.forever() {
            // Differentiate between signals
            let event = match signal {
                SIGHUP => DeviceEvent::Reload,
                SIGINT | SIGTERM => DeviceEvent::Exit,
                _ => continue, // Ignore unexpected signals
            };
            // Silently ignore send errors - if receiver is dropped, we're shutting down anyway
            let _ = tx.send(event);
        }
    });
}
