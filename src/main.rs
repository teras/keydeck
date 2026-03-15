// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

mod device_info;
mod device_manager;
mod device_registry_init;
mod device_trait;
mod dynamic_detection;
mod dynamic_params;
mod elgato_device;
mod event;
mod focus_property;
mod focus_property_wayland;
mod graphics_renderer;
mod keyboard;
mod keyboard_wayland;
mod kwin_script;
mod listener_button;
mod listener_device;
mod listener_focus;
mod listener_focus_wayland;
mod listener_signal;
mod listener_sleep;
mod listener_tick;
mod listener_time;
mod lock;
mod mirajazz_device;
mod paged_device;
mod press_effect;
mod pages;
mod server;
mod services;
mod session;
mod system_info;
mod text_renderer;
mod utils;
mod validate;

use crate::device_registry_init::initialize_device_registry;
use crate::device_trait::KeydeckDevice;
use crate::mirajazz_device::init_registry;
use crate::server::start_server;
use std::env;
use std::sync::atomic::AtomicBool;

pub static DEBUG: AtomicBool = AtomicBool::new(false);

fn print_help() {
    println!("Usage: keydeck [OPTION]...");
    println!("Control a Stream Deck or similar device");
    println!();
    println!("Options:");
    println!("      --logo <PATH>           Set persistent boot logo on device");
    println!("      --list                  List all devices");
    println!("      --info <DEVICE>         Show detailed device information as YAML");
    println!("      --validate <FILE>       Validate configuration file and test services");
    println!("      --json                  Output validation results as JSON (use with --validate)");
    println!("      --verbose               Print verbose messages");
    println!("      --server                Start the server (default when no arguments)");
    println!("      --help                  Display this help and exit");
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<String>>();

    // First pass: process --verbose before anything else
    if args.iter().any(|a| a == "--verbose") {
        DEBUG.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    // Initialize device registry: extract embedded JSON files and get search paths
    let device_paths = match initialize_device_registry() {
        Ok(paths) => paths,
        Err(e) => {
            error_log!("Failed to initialize device registry: {}", e);
            // Fallback to default paths if initialization fails
            vec![
                "/usr/share/keydeck/devices".to_string(),
                format!(
                    "{}/.config/keydeck/devices",
                    env::var("HOME").unwrap_or_default()
                ),
            ]
        }
    };

    if let Err(e) = init_registry(&device_paths) {
        error_log!("{}", e);
    }

    let mut arg_iter = args.iter();
    let mut should_start_server = false;

    while let Some(arg) = arg_iter.next() {
        match arg.as_str() {
            "--help" => print_help(),
            "--logo" => {
                if let Some(path) = arg_iter.next() {
                    match image::open(path) {
                        Ok(img) => {
                            let mut manager = crate::device_manager::DeviceManager::new();
                            for device in manager.iter_active_devices() {
                                device.set_boot_logo(img.clone()).unwrap_or_else(|e| {
                                    error_log!("Error setting boot logo: {}", e);
                                });
                            }
                        }
                        Err(e) => error_log!("Failed to load image '{}': {}", path, e),
                    }
                } else {
                    error_log!("Error: --logo requires a path argument");
                }
            }
            "--list" => {
                let mut manager = crate::device_manager::DeviceManager::new();
                manager.list_devices();
            }
            "--info" => {
                if let Some(arg1) = arg_iter.next() {
                    let mut manager = crate::device_manager::DeviceManager::new();
                    if let Err(e) = manager.info_device(arg1.to_uppercase()) {
                        error_log!("Error: {}", e);
                    }
                } else {
                    error_log!("Error: --info requires a device identifier argument");
                }
            }
            "--validate" => {
                if let Some(config_path) = arg_iter.next() {
                    let json_output = args.iter().any(|a| a == "--json");
                    let success = crate::validate::validate_config(config_path, json_output);
                    std::process::exit(if success { 0 } else { 1 });
                } else {
                    error_log!("Error: --validate requires a configuration file path argument");
                    std::process::exit(1);
                }
            }
            "--json" | "--verbose" => {} // Processed elsewhere
            "--server" => should_start_server = true,
            _ => {
                error_log!("Error: Unknown command '{}'", arg);
            }
        }
    }
    if args.is_empty() || should_start_server {
        start_server();
    }
}
