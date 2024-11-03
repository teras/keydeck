use crate::event::DeviceEvent;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::sync::mpsc::Sender;
use std::thread;

pub fn listener_signal(tx: &Sender<DeviceEvent>) {
    let tx = tx.clone();
    thread::spawn(move || {
        for _ in Signals::new(&[SIGINT, SIGTERM]).unwrap().forever() {
            tx.send(DeviceEvent::Exit).unwrap();
        }
    });
}