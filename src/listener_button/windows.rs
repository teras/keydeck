// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! Windows input listener: single shared HID handle, non-blocking polled reads.
//!
//! On Windows a device exposes one usable HID handle (the vendor collection);
//! a second handle receives no input reports. Reading and rendering therefore
//! share one handle, serialized by the device mutex. Reads are non-blocking and
//! polled so the mutex is never held during a blocking read (which would
//! deadlock rendering) — the approach `elgato-streamdeck`'s async wrapper uses.

use super::{dispatch_update, ERROR_BACKOFF, MAX_CONSECUTIVE_ERRORS};
use crate::device_trait::DeviceReader;
use crate::event::DeviceEvent;
use crate::{error_log, verbose_log};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Poll interval between non-blocking reads (~120 Hz, negligible cost).
const POLL_INTERVAL: Duration = Duration::from_millis(8);

/// Spawns the input-reading loop. `reader` comes from the same device instance
/// used for rendering, so the device is opened exactly once.
pub fn button_listener(
    reader: Arc<dyn DeviceReader>,
    sn: &String,
    tx: &Sender<DeviceEvent>,
    active: &Arc<AtomicBool>,
) {
    let serial = sn.clone();
    let active = active.clone();
    let tx = tx.clone();
    thread::spawn(move || {
        verbose_log!("Starting key listener for device {}", serial);
        let mut consecutive_errors: u32 = 0;
        while active.load(Ordering::Relaxed) {
            match reader.read(None) {
                Ok(updates) => {
                    consecutive_errors = 0;
                    for update in updates {
                        dispatch_update(update, &serial, &tx);
                    }
                }
                Err(e) => {
                    consecutive_errors += 1;
                    error_log!(
                        "Button read error on device '{}' ({}/{}): {:?}",
                        serial,
                        consecutive_errors,
                        MAX_CONSECUTIVE_ERRORS,
                        e
                    );
                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                        error_log!(
                            "Too many consecutive errors on device '{}', stopping listener",
                            serial
                        );
                        active.store(false, Ordering::Relaxed);
                    } else {
                        thread::sleep(ERROR_BACKOFF);
                    }
                }
            }
            thread::sleep(POLL_INTERVAL);
        }
        verbose_log!("Exiting key listener for device {}", serial);
    });
}
