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
            "-i" | "--image" => {
                if let (Some(index), Some(path)) = (arg_iter.next(), arg_iter.next()) {
                    manager.set_button_image(index.parse().expect("Failed to parse button number"), path.to_string()).expect("Failed to set button image");
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
            "-c" | "--clear" => manager.clear_all_button_images().expect("Failed to clear all button images"),
            "-b" | "--brightness" => {
                if let Some(arg1) = arg_iter.next() {
                    manager.set_brightness(arg1.parse().expect("Failed to parse brightness")).expect("Failed to set brightness");
                } else {
                    eprintln!("Error: Setting brightness requires an argument");
                }
            }
            "-1" | "--enable" => {
                if let Some(arg1) = arg_iter.next() {
                    manager.enable_device(arg1.to_uppercase()).expect(format!("Failed to add device with id '{}'", arg1).as_str());
                } else {
                    eprintln!("Error: Adding device requires an argument");
                }
            }
            "-0" | "--disable" => {
                if let Some(arg1) = arg_iter.next() {
                    manager.disable_device(arg1.to_uppercase()).expect(format!("Failed to remove device with id '{}'", arg1).as_str());
                } else {
                    eprintln!("Error: Removing device requires an argument");
                }
            }
            "--list" => manager.list_devices(),
            "--quiet" => manager.verbose = false,
            "--verbose" => manager.verbose = true,
            _ => {
                eprintln!("Error: Unknown command '{}'", arg);
            }
        }
    }
}
