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

    // Configuration - now reloadable via SIGHUP using Arc
    let conf = Arc::new(KeyDeckConf::new());
    let mut conf_pages = Arc::new(conf.page_groups.clone());
    let mut conf_colors = Arc::new(conf.colors.clone());
    let mut conf_buttons = Arc::new(conf.buttons.clone());
    let mut conf_macros = Arc::new(conf.macros.clone());
    let mut conf_services = Arc::new(conf.services.clone());
    let mut conf_image_dir = conf.image_dir.clone();

    // Initialize with empty focus - listener will send current window immediately
    let (mut current_class, mut current_title) = (String::new(), String::new());

    let (tx, rx) = std::sync::mpsc::channel::<DeviceEvent>();
    let still_active = Arc::new(AtomicBool::new(true));
    let should_reset_devices = Arc::new(AtomicBool::new(false));

    // Create TimeManager for handling async wait timers
    let time_manager = Arc::new(TimeManager::new(tx.clone(), still_active.clone()));

    // Create shared services state for dynamic buttons - can be replaced on reload
    let mut services_state = new_services_state();
    let mut services_active = Arc::new(AtomicBool::new(true));

    listener_sleep(&tx, &still_active.clone(), &should_reset_devices);
    listener_device(&tx, &still_active.clone(), &should_reset_devices);
    listener_focus(&tx, &still_active.clone());
    listener_signal(&tx);
    listener_tick(&tx, &still_active.clone(), conf.tick_time);

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
                        let pages_arc = if let Some(page) = conf_pages.get(sn) {
                            Some(Arc::new(page.clone()))
                        } else if let Some(default_page) = conf_pages.get("default") {
                            Some(Arc::new(default_page.clone()))
                        } else {
                            error_log!("Unable to match profile for device with serial number {}, or missing default profile", sn);
                            None
                        };
                        if let Some(pages) = pages_arc {
                            let new_device = PagedDevice::new(pages, conf_image_dir.clone(), conf_colors.clone(), conf_buttons.clone(), conf_macros.clone(), conf_services.clone(), services_state.clone(), services_active.clone(), device, &tx, time_manager.clone());
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
            DeviceEvent::Reload => {
                info_log!("Reloading Configuration (SIGHUP received)");
                info_log!("Stopping services and clearing devices...");

                // Terminate all devices
                for device in devices.values() {
                    device.terminate();
                }
                devices.clear();

                // Stop old services
                services_active.store(false, std::sync::atomic::Ordering::Relaxed);

                // Reload configuration from file
                info_log!("Reloading configuration from file...");
                let new_conf = Arc::new(KeyDeckConf::new());
                conf_pages = Arc::new(new_conf.page_groups.clone());
                conf_colors = Arc::new(new_conf.colors.clone());
                conf_buttons = Arc::new(new_conf.buttons.clone());
                conf_macros = Arc::new(new_conf.macros.clone());
                conf_services = Arc::new(new_conf.services.clone());
                conf_image_dir = new_conf.image_dir.clone();

                // Create new services state and active flag
                services_state = new_services_state();
                services_active = Arc::new(AtomicBool::new(true));

                // Reset device listener so it rediscovers all connected devices
                should_reset_devices.store(true, std::sync::atomic::Ordering::Relaxed);

                info_log!("Configuration reloaded - devices will reconnect with new config");
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