use elgato_streamdeck::info::Kind;
use elgato_streamdeck::{list_devices, new_hidapi, DeviceStateReader, StreamDeck};
use hidapi::HidApi;
use image::open;
use std::sync::Arc;
use std::time::Duration;

#[macro_export]
macro_rules! verbose_log {
    ($self:expr, $msg:expr) => {
        if $self.verbose {
            println!("{}", $msg);
        }
    };
}

pub struct DeviceManager {
    // hid_api: Arc<HidApi>,
    devices: Vec<StreamDeckDevice>,
    pub verbose: bool,
    image_dir: Option<String>,
    auto_added: bool,
}

pub struct StreamDeckDevice {
    hid_api: Arc<HidApi>,
    kind: Kind,
    serial: String,
    device_id: String,
    deck: Option<Arc<StreamDeck>>,
    reader: Option<Arc<DeviceStateReader>>,
    enabled: bool,
}

impl StreamDeckDevice {
    pub fn get_deck(&mut self) -> Arc<StreamDeck> {
        self.deck.get_or_insert_with(|| {
            Arc::new(
                StreamDeck::connect(&self.hid_api, self.kind, &self.serial)
                    .expect("Failed to connect to device"),
            )
        }).clone()  // Return a clone of the Arc<StreamDeck>
    }

    pub fn get_reader(&mut self) -> Arc<DeviceStateReader> {
        if self.reader.is_none() {
            let deck = self.get_deck();
            self.reader = Some(deck.get_reader());
        }
        self.reader.as_ref().unwrap().clone()
    }
}

impl DeviceManager {
    pub fn new() -> Self {
        let hidapi = Arc::new(new_hidapi().ok().expect("Failed to create hidapi context"));
        let mut devices: Vec<StreamDeckDevice> = vec![];
        for (kind, serial) in list_devices(&hidapi) {
            let device_id = format!("{:04X}:{:04X}", kind.vendor_id(), kind.product_id());
            devices.push(
                StreamDeckDevice {
                    hid_api: Arc::clone(&hidapi),
                    kind: kind,
                    serial: serial,
                    device_id: device_id,
                    deck: None,
                    reader: None,
                    enabled: false,
                }
            );
        }
        if devices.is_empty() {
            panic!("No devices connected");
        }
        Self {
            devices: devices,
            verbose: false,
            image_dir: None,
            auto_added: true,
        }
    }

    pub fn grab_event(&mut self) -> Result<(), String> {
        if self.count_active_devices() != 1 {
            return Err("Only one active device is allowed to grab events".to_string());
        }
        for device in self.iter_active_devices() {
            if let Ok(updates) = device.get_reader().read(Some(Duration::from_secs_f64(100.0))) {
                for update in updates {
                    println!("{:?}", update);
                }
            }
        }
        Ok(())
    }

    pub fn shutdown_devices(&mut self) -> Result<(), String> {
        for device in self.iter_active_devices() {
            device.get_deck().shutdown().map_err(|e| format!("Failed to shutdown device '{}': {}", device.get_deck().serial_number().unwrap(), e))?;
        }
        verbose_log!(self, "Shutdown devices");
        Ok(())
    }

    pub(crate) fn reset_devices(&mut self) -> Result<(), String> {
        for device in self.iter_active_devices() {
            device.get_deck().reset().map_err(|e| format!("Failed to reset device '{}': {}", device.get_deck().serial_number().unwrap(), e))?;
        }
        verbose_log!(self, "Reset devices");
        Ok(())
    }

    pub(crate) fn sleep_devices(&mut self) -> Result<(), String> {
        for device in self.iter_active_devices() {
            device.get_deck().sleep().map_err(|e| format!("Failed to sleep device '{}': {}", device.get_deck().serial_number().unwrap(), e))?;
        }
        verbose_log!(self, "Sleep devices");
        Ok(())
    }

    pub(crate) fn set_logo_image(&mut self, logo_image: String) -> Result<(), String> {
        let image = load_image(logo_image.clone(), self.image_dir.clone());
        for device in self.iter_active_devices() {
            let deck = device.get_deck();
            deck.set_logo_image(image.clone()).map_err(|e| format!("Failed to set logo image on device '{}': {}", deck.serial_number().unwrap(), e))?;
            deck.flush().map_err(|e| format!("Failed to flush logo image on device '{}': {}", deck.serial_number().unwrap(), e))?;
        }
        verbose_log!(self, format!("Set logo image {}", logo_image));
        Ok(())
    }

