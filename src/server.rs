use crate::device_listener::device_listener;
use crate::device_manager::find_device_by_serial;
use crate::event::DeviceEvent;
use crate::focus_listener::focus_listener;
use crate::paged_device::PagedDevice;
use crate::pages::Pages;
use crate::tick_device::tick_listener;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub fn start_server() {
    let pages = Arc::new(Pages::new());
    let (tx, rx) = std::sync::mpsc::channel::<DeviceEvent>();

    let still_active = Arc::new(AtomicBool::new(true));
    device_listener(&tx, &still_active.clone());
    focus_listener(&tx, &still_active.clone());
    tick_listener(&tx, &still_active.clone());

    let mut devices: HashMap<String, PagedDevice> = HashMap::new();

    for message in rx {
        match message {
            DeviceEvent::ButtonDown(sn, button_id) => {
                if let Some(device) = devices.get(&sn) {
                    device.button_down(button_id);
                }
            }
            DeviceEvent::ButtonUp(sn, button_id) => {
                if let Some(device) = devices.get(&sn) {
                    device.button_up(button_id);
                }
            }
            DeviceEvent::EncoderDown(sn, encoder_id) => {
                if let Some(device) = devices.get(&sn) {
                    device.encoder_down(encoder_id);
                }
            }
            DeviceEvent::EncoderUp(sn, encoder_id) => {
                if let Some(device) = devices.get(&sn) {
                    device.encoder_up(encoder_id);
                }
            }
            DeviceEvent::EncoderTwist(sn, encoder_id, value) => {
                if let Some(device) = devices.get(&sn) {
                    device.encoder_twist(encoder_id, value);
                }
            }
            DeviceEvent::TouchPointDown(sn, point_id) => {
                if let Some(device) = devices.get(&sn) {
                    device.touch_point_down(point_id);
                }
            }
            DeviceEvent::TouchPointUp(sn, point_id) => {
                if let Some(device) = devices.get(&sn) {
                    device.touch_point_up(point_id);
                }
            }
            DeviceEvent::TouchScreenPress(sn, x, y) => {
                if let Some(device) = devices.get(&sn) {
                    device.touch_screen_press(x, y);
                }
            }
            DeviceEvent::TouchScreenLongPress(sn, x, y) => {
                if let Some(device) = devices.get(&sn) {
                    device.touch_screen_long_press(x, y);
                }
            }
            DeviceEvent::TouchScreenSwipe(sn, from, to) => {
                if let Some(device) = devices.get(&sn) {
                    device.touch_screen_swipe(from, to);
                }
            }
            DeviceEvent::FocusChanges(class, title) => {
                println!("Focus change: {} - {}", class, title);
            }
            DeviceEvent::Tick => {
                println!("Tick");
            }
            DeviceEvent::NewDevice(sn) => {
                if devices.contains_key(&sn) {
                    return;
                }
                if let Some(device) = find_device_by_serial(&sn) {
                    devices.insert(sn, PagedDevice::new(pages.clone(), device, &tx));
                }
            }
            DeviceEvent::RemovedDevice(sn) => {
                if let Some(device) = devices.remove(&sn) {
                    device.terminate();
                }
            }
        }
    }
}