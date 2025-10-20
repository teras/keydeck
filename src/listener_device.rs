use crate::event::{send, DeviceEvent};
use crate::verbose_log;
use elgato_streamdeck::{list_devices, new_hidapi};
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
            let hidapi = match new_hidapi().ok() {
                Some(api) => Arc::new(api),
                None => {
                    verbose_log!("Failed to create hidapi context, retrying in 2 seconds...");
                    thread::sleep(std::time::Duration::from_secs_f64(2.0));
                    continue;
                }
            };
            let mut current = devices.clone();
            for (_, serial) in list_devices(&hidapi) {
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