    pub fn clear_button_image(&mut self, button_idx: u8) -> Result<(), String> {
        for device in self.iter_active_devices() {
            let deck = device.get_deck();
            deck.clear_button_image(button_idx).map_err(|e| format!("Failed to clear button image on device '{}' from button {}: {}", deck.serial_number().unwrap(), button_idx, e))?;
        }
        verbose_log!(self, format!("Cleared button image from button {}", button_idx));
        Ok(())
    }

    pub fn set_button_image(&mut self, button_idx: u8, img_path: String) -> Result<(), String> {
        let image = load_image(img_path.clone(), self.image_dir.clone());
        for device in self.iter_active_devices() {
            let deck = device.get_deck();
            deck.set_button_image(button_idx, image.clone()).map_err(|e| format!("Failed to set button image {} on device '{}' to button {}: {}", img_path, deck.serial_number().unwrap(), button_idx, e))?;
            deck.flush().map_err(|e| format!("Failed to flush button image {} on device '{}' to button {}: {}", img_path, deck.serial_number().unwrap(), button_idx, e))?;
        }
        verbose_log!(self, format!("Set button image {} to button {}", img_path, button_idx));
        Ok(())
    }

    pub fn set_image_dir(&mut self, img_dir: String) {
        self.image_dir = Some(img_dir);
    }

    pub fn set_brightness(&mut self, brightness: u8) -> Result<(), String> {
        for device in self.iter_active_devices() {
            let deck = device.get_deck();
            deck.set_brightness(brightness).map_err(|e| format!("Failed to set brightness on device '{}': {}", deck.serial_number().unwrap(), e))?;
        }
        verbose_log!(self, format!("Set brightness to {}", brightness));
        Ok(())
    }

    pub fn clear_all_button_images(&mut self) -> Result<(), String> {
        for device in self.iter_active_devices() {
            let deck = device.get_deck();
            deck.clear_all_button_images().map_err(|e| format!("Failed to clear all button images on device '{}': {}", deck.serial_number().unwrap(), e))?;
            deck.flush().map_err(|e| format!("Failed to flush clear all button images on device '{}': {}", deck.serial_number().unwrap(), e))?;
        }
        verbose_log!(self, "Cleared all button images");
        Ok(())
    }

    pub fn list_devices(&mut self) {
        for device in self.iter_active_devices() {
            println!("{} {} {:?}", device.device_id, device.serial, device.kind);
        }
        println!("Total devices: {}", self.count_active_devices());
    }

    pub fn enable_device(&mut self, identifier: String) -> Result<(), String> {
        if self.auto_added {
            self.set_state_all_devices(false);
            self.auto_added = false;
        }
        for device in &mut self.devices {
            if device.device_id == identifier || device.serial.trim() == identifier {
                device.enabled = true;
                return Ok(());
            }
        }
        Err(format!("Enabling device with id '{}' not found", identifier))
    }

    pub fn disable_device(&mut self, device_id: String) -> Result<(), String> {
        self.auto_added = false;
        for device in &mut self.devices {
            if device.device_id == device_id || device.serial.trim() == device_id {
                device.enabled = false;
                return Ok(());
            }
        }
        Err(format!("Disabling device with id '{}' not found", device_id))
    }

    fn set_state_all_devices(&mut self, state: bool) {
        for device in &mut self.devices {
            device.enabled = state;
        }
    }

    fn count_active_devices(&self) -> usize {
        let mut count = 0;
        for device in self.devices.iter() {
            if device.enabled {
                count += 1;
            }
        }
        count
    }

    fn iter_active_devices(&mut self) -> impl Iterator<Item=&mut StreamDeckDevice> {
        if self.count_active_devices() == 0 {
            self.set_state_all_devices(true);
            self.auto_added = true;
        }
        self.devices.iter_mut().filter(|device| device.enabled)
    }
}

fn load_image(image: String, img_path: Option<String>) -> image::DynamicImage {
    if std::path::Path::new(&image).exists() {
        return open(image).unwrap();
    }
    let img_path = img_path.unwrap_or_else(|| ".".to_string());
    let image = format!("{}/{}", img_path, image.replace("\\", "/"));
    open(image).unwrap()
}


