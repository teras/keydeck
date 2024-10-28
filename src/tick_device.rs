use crate::event::{send, DeviceEvent};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

pub fn tick_listener(tx: &Sender<DeviceEvent>, still_active: &Arc<AtomicBool>) {
    let tx = tx.clone();
    let still_active = still_active.clone();
    thread::spawn(move || {
        while still_active.load(std::sync::atomic::Ordering::Relaxed) {
            thread::sleep(std::time::Duration::from_secs(2));
            send(&tx, DeviceEvent::Tick);
        }
    });
}