// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

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
use crate::pages::{KeyDeckConf, KeyDeckConfLoader};
use crate::services::new_services_state;
use crate::{error_log, info_log, verbose_log};
use indexmap::IndexMap;
use keydeck::get_icon_dir;
use keydeck_types::pages::{Button, Macro, Pages, ServiceConfig};
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

/// Helper function to initialize a device with given configuration.
/// Always creates the device, even if no config exists (device will be inactive until config is provided).
fn initialize_device(
    sn: &str,
    conf_pages: &Arc<IndexMap<String, Pages>>,
    conf_colors: &Arc<Option<IndexMap<String, String>>>,
    conf_buttons: &Arc<Option<IndexMap<String, Button>>>,
    conf_macros: &Arc<Option<IndexMap<String, Macro>>>,
    conf_services: &Arc<Option<IndexMap<String, ServiceConfig>>>,
    services_state: &crate::services::ServicesState,
    services_active: &Arc<AtomicBool>,
    icon_dir: Option<&String>,
    tx: &std::sync::mpsc::Sender<DeviceEvent>,
    time_manager: &Arc<TimeManager>,
    current_class: &str,
    current_title: &str,
    conf_brightness: u8,
    devices: &mut HashMap<String, PagedDevice>,
    initial_page: Option<String>,
) {
    if let Some(device) = find_device_by_serial(sn) {
        verbose_log!("Looking for configuration for device serial: '{}'", sn);
        verbose_log!("Available page groups: {:?}", conf_pages.keys().collect::<Vec<_>>());

        let pages_arc = if let Some(page) = conf_pages.get(sn) {
            verbose_log!("Found specific configuration for device {}", sn);
            Arc::new(page.clone())
        } else if let Some(default_page) = conf_pages.get("default") {
            verbose_log!("Using default configuration for device {}", sn);
            Arc::new(default_page.clone())
        } else {
            verbose_log!("No configuration found for device with serial number {}, initializing with empty config", sn);
            // Create empty Pages configuration
            Arc::new(Pages {
                main_page: None,
                restore_mode: keydeck_types::pages::FocusChangeRestorePolicy::Main,
                pages: IndexMap::new(),
            })
        };

        let new_device = PagedDevice::new(
            pages_arc,
            icon_dir.cloned(),
            conf_colors.clone(),
            conf_buttons.clone(),
            conf_macros.clone(),
            conf_services.clone(),
            services_state.clone(),
            services_active.clone(),
            Box::new(device),
            tx,
            time_manager.clone(),
            initial_page,
            conf_brightness,
        );
        new_device.focus_changed(current_class, current_title, false);
        info_log!("Adding device {}", sn);
        devices.insert(sn.to_string(), new_device);
    }
}

pub fn start_server() {
    ensure_lock();
    info_log!("Starting KeyDeck Server");

    // Configuration - now reloadable via SIGHUP using Arc
    let conf = Arc::new(KeyDeckConfLoader::load());
    let mut conf_pages = Arc::new(conf.page_groups.clone());
    let mut conf_colors = Arc::new(conf.colors.clone());
    let mut conf_buttons = Arc::new(conf.buttons.clone());
    let mut conf_macros = Arc::new(conf.macros.clone());
    let mut conf_services = Arc::new(conf.services.clone());
    let icon_dir = Some(get_icon_dir());
    let mut conf_brightness = conf.brightness;
    let conf_tick_time = Arc::new(std::sync::Mutex::new(conf.tick_time));

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
    listener_tick(&tx, &still_active.clone(), conf_tick_time.clone());

    let mut devices: HashMap<String, PagedDevice> = HashMap::new();
    // Track saved page states across reload events
    let mut saved_pages: HashMap<String, String> = HashMap::new();
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
                    device.get_hardware().keep_alive();
                    device.handle_tick();
                }
            }
            ref message @ DeviceEvent::NewDevice { ref sn } => {
                // Dispatch wait event first
                dispatch_wait_event(message, &devices);
                // Then handle new device
                if !devices.contains_key(sn) {
                    // Check if we have a saved page for this device (from reload)
                    let initial_page = saved_pages.remove(sn).clone();
                    if initial_page.is_some() {
                        verbose_log!("Restoring device {} to page '{}'", sn, initial_page.as_ref().unwrap());
                    }
                    initialize_device(sn, &conf_pages, &conf_colors, &conf_buttons, &conf_macros, &conf_services, &services_state, &services_active, icon_dir.as_ref(), &tx, &time_manager, &current_class, &current_title, conf_brightness, &mut devices, initial_page);
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

                // Stop old services (but keep devices running)
                services_active.store(false, std::sync::atomic::Ordering::Relaxed);

                // Reload configuration from file
                info_log!("Reloading configuration from file...");
                let new_conf = Arc::new(KeyDeckConfLoader::load());
                conf_pages = Arc::new(new_conf.page_groups.clone());
                conf_colors = Arc::new(new_conf.colors.clone());
                conf_buttons = Arc::new(new_conf.buttons.clone());
                conf_macros = Arc::new(new_conf.macros.clone());
                conf_services = Arc::new(new_conf.services.clone());
                // icon_dir remains hard-coded - no need to update
                conf_brightness = new_conf.brightness;

                // Update tick_time in the mutex (will be used in next tick cycle)
                *conf_tick_time.lock().unwrap() = new_conf.tick_time;

                // Create new services state and active flag
                services_state = new_services_state();
                services_active = Arc::new(AtomicBool::new(true));

                // Update all connected devices with new configuration
                info_log!("Updating {} device(s) with new configuration...", devices.len());
                for (sn, device) in devices.iter_mut() {
                    verbose_log!("Reloading device {}", sn);

                    // Get the Pages configuration for this device (by serial number or default)
                    let pages_arc = if let Some(page) = conf_pages.get(sn) {
                        Arc::new(page.clone())
                    } else if let Some(default_page) = conf_pages.get("default") {
                        Arc::new(default_page.clone())
                    } else {
                        verbose_log!("No configuration found for device with serial number {}, using empty config", sn);
                        Arc::new(Pages {
                            main_page: None,
                            restore_mode: keydeck_types::pages::FocusChangeRestorePolicy::Main,
                            pages: IndexMap::new(),
                        })
                    };

                    device.reload(
                        pages_arc,
                        conf_colors.clone(),
                        conf_buttons.clone(),
                        conf_macros.clone(),
                        conf_services.clone(),
                        services_state.clone(),
                        services_active.clone(),
                        conf_brightness
                    );
                }

                info_log!("Configuration reloaded successfully");
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
            DeviceEvent::SetBrightness { sn, brightness } => {
                if let Some(device) = devices.get(&sn) {
                    verbose_log!("Setting brightness to {} for device {}", brightness, sn);
                    device.get_hardware().set_brightness(brightness).unwrap_or_else(|e| {
                        error_log!("Error while setting brightness on device {}: {}", sn, e)
                    });
                }
            }
        }
    }
}
