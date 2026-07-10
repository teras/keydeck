// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! Linux/macOS input listener: own HID handle, blocking reads.
//!
//! hidraw (Linux) and IOKit (macOS) allow multiple independent handles on the
//! same device, each delivering input reports. The listener therefore opens its
//! own handle and uses a blocking read with a long timeout — zero idle CPU.
//! This is the original design, unchanged.

use super::{dispatch_update, ERROR_BACKOFF, MAX_CONSECUTIVE_ERRORS};
use crate::device_manager::find_device_by_serial;
use crate::device_trait::KeydeckDevice;
use crate::event::DeviceEvent;
use crate::{error_log, verbose_log};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Blocking read timeout. The read blocks until input arrives (or this elapses),
/// so the listener consumes no CPU while idle.
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Spawns the input-reading loop. Opens its own handle on the device.
pub fn button_listener(sn: &String, tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>) {
    let serial = sn.clone();
    let active = active.clone();
    let tx = tx.clone();
    thread::spawn(move || {
        if let Some(device) = find_device_by_serial(&serial) {
            verbose_log!("Starting key listener for device {}", serial);
            let reader = device.get_reader();
            let mut consecutive_errors: u32 = 0;
            while active.load(Ordering::Relaxed) {
                match reader.read(Some(READ_TIMEOUT)) {
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
            }
            verbose_log!("Exiting key listener for device {}", serial);
        }
    });
}
