mod device_manager;

use crate::device_manager::DeviceManager;
use std::env;


fn main() {
    let args = env::args().skip(1).collect::<Vec<String>>();
    let mut arg_iter = args.iter();

    let mut manager = DeviceManager::new();

    while let Some(arg) = arg_iter.next() {
        match arg.as_str() {
            "--help" => println!("Usage: streamdeck-rs [options]\nOptions:\n-i, --image <button number> <image path> - Set the image for a button\n-d, --img-dir <directory path> - Set the directory where images are searched\n-c, --clear - Clear all button images\n-b, --brightness <brightness> - Set the brightness of the device\n--enable <device id> - Enable a device\n--disable <device id> - Disable a device\n--list-all - List all devices\n--list-enabled - List enabled devices\n--quiet - Disable verbose output\n--verbose - Enable verbose output"),
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
            "--reset" => manager.reset_devices().unwrap(),
            "--shutdown" => manager.shutdown_devices().unwrap(),
            "--list" => manager.list_devices(),
            "--quiet" => manager.verbose = false,
            "--verbose" => manager.verbose = true,
            _ => {
                eprintln!("Error: Unknown command '{}'", arg);
            }
        }
    }
}
