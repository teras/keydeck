// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! Device input listener.
//!
//! The reading strategy is platform-specific and lives in per-platform files to
//! keep each one clean:
//! * `windows` — single shared HID handle, non-blocking polled reads (a second
//!   handle receives no input on Windows, and a blocking read on the shared
//!   handle would deadlock rendering).
//! * `unix` (Linux/macOS) — own handle, blocking reads (hidraw/IOKit allow
//!   multiple independent handles; zero idle CPU, the original design).
//!
//! Both share [`dispatch_update`] for turning driver updates into events.

use crate::device_trait::DeviceStateUpdate;
use crate::event::{send, DeviceEvent};
use std::sync::mpsc::Sender;
use std::time::Duration;

pub(crate) const MAX_CONSECUTIVE_ERRORS: u32 = 3;
pub(crate) const ERROR_BACKOFF: Duration = Duration::from_secs(1);

/// Converts a driver-level input update into a `DeviceEvent` and dispatches it.
pub(crate) fn dispatch_update(update: DeviceStateUpdate, serial: &str, tx: &Sender<DeviceEvent>) {
    let sn = serial.to_string();
    match update {
        DeviceStateUpdate::ButtonDown(button_id) => send(
            tx,
            DeviceEvent::ButtonDown {
                sn,
                button_id: button_id + 1,
            },
        ),
        DeviceStateUpdate::ButtonUp(button_id) => send(
            tx,
            DeviceEvent::ButtonUp {
                sn,
                button_id: button_id + 1,
            },
        ),
        DeviceStateUpdate::EncoderDown(encoder_id) => send(
            tx,
            DeviceEvent::EncoderDown {
                sn,
                encoder_id: encoder_id + 1,
            },
        ),
        DeviceStateUpdate::EncoderUp(encoder_id) => send(
            tx,
            DeviceEvent::EncoderUp {
                sn,
                encoder_id: encoder_id + 1,
            },
        ),
        DeviceStateUpdate::EncoderTwist { encoder, ticks } => send(
            tx,
            DeviceEvent::EncoderTwist {
                sn,
                encoder_id: encoder + 1,
                value: ticks,
            },
        ),
        DeviceStateUpdate::TouchPointDown(point_id) => send(
            tx,
            DeviceEvent::TouchPointDown {
                sn,
                point_id: point_id + 1,
            },
        ),
        DeviceStateUpdate::TouchPointUp(point_id) => send(
            tx,
            DeviceEvent::TouchPointUp {
                sn,
                point_id: point_id + 1,
            },
        ),
        DeviceStateUpdate::TouchScreenPress { x, y } => {
            send(tx, DeviceEvent::TouchScreenPress { sn, x, y })
        }
        DeviceStateUpdate::TouchScreenLongPress { x, y } => {
            send(tx, DeviceEvent::TouchScreenLongPress { sn, x, y })
        }
        DeviceStateUpdate::TouchScreenSwipe {
            x,
            y,
            target_x,
            target_y,
        } => send(
            tx,
            DeviceEvent::TouchScreenSwipe {
                sn,
                start: (x, y),
                end: (target_x, target_y),
            },
        ),
    }
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::button_listener;

#[cfg(not(target_os = "windows"))]
mod unix;
#[cfg(not(target_os = "windows"))]
pub use unix::button_listener;
