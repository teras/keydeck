// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

mod device_info;
mod device_manager;
mod device_registry_init;
mod context;
mod device_trait;
mod dynamic_detection;
mod dynamic_params;
mod elgato_device;
mod event;
mod integrations;
mod graphics_renderer;
mod listener_button;
#[cfg(unix)]
mod listener_context;
mod listener_device;
mod listener_tick;
mod listener_time;
mod lock;
mod mirajazz_device;
mod paged_device;
mod platform;
mod press_effect;
mod pages;
mod server;
mod services;
mod system_info;
mod text_renderer;
mod utils;
mod validate;

// Linux-only native backends (X11 / Wayland / KWin / logind / signals).
// On Windows and macOS these are provided by `platform::{windows,macos}`.
#[cfg(target_os = "linux")]
mod focus_property;
#[cfg(target_os = "linux")]
mod focus_property_wayland;
#[cfg(target_os = "linux")]
mod keyboard;
#[cfg(target_os = "linux")]
mod keyboard_wayland;
#[cfg(target_os = "linux")]
mod kwin_script;
#[cfg(target_os = "linux")]
mod listener_focus;
#[cfg(target_os = "linux")]
mod listener_focus_wayland;
#[cfg(target_os = "linux")]
mod listener_signal;
#[cfg(target_os = "linux")]
mod listener_sleep;
#[cfg(target_os = "linux")]
mod session;

use crate::device_registry_init::initialize_device_registry;
use crate::device_trait::KeydeckDevice;
use crate::mirajazz_device::init_registry;
use crate::server::start_server;
use std::env;
use std::sync::atomic::AtomicU8;

/// Verbosity level: 0 = normal, 1 = detailed, 2 = verbose/debug
pub static VERBOSITY: AtomicU8 = AtomicU8::new(0);

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
    println!("      --set <KEY=VALUE>       Set a context variable on the running daemon");
    println!("                                (empty value clears it; used by external watchers)");
    println!("      --daemon <ACTION>       Manage the daemon lifecycle. ACTION is one of:");
    println!("                                install    register autostart at login");
    println!("                                uninstall  remove autostart entry");
    println!("                                start      start the daemon now");
    println!("                                stop       stop the running daemon");
    println!("                                restart    restart the daemon");
    println!("                                status     print JSON {{running,pid,enabled}}");
    println!("                                reload     reload config of running daemon");
    println!("      --integration <NAME> <ACTION>");
    println!("                              Manage a terminal integration (NAME: kitty).");
    println!("                              ACTION: install, uninstall, status");
    println!("  -v, --verbose               Print detailed messages (key presses, page changes)");
    println!("  -vv, --verbose --verbose    Print all verbose/debug messages");
    println!("      --server                Start the server (default when no arguments)");
    println!("      --help                  Display this help and exit");
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<String>>();

    // First pass: process verbosity flags before anything else
    let verbosity = args.iter().fold(0u8, |level, a| match a.as_str() {
        "-vv" => level.max(2),
        "-v" | "--verbose" => level.saturating_add(1).min(2),
        _ => level,
    });
    VERBOSITY.store(verbosity, std::sync::atomic::Ordering::Relaxed);

    // Initialize device registry: extract embedded JSON files and get search paths
    let device_paths = match initialize_device_registry() {
        Ok(paths) => paths,
        Err(e) => {
            error_log!("Failed to initialize device registry: {}", e);
            // Fallback to the user config directory if initialization fails
            vec![keydeck::get_config_dir()
                .join("devices")
                .to_string_lossy()
                .into_owned()]
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
            "--daemon" => {
                use crate::platform::lifecycle::Action;
                let action = arg_iter.next().and_then(|a| Action::parse(a));
                match action {
                    Some(action) => match crate::platform::lifecycle::run(action) {
                        Ok(code) => std::process::exit(code),
                        Err(e) => {
                            error_log!("Daemon control failed: {}", e);
                            std::process::exit(1);
                        }
                    },
                    None => {
                        error_log!("Error: --daemon requires an action ({})", Action::NAMES);
                        std::process::exit(1);
                    }
                }
            }
            "--set" => {
                if let Some(kv) = arg_iter.next() {
                    #[cfg(unix)]
                    crate::listener_context::send_context_var(kv);
                    #[cfg(not(unix))]
                    {
                        let _ = kv;
                        error_log!("Error: --set is not supported on this platform");
                        std::process::exit(1);
                    }
                } else {
                    error_log!("Error: --set requires a KEY=VALUE argument");
                    std::process::exit(1);
                }
            }
            "--integration" => {
                let name = arg_iter.next();
                let action = arg_iter.next();
                match (name, action) {
                    (Some(name), Some(action)) => {
                        std::process::exit(crate::integrations::run(name, action));
                    }
                    _ => {
                        error_log!(
                            "Error: --integration requires <NAME> <ACTION> (names: {}; actions: {})",
                            crate::integrations::NAMES,
                            crate::integrations::ACTIONS
                        );
                        std::process::exit(1);
                    }
                }
            }
            "--json" | "--verbose" | "-v" | "-vv" => {} // Processed elsewhere
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
