use elgato_streamdeck::info::Kind;
use elgato_streamdeck::{list_devices, new_hidapi, DeviceStateReader, StreamDeck};
use hidapi::HidApi;
use image::{open, DynamicImage};
use std::cell::RefCell;
use std::path::Path;
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
    devices: Vec<StreamDeckDevice>,
    image_dir: Option<String>,
    auto_added: bool,
}

pub struct StreamDeckDevice {
    hid_api: Arc<HidApi>,
    kind: Kind,
    serial: String,
    device_id: String,
    deck: RefCell<Option<Arc<StreamDeck>>>,
    reader: RefCell<Option<Arc<DeviceStateReader>>>,
    enabled: bool,
    pub verbose: bool,
}

impl StreamDeckDevice {
    pub fn get_deck(&self) -> Arc<StreamDeck> {
        self.deck.borrow_mut().get_or_insert_with(|| {
            Arc::new(
                StreamDeck::connect(&self.hid_api, self.kind, &self.serial)
                    .expect("Failed to connect to device"),
            )
        }).clone()  // Return a clone of the Arc<StreamDeck>
    }

    pub fn get_reader(&self) -> Arc<DeviceStateReader> {
        // Borrow mutably to check or initialize `reader`
        if self.reader.borrow().is_none() {
            let deck = self.get_deck();
            // Borrow mutably and set `reader`
            *self.reader.borrow_mut() = Some(deck.get_reader());
        }

        // Borrow immutably to clone the reader
        self.reader.borrow().as_ref().unwrap().clone()
    }

