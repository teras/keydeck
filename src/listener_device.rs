// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::device_manager::DeviceManager;
use crate::event::{send, DeviceEvent};
use crate::verbose_log;
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

pub fn listener_device(tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>, should_reset: &Arc<AtomicBool>) {
    let active = active.clone();
    let should_reset = should_reset.clone();
    let tx = tx.clone();
    thread::spawn(move || {
        let mut devices: HashSet<String> = HashSet::new();
        verbose_log!("Starting device listener");
        while active.load(std::sync::atomic::Ordering::Relaxed) {
            if should_reset.load(std::sync::atomic::Ordering::Relaxed) {
                devices.clear();
                should_reset.store(false, std::sync::atomic::Ordering::Relaxed);
            }

            let mut current = devices.clone();
            for serial in DeviceManager::enumerate_connected_devices() {
                if current.contains(&serial) {
                    current.remove(&serial);
                } else {
                    verbose_log!("Found new device: {}", serial);
                    devices.insert(serial.clone());
                    send(&tx, DeviceEvent::NewDevice { sn: serial });
                }
            }
            for removed in current {
                verbose_log!("Device removed: {}", removed);
                devices.remove(&removed);
                send(&tx, DeviceEvent::RemovedDevice { sn: removed });
            }
            thread::sleep(std::time::Duration::from_secs_f64(2.0));
        }
        verbose_log!("Exiting device listener");
    });
}
