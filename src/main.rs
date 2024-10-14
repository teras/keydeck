mod device_manager;

use std::env;
use crate::device_manager::DeviceManager;


fn main() {
    let args = env::args().skip(1).collect::<Vec<String>>();
    let mut arg_iter = args.iter();

    let mut manager = DeviceManager::new();

    while let Some(arg) = arg_iter.next() {
        match arg.as_str() {
            "--help" => println!("Usage: streamdeck-rs [options]\n\nOptions:\n  -l, --list\t\tList all connected devices\n  -d, --add <id>\tAdd device with id\n  -r, --remove <id>\tRemove device with id\n  -c, --current\t\tList current devices\n  -b, --button <num> <image>\tSet button image\n"),
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
            "-b" | "--button" => {
                if let (Some(arg1), Some(arg2)) = (arg_iter.next(), arg_iter.next()) {
                    manager.set_button_image(arg1.parse().expect("Failed to parse button number"), arg2.to_string()).expect("Failed to set button image");
                } else {
                    eprintln!("Error: Setting button image requires two arguments, button number and image path");
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

fn parse_device_id(device_id: &str) -> Option<(u16, u16)> {
    let parts: Vec<&str> = device_id.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    Some((u16::from_str_radix(parts[0], 16).ok()?, u16::from_str_radix(parts[1], 16).ok()?))
}