    pub fn shutdown(&self) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!(self, format!("Shutting down device '{}'", self.serial));
        deck.shutdown().map_err(|e| format!("Failed to shutdown device '{}': {}", deck.serial_number().unwrap(), e))
    }

    pub fn reset(&self) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!(self, format!("Resetting device '{}'", self.serial));
        deck.reset().map_err(|e| format!("Failed to reset device '{}': {}", deck.serial_number().unwrap(), e))
    }

    pub fn sleep(&self) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!(self, format!("Sleeping device '{}'", self.serial));
        deck.sleep().map_err(|e| format!("Failed to sleep device '{}': {}", deck.serial_number().unwrap(), e))
    }

    pub fn set_logo_image_cached(&self, image: DynamicImage) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!(self, format!("Setting logo image on device '{}'",  deck.serial_number().unwrap()));
        deck.set_logo_image(image).map_err(|e| format!("Failed to set logo image on device '{}': {}", deck.serial_number().unwrap(), e))
    }

    pub fn set_logo_image(&self, image_dir: Option<String>, logo_image: String) -> Result<(), String> {
        verbose_log!(self, format!("Set logo image on device '{}'", self.serial));
        let img_data = load_image(&logo_image, image_dir.clone())?;
        self.get_deck().set_logo_image(img_data).map_err(|e| format!("Failed to set logo image on device '{}': {}", self.serial, e))
    }

    pub fn clear_button_image(&self, button_idx: u8) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!(self, format!("Clearing button image on device '{}' from button {}", deck.serial_number().unwrap(), button_idx));
        deck.clear_button_image(button_idx).map_err(|e| format!("Failed to clear button image on device '{}' from button {}: {}", deck.serial_number().unwrap(), button_idx, e))
    }

    pub fn set_button_image_cached(&self, button_idx: u8, image: DynamicImage) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!(self, format!("Setting button image on device '{}' to button {}", deck.serial_number().unwrap(), button_idx));
        deck.set_button_image(button_idx, image).map_err(|e| format!("Failed to set button image on device '{}' to button {}: {}", deck.serial_number().unwrap(), button_idx, e))
    }

    pub fn set_button_image(&self, button_idx: u8, image_dir: Option<String>, image_name: String) -> Result<(), String> {
        let deck = self.get_deck();
        let image_data = load_image(&image_name, image_dir)?;
        verbose_log!(self, format!("Setting button image {} on device '{}' to button {}", image_name, deck.serial_number().unwrap(), button_idx));
        deck.set_button_image(button_idx, image_data).map_err(|e| format!("Failed to set button image {} on device '{}' to button {}: {}", image_name, deck.serial_number().unwrap(), button_idx, e))
    }

    pub fn flush(&self) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!(self, format!("Flushing device '{}'", deck.serial_number().unwrap()));
        deck.flush().map_err(|e| format!("Failed to flush device '{}': {}", deck.serial_number().unwrap(), e))
    }

    pub fn set_brightness(&self, brightness: u8) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!(self, format!("Setting brightness {} on device '{}'", brightness, deck.serial_number().unwrap()));
        deck.set_brightness(brightness).map_err(|e| format!("Failed to set brightness on device '{}': {}", deck.serial_number().unwrap(), e))
    }

    pub fn clear_all_button_images(&self) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!(self, format!("Cleared all button images on device '{}'", deck.serial_number().unwrap()));
        deck.clear_all_button_images().map_err(|e| format!("Failed to clear all button images on device '{}': {}", deck.serial_number().unwrap(), e))
    }

    pub fn get_button_count(&self) -> u8 {
        self.kind.key_count()
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
                    deck: RefCell::new(None),
                    reader: RefCell::new(None),
                    enabled: true,
                    verbose: false,
                }
            );
        }
        if devices.is_empty() {
            panic!("No devices connected");
        }
        Self {
            devices: devices,
            image_dir: None,
            auto_added: true,
        }
    }

    pub fn grab_event(&mut self) -> Result<(), String> {
        let active = self.count_active_devices();
        if self.count_active_devices() != 1 {
            return Err(format!("Only one active device is allowed to grab events, found {}", active));
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
            device.shutdown()?;
        }
        Ok(())
    }

    pub(crate) fn reset_devices(&mut self) -> Result<(), String> {
        for device in self.iter_active_devices() {
            device.reset()?;
        }
        Ok(())
    }

    pub(crate) fn sleep_devices(&mut self) -> Result<(), String> {
        for device in self.iter_active_devices() {
            device.sleep()?;
        }
        Ok(())
    }

    pub(crate) fn set_logo_image(&mut self, logo_image: String) -> Result<(), String> {
        let image_data = load_image(&logo_image, self.image_dir.clone())?;
        for device in self.iter_active_devices() {
            device.set_logo_image_cached(image_data.clone())?;
            device.flush()?;
        }
        Ok(())
    }

    pub fn clear_button_image(&mut self, button_idx: u8) -> Result<(), String> {
        for device in self.iter_active_devices() {
            device.clear_button_image(button_idx)?;
        }
        Ok(())
    }

    pub fn flush_devices(&mut self) -> Result<(), String> {
        for device in self.iter_active_devices() {
            device.flush()?;
        }
        Ok(())
    }

    pub fn set_button_image(&mut self, button_idx: u8, img_path: String) -> Result<(), String> {
        let image_data = load_image(&img_path, self.image_dir.clone())?;
        for device in self.iter_active_devices() {
            device.set_button_image_cached(button_idx, image_data.clone())?;
            device.flush()?;
        }
        Ok(())
    }

    pub fn set_image_dir(&mut self, img_dir: String) {
        self.image_dir = Some(img_dir);
    }

    pub fn set_brightness(&mut self, brightness: u8) -> Result<(), String> {
        for device in self.iter_active_devices() {
            device.set_brightness(brightness)?;
        }
        Ok(())
    }

    pub fn clear_all_button_images(&mut self) -> Result<(), String> {
        for device in self.iter_active_devices() {
            device.clear_all_button_images()?;
        }
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

    pub fn set_verbose(&mut self, verbose: bool) {
        for device in self.devices.iter_mut() {
            device.verbose = verbose;
        }
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

    pub fn iter_active_devices(&mut self) -> impl Iterator<Item=&mut StreamDeckDevice> {
        self.devices.iter_mut().filter(|device| device.enabled)
    }
}

pub fn load_image(image: &str, img_path: Option<String>) -> Result<DynamicImage, String> {
    if Path::new(image).exists() {
        open(image).map_err(|e| format!("Failed to load image {}: {}", image, e))
    } else {
        let image_path = format!("{}/{}", img_path.unwrap_or_else(|| ".".to_string()), image.replace("\\", "/"));
        open(&image_path).map_err(|e| format!("Failed to load image {}: {}", image_path, e))
    }
}

pub fn find_device_by_serial(device_sn: &str) -> Option<StreamDeckDevice> {
    let hidapi = Arc::new(new_hidapi().ok().expect("Failed to create hidapi context"));
    for (kind, serial) in list_devices(&hidapi) {
        if serial == device_sn {
            let device_id = format!("{:04X}:{:04X}", kind.vendor_id(), kind.product_id());
            return Some(StreamDeckDevice {
                hid_api: Arc::clone(&hidapi),
                kind,
                serial,
                device_id,
                deck: RefCell::new(None),
                reader: RefCell::new(None),
                enabled: true,
                verbose: false,
            });
        }
    }
    None
}