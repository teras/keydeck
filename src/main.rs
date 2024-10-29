mod device_manager;
mod server;
mod pages;
mod focus_property;
mod event;
mod key_listener;
mod device_listener;
mod paged_device;
mod utils;
mod focus_listener;
mod tick_device;

use crate::device_manager::DeviceManager;
use crate::server::start_server;
use std::env;
use std::sync::atomic::AtomicBool;

pub static DEBUG: AtomicBool = AtomicBool::new(false);

fn print_help() {
    println!("Usage: streamdeck [OPTION]...");
    println!("Control an Elgato Stream Deck device");
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
    let mut arg_iter = args.iter();

    let mut manager = DeviceManager::new();

    while let Some(arg) = arg_iter.next() {
        match arg.as_str() {
            "--help" => print_help(),
            "-g" | "--grab" => manager.grab_event().unwrap(),
            "-i" | "--image" => {
                if let (Some(index), Some(path)) = (arg_iter.next(), arg_iter.next()) {
                    manager.set_button_image(index.parse().expect("Failed to parse button number"), path.to_string()).unwrap();
                } else {
                    eprintln!("Error: Setting button image requires two arguments, button number and image path");
                }
            }
            "-d" | "--img-dir" => {
                if let Some(arg1) = arg_iter.next() {
                    manager.set_image_dir(arg1.to_string());
                } else {
                    eprintln!("Error: Setting the directory where the images are searched requires an argument");
                }
            }
            "-c" | "--clear" => manager.clear_all_button_images().unwrap(),
            "-cb" | "--clear-button" => {
                if let Some(arg1) = arg_iter.next() {
                    manager.clear_button_image(arg1.parse().expect("Failed to parse button number")).unwrap();
                } else {
                    eprintln!("Error: Clearing button image requires an argument");
                }
            }
            "-b" | "--brightness" => {
                if let Some(arg1) = arg_iter.next() {
                    manager.set_brightness(arg1.parse().expect("Failed to parse brightness")).unwrap();
                } else {
                    eprintln!("Error: Setting brightness requires an argument");
                }
            }
            "-l" | "--logo" => {
                if let Some(arg1) = arg_iter.next() {
                    manager.set_logo_image(arg1.to_string()).unwrap();
                } else {
                    eprintln!("Error: Setting logo image requires an argument");
                }
            }
            "-s" | "--sleep" => manager.sleep_devices().unwrap(),
            "-1" | "--enable" => {
                if let Some(arg1) = arg_iter.next() {
                    manager.enable_device(arg1.to_uppercase()).unwrap();
                } else {
                    eprintln!("Error: Adding device requires an argument");
                }
            }
            "-0" | "--disable" => {
                if let Some(arg1) = arg_iter.next() {
                    manager.disable_device(arg1.to_uppercase()).unwrap();
                } else {
                    eprintln!("Error: Removing device requires an argument");
                }
            }
            "--flush" => manager.flush_devices().unwrap(),
            "--reset" => manager.reset_devices().unwrap(),
            "--shutdown" => manager.shutdown_devices().unwrap(),
            "--list" => manager.list_devices(),
            "--quiet" => DEBUG.store(false, std::sync::atomic::Ordering::Relaxed),
            "--verbose" => DEBUG.store(true, std::sync::atomic::Ordering::Relaxed),
            "--server" => start_server(),
            _ => {
                eprintln!("Error: Unknown command '{}'", arg);
            }
        }
    }
    if args.len() == 0 {
        println!("Starting server");
        start_server();
    }
}
