use elgato_streamdeck::{list_devices, new_hidapi, StreamDeck};
use image::open;
use indexmap::IndexMap;

#[macro_export]
macro_rules! verbose_log {
    ($self:expr, $msg:expr) => {
        if $self.verbose {
            println!("{}", $msg);
        }
    };
}

pub struct DeviceManager {
    device_ids: IndexMap<String, StreamDeck>,
    pub verbose: bool,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            device_ids: IndexMap::new(),
            verbose: false,
        }
    }

    pub fn list_all_devices(&self) {
        match new_hidapi() {
            Ok(hid) => {
                println!("Found devices:");
                for (kind, serial) in list_devices(&hid) {
                    println!("{:04X}:{:04X} {} {:?}", kind.vendor_id(), kind.product_id(), serial, kind);
                }
                println!("--------------");
            }
            Err(e) => {
                eprintln!("Unable to create hidapi context: {}", e);
            }
        }
    }

    pub fn current_devices(&self) {
        if self.device_ids.len() == 0 {
            verbose_log!(self, "No devices connected");
        } else {
            for (device_id, _) in self.device_ids.iter() {
                verbose_log!(self, format!( "Using device: {}", device_id));
            }
        }
    }

    fn add_all_devices(&mut self) -> Result<(), String> {
        match new_hidapi() {
            Ok(hid) => {
                for (kind, serial) in list_devices(&hid) {
                    let device_id = format!("{:04X}:{:04X}", kind.vendor_id(), kind.product_id());
                    self.device_ids.insert(
                        device_id.to_string(),
                        StreamDeck::connect(&hid, kind, &serial)
                            .expect(&format!("Failed to connect to device with id '{}'", device_id)),
                    );
                    verbose_log!(self, format!("Connected to '{}'", serial));
                }
                Ok(())
            }
            Err(e) => {
                Err(format!("Unable to create hidapi context: {}", e))
            }
        }
    }

    pub fn add_device(&mut self, device_id: String) -> Result<(), String> {
        if self.device_ids.contains_key(device_id.as_str()) {
            return Ok(());
        }
        match parse_device_id(device_id.as_str()) {
            Some((vendor_id, product_id)) => {
                match new_hidapi() {
                    Ok(hid) => {
                        for (kind, serial) in list_devices(&hid) {
                            if kind.vendor_id() == vendor_id && kind.product_id() == product_id {
                                self.device_ids.insert(
                                    device_id.to_string(),
                                    StreamDeck::connect(&hid, kind, &serial)
                                        .expect(&format!("Failed to connect to device with id '{}'", device_id)),
                                );
                                verbose_log!(self, format!("Connected to '{}'", serial));
                                return Ok(());
                            }
                        }
                        Err(format!("Device with id '{}' not found", device_id))
                    }
                    Err(e) => {
                        Err(format!("Unable to create hidapi context: {}", e))
                    }
                }
            }
            None => {
                Err(format!("Invalid device ID '{}'", device_id))
            }
        }
    }

    pub fn remove_device(&mut self, device_id: String) -> Result<(), String> {
        if self.device_ids.contains_key(device_id.as_str()) {
            self.device_ids.swap_remove(device_id.as_str());
            verbose_log!(self, format!("Removed device with id '{}'", device_id));
            Ok(())
        } else {
            Err(format!("Device with id '{}' not found", device_id))
        }
    }

    pub fn set_button_image(&mut self, button_idx: u8, img_path: String) -> Result<(), String> {
        self.ensure_devices()?;
        let image = open(img_path.clone()).unwrap();
        for (_, device) in self.device_ids.iter_mut() {
            device.set_button_image(button_idx, image.clone()).expect(format!("Failed to set button image {} on device '{}' to button {}", img_path, device.serial_number().unwrap(), button_idx).as_str());
            device.flush().expect(format!("Failed to flush button image {} on device '{}' to button {}", img_path, device.serial_number().unwrap(), button_idx).as_str());
        }
        verbose_log!(self, format!("Set button image {} to button {}", img_path, button_idx));
        Ok(())
    }

    pub fn set_brightness(&mut self, brightness: u8) -> Result<(), String> {
        self.ensure_devices()?;
        for (_, device) in self.device_ids.iter_mut() {
            device.set_brightness(brightness).expect(format!("Failed to set brightness on device '{}'", device.serial_number().unwrap()).as_str());
        }
        verbose_log!(self, format!("Set brightness to {}", brightness));
        Ok(())
    }

    pub fn clear_all_button_images(&mut self) -> Result<(), String> {
        self.ensure_devices()?;
        for (_, device) in self.device_ids.iter_mut() {
            device.clear_all_button_images().expect(format!("Failed to clear all button images on device '{}'", device.serial_number().unwrap()).as_str());
            device.flush().expect(format!("Failed to flush clear all button images on device '{}'", device.serial_number().unwrap()).as_str());
        }
        verbose_log!(self, "Cleared all button images");
        Ok(())
    }

    fn ensure_devices(&mut self) -> Result<(), String> {
        if self.device_ids.len() == 0 {
            self.add_all_devices()?;
        }
        if self.device_ids.len() == 0 {
            return Err("No devices connected".to_string());
        }
        Ok(())
    }
}

fn parse_device_id(device_id: &str) -> Option<(u16, u16)> {
    let parts: Vec<&str> = device_id.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    Some((u16::from_str_radix(parts[0], 16).ok()?, u16::from_str_radix(parts[1], 16).ok()?))
}