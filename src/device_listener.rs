use crate::event::{send, DeviceEvent};
use elgato_streamdeck::{list_devices, new_hidapi};
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use crate::verbose_log;

pub fn device_listener(tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>) {
    let active = active.clone();
    let tx = tx.clone();
    thread::spawn(move || {
        let mut devices: HashSet<String> = HashSet::new();
        verbose_log!("Starting device listener");
        while active.load(std::sync::atomic::Ordering::Relaxed) {
            let hidapi = Arc::new(new_hidapi().ok().expect("Failed to create hidapi context"));
            let mut current = devices.clone();
            for (_, serial) in list_devices(&hidapi) {
                if current.contains(&serial) {
                    current.remove(&serial);
                } else {
                    verbose_log!("Found new device: {}", serial);
                    devices.insert(serial.clone());
                    send(&tx, DeviceEvent::NewDevice(serial));
                }
            }
            for removed in current {
                verbose_log!("Device removed: {}", removed);
                devices.remove(&removed);
                send(&tx, DeviceEvent::RemovedDevice(removed));
            }
            thread::sleep(std::time::Duration::from_secs_f64(2.0));
        }
        verbose_log!("Exiting device listener");
    });
}