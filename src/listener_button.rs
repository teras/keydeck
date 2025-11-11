// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::device_manager::find_device_by_serial;
use crate::device_trait::{KeydeckDevice, DeviceStateUpdate};
use crate::event::{send, DeviceEvent};
use crate::verbose_log;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub fn button_listener(sn: &String, tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>) {
    let serial = sn.clone();
    let active = active.clone();
    let tx = tx.clone();
    thread::spawn(move || {
        if let Some(device) = find_device_by_serial(&serial) {
            verbose_log!("Starting key listener for device {}", serial);
            while active.load(std::sync::atomic::Ordering::Relaxed) {
                if let Ok(updates) = device.get_reader().read(Some(Duration::from_secs_f64(10.0))) {
                    for update in updates {
                        let sn = serial.clone();
                        match update {
                            DeviceStateUpdate::ButtonDown(button_id) => send(&tx, DeviceEvent::ButtonDown { sn, button_id: button_id + 1 }),
                            DeviceStateUpdate::ButtonUp(button_id) => send(&tx, DeviceEvent::ButtonUp { sn, button_id: button_id + 1 }),
                            DeviceStateUpdate::EncoderDown(encoder_id) => send(&tx, DeviceEvent::EncoderDown { sn, encoder_id: encoder_id + 1 }),
                            DeviceStateUpdate::EncoderUp(encoder_id) => send(&tx, DeviceEvent::EncoderUp { sn, encoder_id: encoder_id + 1 }),
                            DeviceStateUpdate::EncoderTwist { encoder, ticks } => send(&tx, DeviceEvent::EncoderTwist { sn, encoder_id: encoder + 1, value: ticks }),
                            DeviceStateUpdate::TouchPointDown(point_id) => send(&tx, DeviceEvent::TouchPointDown { sn, point_id: point_id + 1 }),
                            DeviceStateUpdate::TouchPointUp(point_id) => send(&tx, DeviceEvent::TouchPointUp { sn, point_id: point_id + 1 }),
                            DeviceStateUpdate::TouchScreenPress { x, y } => send(&tx, DeviceEvent::TouchScreenPress { sn, x, y }),
                            DeviceStateUpdate::TouchScreenLongPress { x, y } => send(&tx, DeviceEvent::TouchScreenLongPress { sn, x, y }),
                            DeviceStateUpdate::TouchScreenSwipe { x, y, target_x, target_y } => {
                                send(&tx, DeviceEvent::TouchScreenSwipe { sn, start: (x, y), end: (target_x, target_y) })
                            },
                        }
                    }
                } else {
                    active.store(false, std::sync::atomic::Ordering::Relaxed);
                }
            }
            verbose_log!("Exiting key listener for device {}", serial);
        }
    });
}
