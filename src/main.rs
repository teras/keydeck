mod device_manager;

use crate::device_manager::DeviceManager;
use std::env;


fn main() {
    let args = env::args().skip(1).collect::<Vec<String>>();
    let mut arg_iter = args.iter();

    let mut manager = DeviceManager::new();

    while let Some(arg) = arg_iter.next() {
        match arg.as_str() {
            "--help" => println!("Usage: {} [options]\n\nOptions:\n  -l, --list\t\tList all connected devices\n  -d, --add <id>\tAdd a device with the given id\n  -r, --remove <id>\tRemove a device with the given id\n  -c, --current\t\tList all current devices\n  --button <index> <path>\tSet the image for a button\n  --clear\t\tClear all button images\n  --brightness <value>\tSet the brightness of all devices\n  --quiet\t\tDisable verbose logging\n  --verbose\t\tEnable verbose logging", env::args().next().unwrap()),
            "--list" | "-l" => manager.list_all_devices(),
            "-d" | "--add" => {
                if let Some(arg1) = arg_iter.next() {
                    manager.add_device(arg1.to_uppercase()).expect(format!("Failed to add device with id '{}'", arg1).as_str());
                } else {
                    eprintln!("Error: Adding device requires an argument");
                }
            }
            "-r" | "--remove" => {
                if let Some(arg1) = arg_iter.next() {
                    manager.remove_device(arg1.to_uppercase()).expect(format!("Failed to remove device with id '{}'", arg1).as_str());
                } else {
                    eprintln!("Error: Removing device requires an argument");
                }
            }
            "-c" | "--current" => manager.current_devices(),
            "--button" => {
                if let (Some(index), Some(path)) = (arg_iter.next(), arg_iter.next()) {
                    manager.set_button_image(index.parse().expect("Failed to parse button number"), path.to_string()).expect("Failed to set button image");
                } else {
                    eprintln!("Error: Setting button image requires two arguments, button number and image path");
                }
            }
            "--clear" => manager.clear_all_button_images().expect("Failed to clear all button images"),
            "--brightness" => {
                if let Some(arg1) = arg_iter.next() {
                    manager.set_brightness(arg1.parse().expect("Failed to parse brightness")).expect("Failed to set brightness");
                } else {
                    eprintln!("Error: Setting brightness requires an argument");
                }
            }
            "--quiet" => manager.verbose = false,
            "--verbose" => manager.verbose = true,
            _ => {
                eprintln!("Error: Unknown command '{}'", arg);
            }
        }
    }
}
