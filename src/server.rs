use crate::device_manager::find_device_by_serial;
use crate::event::DeviceEvent;
use crate::focus_property::get_focus;
use crate::listener_signal::listener_signal;
use crate::listener_device::listener_device;
use crate::listener_focus::listener_focus;
use crate::listener_sleep::listener_sleep;
use crate::listener_tick::listener_tick;
use crate::paged_device::PagedDevice;
use crate::pages::Pages;
use crate::{info_log, verbose_log};
use std::collections::HashMap;
use std::process::exit;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub fn start_server() {
    let pages = Arc::new(Pages::new());
    let (mut current_class, mut current_title) = get_focus();

    let (tx, rx) = std::sync::mpsc::channel::<DeviceEvent>();
    let still_active = Arc::new(AtomicBool::new(true));
    let should_reset_devices = Arc::new(AtomicBool::new(false));

    listener_sleep(&tx, &still_active.clone(), &should_reset_devices);
    listener_device(&tx, &still_active.clone(), &should_reset_devices);
    listener_focus(&tx, &still_active.clone());
    listener_signal(&tx);
    listener_tick(&tx, &still_active.clone());

    let mut devices: HashMap<String, PagedDevice> = HashMap::new();
    for message in rx {
        match message {
            DeviceEvent::ButtonDown { sn, button_id } => {
                if let Some(device) = devices.get(&sn) {
                    device.button_down(button_id);
                }
            }
            DeviceEvent::ButtonUp { sn, button_id } => {
                if let Some(device) = devices.get(&sn) {
                    device.button_up(button_id);
                }
            }
            DeviceEvent::EncoderDown { sn, encoder_id } => {
                if let Some(device) = devices.get(&sn) {
                    device.encoder_down(encoder_id);
                }
            }
            DeviceEvent::EncoderUp { sn, encoder_id } => {
                if let Some(device) = devices.get(&sn) {
                    device.encoder_up(encoder_id);
                }
            }
            DeviceEvent::EncoderTwist { sn, encoder_id, value } => {
                if let Some(device) = devices.get(&sn) {
                    device.encoder_twist(encoder_id, value);
                }
            }
            DeviceEvent::TouchPointDown { sn, point_id } => {
                if let Some(device) = devices.get(&sn) {
                    device.touch_point_down(point_id);
                }
            }
            DeviceEvent::TouchPointUp { sn, point_id } => {
                if let Some(device) = devices.get(&sn) {
                    device.touch_point_up(point_id);
                }
            }
            DeviceEvent::TouchScreenPress { sn, x, y } => {
                if let Some(device) = devices.get(&sn) {
                    device.touch_screen_press(x, y);
                }
            }
            DeviceEvent::TouchScreenLongPress { sn, x, y } => {
                if let Some(device) = devices.get(&sn) {
                    device.touch_screen_long_press(x, y);
                }
            }
            DeviceEvent::TouchScreenSwipe { sn, start, end } => {
                if let Some(device) = devices.get(&sn) {
                    device.touch_screen_swipe(start, end);
                }
            }
            DeviceEvent::FocusChanges { class, title } => {
                current_class = class;
                current_title = title;
                for device in devices.values() {
                    device.focus_changed(&current_class, &current_title);
                }
            }
            DeviceEvent::Tick => {
                for device in devices.values() {
                    device.keep_alive();
                }
            }
            DeviceEvent::NewDevice { sn } => {
                if devices.contains_key(&sn) {
                    return;
                }
                if let Some(device) = find_device_by_serial(&sn) {
                    info_log!("Adding device {}", sn);
                    let new_device = PagedDevice::new(pages.clone(), device, &tx);
                    new_device.focus_changed(&current_class, &current_title);
                    devices.insert(sn, new_device);
                }
            }
            DeviceEvent::RemovedDevice { sn } => {
                if let Some(device) = devices.remove(&sn) {
                    info_log!("Removing device {}", sn);
                    device.disable();
                }
            }
            DeviceEvent::Exit => {
                info_log!("Exiting Apllication");
                for device in devices.values() {
                    device.terminate();
                }
                still_active.store(false, std::sync::atomic::Ordering::Relaxed);
                exit(0);
            }
            DeviceEvent::Sleep { sleep } => {
                if sleep {
                    verbose_log!("Sleeping");
                    for device in devices.values() {
                        device.terminate();
                    }
                    devices.clear();
                } else {
                    verbose_log!("Waking up");
                }
            }
        }
    }
}