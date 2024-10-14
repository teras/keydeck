use crate::parse_device_id;
use elgato_streamdeck::{list_devices, new_hidapi, StreamDeck};
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
    pub fn set_button_image(&mut self, p0: String, p1: String) -> Result<(), String> {
        if self.device_ids.len() == 0 {
            self.add_all_devices()?;
        }
        if self.device_ids.len() == 0 {
            return Err("No devices connected".to_string());
        }
        println!("Setting button image {} {}", p0, p1);
        Ok(())
    }
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            device_ids: IndexMap::new(),
            verbose: true,
        }
    }

    pub fn list_all_devices(&self) {
        match new_hidapi() {
            Ok(hid) => {
                verbose_log!(self, "Found devices:");
                for (kind, serial) in list_devices(&hid) {
                    verbose_log!(self, format!("{:04X}:{:04X} {} {:?}", kind.vendor_id(), kind.product_id(), serial, kind));
                }
                verbose_log!(self, "--------------");
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
            self.device_ids.remove(device_id.as_str());
            verbose_log!(self, format!("Removed device with id '{}'", device_id));
            Ok(())
        } else {
            Err(format!("Device with id '{}' not found", device_id))
        }
    }
}
