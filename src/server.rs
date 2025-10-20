use crate::device_manager::find_device_by_serial;
use crate::event::DeviceEvent;
use crate::listener_device::listener_device;
use crate::listener_focus::listener_focus;
use crate::listener_signal::listener_signal;
use crate::listener_sleep::listener_sleep;
use crate::listener_tick::listener_tick;
use crate::listener_time::TimeManager;
use crate::lock::{cleanup_lock, ensure_lock};
use crate::paged_device::PagedDevice;
use crate::pages::KeyDeckConf;
use crate::services::new_services_state;
use crate::{error_log, info_log, verbose_log};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Helper function to dispatch wait events to all devices that might be waiting for them.
/// Checks if the event can be waited for, and if so, notifies all devices.
fn dispatch_wait_event(message: &DeviceEvent, devices: &HashMap<String, PagedDevice>) {
    if let Some(event_type) = message.wait_event_type() {
        for device in devices.values() {
            device.check_pending_event(&event_type);
        }
    }
}

pub fn start_server() {
    ensure_lock();
    info_log!("Starting KeyDeck Server");

    let conf = Arc::new(KeyDeckConf::new());
    // Initialize with empty focus - listener will send current window immediately
    let (mut current_class, mut current_title) = (String::new(), String::new());

    let (tx, rx) = std::sync::mpsc::channel::<DeviceEvent>();
    let still_active = Arc::new(AtomicBool::new(true));
    let should_reset_devices = Arc::new(AtomicBool::new(false));

    // Create TimeManager for handling async wait timers
    let time_manager = TimeManager::new(tx.clone(), still_active.clone());

    // Create shared services state for dynamic buttons
    let services_state = new_services_state();

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
            ref message @ DeviceEvent::FocusChanges { ref class, ref title } => {
                current_class = class.clone();
                current_title = title.clone();
                // Dispatch wait event first
                dispatch_wait_event(message, &devices);
                // Then handle normal focus change
                for device in devices.values() {
                    device.focus_changed(&current_class, &current_title, false);
                }
            }
            ref message @ DeviceEvent::Tick => {
                // Dispatch wait event first
                dispatch_wait_event(message, &devices);
                // Then handle tick
                for device in devices.values() {
                    device.keep_alive();
                    device.handle_tick();
                }
            }
            ref message @ DeviceEvent::NewDevice { ref sn } => {
                // Dispatch wait event first
                dispatch_wait_event(message, &devices);
                // Then handle new device
                if !devices.contains_key(sn) {
                    if let Some(device) = find_device_by_serial(sn) {
                        let pages = if let Some(page) = conf.page_groups.get(sn) {
                            Some(page)
                        } else if let Some(default_page) = conf.page_groups.get("default") {
                            Some(default_page)
                        } else {
                            error_log!("Unable to match profile for device with serial number {}, or missing default profile", sn);
                            None
                        };
                        if let Some(pages) = pages {
                            let new_device = PagedDevice::new(&pages, conf.image_dir.clone(), &conf.colors, &conf.buttons, &conf.macros, &conf.services, services_state.clone(), device, &tx, &time_manager);
                            new_device.focus_changed(&current_class, &current_title, false);
                            info_log!("Adding device {}", sn);
                            devices.insert(sn.clone(), new_device);
                        }
                    }
                }
            }
            ref message @ DeviceEvent::RemovedDevice { ref sn } => {
                // Dispatch wait event first
                dispatch_wait_event(message, &devices);
                // Then handle device removal
                if let Some(device) = devices.remove(sn) {
                    info_log!("Removing device {}", sn);
                    device.disable();
                }
            }
            DeviceEvent::Exit => {
                info_log!("Exiting Application");
                for device in devices.values() {
                    device.terminate();
                }
                still_active.store(false, std::sync::atomic::Ordering::Relaxed);

                // Clean up KWin scripts before exiting (for Wayland)
                crate::kwin_script::KWinScriptClient::cleanup_stale_scripts_static();

                cleanup_lock();
                break; // Exit the event loop gracefully
            }
            ref message @ DeviceEvent::Sleep { sleep } => {
                // Dispatch wait event first
                dispatch_wait_event(message, &devices);
                // Handle sleep event
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
            ref message @ DeviceEvent::TimerComplete { ref sn } => {
                // Dispatch wait event to the specific device waiting for this timer
                dispatch_wait_event(message, &devices);
                verbose_log!("Timer completed for device {}", sn);
            }
        }
    }
}