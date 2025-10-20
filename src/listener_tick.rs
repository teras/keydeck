use crate::event::{send, DeviceEvent};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub fn listener_tick(tx: &Sender<DeviceEvent>, still_active: &Arc<AtomicBool>, tick_time: f64) {
    let tx = tx.clone();
    let still_active = still_active.clone();
    thread::spawn(move || {
        while still_active.load(std::sync::atomic::Ordering::Relaxed) {
            thread::sleep(Duration::from_secs_f64(tick_time));
            send(&tx, DeviceEvent::Tick);
        }
    });
}