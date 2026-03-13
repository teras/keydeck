// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::event::DeviceEvent;
use crate::error_log;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use tokio::runtime::Runtime;

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool);
}

pub fn listener_sleep(
    tx: &Sender<DeviceEvent>,
    still_active: &Arc<AtomicBool>,
    should_reset: &Arc<AtomicBool>,
) {
    let tx = tx.clone();
    let still_active = still_active.clone();
    let should_reset = should_reset.clone();
    thread::spawn(move || {
        // zbus::blocking requires a tokio runtime context on the current thread
        let rt = match Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error_log!("Failed to create tokio runtime for sleep listener: {}", e);
                return;
            }
        };
        let _guard = rt.enter();
        let conn = match zbus::blocking::Connection::system() {
            Ok(conn) => conn,
            Err(e) => {
                error_log!("Failed to connect to system D-Bus: {}", e);
                return;
            }
        };

        let proxy = match Login1ManagerProxyBlocking::builder(&conn).build() {
            Ok(proxy) => proxy,
            Err(e) => {
                error_log!("Failed to create login1 proxy: {}", e);
                return;
            }
        };

        let signals = match proxy.receive_prepare_for_sleep() {
            Ok(signals) => signals,
            Err(e) => {
                error_log!("Failed to subscribe to sleep signal: {}", e);
                return;
            }
        };

        for signal in signals {
            if !still_active.load(Ordering::Relaxed) {
                break;
            }
            match signal.args() {
                Ok(args) => {
                    let _ = tx.send(DeviceEvent::Sleep { sleep: args.start });
                    if args.start {
                        should_reset.store(true, Ordering::Relaxed);
                    }
                }
                Err(e) => {
                    error_log!("Failed to read sleep signal args: {}", e);
                }
            }
        }
    });
}
