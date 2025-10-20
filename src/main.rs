mod device_manager;
mod server;
mod pages;
mod focus_property;
mod focus_property_wayland;
mod event;
mod listener_button;
mod listener_device;
mod listener_time;
mod paged_device;
mod utils;
mod listener_focus;
mod listener_focus_wayland;
mod kwin_script;
mod listener_tick;
mod listener_sleep;
mod listener_signal;
mod keyboard;
mod lock;
mod session;
mod text_renderer;

use crate::device_manager::DeviceManager;
use crate::server::start_server;
use std::env;
use std::sync::atomic::AtomicBool;

pub static DEBUG: AtomicBool = AtomicBool::new(false);

fn print_help() {
    println!("Usage: keydeck [OPTION]...");
    println!("Control a Stream Deck or similar device");
    println!();
    println!("Options:");
    println!("  -g, --grab                  Grab the next event (note, it can be more than one)");
    println!("  -i, --image <BUTTON> <PATH> Set the image for a button");
    println!("  -d, --img-dir <DIR>         Set the directory where the images are searched");
    println!("  -c, --clear                 Clear all button images");
    println!("  -cb, --clear-button <BUTTON> Clear the image for a button");
    println!("  -b, --brightness <BRIGHTNESS> Set the brightness of the device");
    println!("  -l, --logo <PATH>           Set the logo image");
    println!("  -s, --sleep                 Put the device to sleep");
    println!("  -1, --enable <DEVICE>       Enable a device");
    println!("  -0, --disable <DEVICE>      Disable a device");
    println!("      --flush                 Flush devices");
    println!("      --reset                 Reset devices");
    println!("      --shutdown              Shutdown devices");
    println!("      --list                  List all devices");
    println!("      --quiet                 Do not print verbose messages");
    println!("      --verbose               Print verbose messages");
    println!("      --server                Start the server");
    println!("      --help                  Display this help and exit");
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<String>>();

    // First pass: process flags (--verbose, --quiet) before anything else
    for arg in &args {
        match arg.as_str() {
            "--quiet" => DEBUG.store(false, std::sync::atomic::Ordering::Relaxed),
            "--verbose" => DEBUG.store(true, std::sync::atomic::Ordering::Relaxed),
            _ => {}
        }
    }

    let mut arg_iter = args.iter();
    let mut manager = DeviceManager::new();
    let mut should_start_server = false;

    while let Some(arg) = arg_iter.next() {
        match arg.as_str() {
            "--help" => print_help(),
            "-g" | "--grab" => {
                if let Err(e) = manager.grab_event() {
                    error_log!("Error: {}", e);
                }
            }
            "-i" | "--image" => {
                if let (Some(index), Some(path)) = (arg_iter.next(), arg_iter.next()) {
                    match index.parse() {
                        Ok(button_num) => {
                            if let Err(e) = manager.set_button_image(button_num, path.to_string()) {
                                error_log!("Error: {}", e);
                            }
                        }
                        Err(_) => error_log!("Error: Invalid button number '{}', expected a number", index),
                    }
                } else {
                    error_log!("Error: Setting button image requires two arguments, button number and image path");
                }
            }
            "-d" | "--img-dir" => {
                if let Some(arg1) = arg_iter.next() {
                    manager.set_image_dir(arg1.to_string());
                } else {
                    error_log!("Error: Setting the directory where the images are searched requires an argument");
                }
            }
            "-c" | "--clear" => {
                if let Err(e) = manager.clear_all_button_images() {
                    error_log!("Error: {}", e);
                }
            }
            "-cb" | "--clear-button" => {
                if let Some(arg1) = arg_iter.next() {
                    match arg1.parse() {
                        Ok(button_num) => {
                            if let Err(e) = manager.clear_button_image(button_num) {
                                error_log!("Error: {}", e);
                            }
                        }
                        Err(_) => error_log!("Error: Invalid button number '{}', expected a number", arg1),
                    }
                } else {
                    error_log!("Error: Clearing button image requires an argument");
                }
            }
            "-b" | "--brightness" => {
                if let Some(arg1) = arg_iter.next() {
                    match arg1.parse() {
                        Ok(brightness) => {
                            if let Err(e) = manager.set_brightness(brightness) {
                                error_log!("Error: {}", e);
                            }
                        }
                        Err(_) => error_log!("Error: Invalid brightness '{}', expected a number 0-100", arg1),
                    }
                } else {
                    error_log!("Error: Setting brightness requires an argument");
                }
            }
            "-l" | "--logo" => {
                if let Some(arg1) = arg_iter.next() {
                    if let Err(e) = manager.set_logo_image(arg1.to_string()) {
                        error_log!("Error: {}", e);
                    }
                } else {
                    error_log!("Error: Setting logo image requires an argument");
                }
            }
            "-s" | "--sleep" => {
                if let Err(e) = manager.sleep_devices() {
                    error_log!("Error: {}", e);
                }
            }
            "-1" | "--enable" => {
                if let Some(arg1) = arg_iter.next() {
                    if let Err(e) = manager.enable_device(arg1.to_uppercase()) {
                        error_log!("Error: {}", e);
                    }
                } else {
                    error_log!("Error: Adding device requires an argument");
                }
            }
            "-0" | "--disable" => {
                if let Some(arg1) = arg_iter.next() {
                    if let Err(e) = manager.disable_device(arg1.to_uppercase()) {
                        error_log!("Error: {}", e);
                    }
                } else {
                    error_log!("Error: Removing device requires an argument");
                }
            }
            "--flush" => {
                if let Err(e) = manager.flush_devices() {
                    error_log!("Error: {}", e);
                }
            }
            "--reset" => {
                if let Err(e) = manager.reset_devices() {
                    error_log!("Error: {}", e);
                }
            }
            "--shutdown" => {
                if let Err(e) = manager.shutdown_devices() {
                    error_log!("Error: {}", e);
                }
            }
            "--list" => manager.list_devices(),
            "--quiet" | "--verbose" => {}, // Already processed in first pass
            "--server" => should_start_server = true,
            _ => {
                error_log!("Error: Unknown command '{}'", arg);
            }
        }
    }
    if args.len() == 0 || should_start_server {
        start_server();
    }
}
