use crate::device_trait::{DeviceError, DeviceReader, DeviceStateUpdate, KeydeckDevice};
use crate::{error_log, verbose_log};
use elgato_streamdeck::info::Kind;
use elgato_streamdeck::{DeviceStateReader, StreamDeck};
use hidapi::HidApi;
use image::DynamicImage;
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;

pub struct ElgatoDevice {
    pub(crate) hid_api: Arc<HidApi>,
    pub(crate) kind: Kind,
    pub serial: String,
    device_id: String,
    pub(crate) deck: RefCell<Option<Arc<StreamDeck>>>,
    pub(crate) reader: RefCell<Option<Arc<DeviceStateReader>>>,
    enabled: bool,
}

// SAFETY: ElgatoDevice is safe to Send/Sync because:
// - RefCell is only used for lazy initialization (get_deck, get_reader_arc)
// - The actual StreamDeck/DeviceStateReader are Arc-wrapped and can be safely shared
// - Access patterns ensure no concurrent mutation (initialized once per access)
unsafe impl Send for ElgatoDevice {}
unsafe impl Sync for ElgatoDevice {}

impl ElgatoDevice {
    /// Check if a device with given VID/PID is supported by Elgato library
    /// Elgato Stream Deck devices use VID 0x0fd9
    pub fn is_supported(vid: u16, _pid: u16) -> bool {
        vid == 0x0fd9
    }

    pub fn new(
        hid_api: Arc<HidApi>,
        kind: Kind,
        serial: String,
        device_id: String,
    ) -> Self {
        Self {
            hid_api,
            kind,
            serial,
            device_id,
            deck: RefCell::new(None),
            reader: RefCell::new(None),
            enabled: true,
        }
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn get_deck(&self) -> Arc<StreamDeck> {
        self.deck.borrow_mut().get_or_insert_with(|| {
            Arc::new(
                StreamDeck::connect(&self.hid_api, self.kind, &self.serial)
                    .unwrap_or_else(|e| {
                        error_log!("Failed to connect to Stream Deck device '{}': {}", self.serial, e);
                        error_log!("This may be due to:");
                        error_log!("  - Device was unplugged");
                        error_log!("  - Insufficient USB permissions");
                        error_log!("  - Device busy/in use by another process");
                        panic!("Cannot continue without device connection");
                    }),
            )
        }).clone()
    }

    pub fn get_reader_arc(&self) -> Arc<DeviceStateReader> {
        // Borrow mutably to check or initialize `reader`
        if self.reader.borrow().is_none() {
            let deck = self.get_deck();
            // Borrow mutably and set `reader`
            *self.reader.borrow_mut() = Some(deck.get_reader());
        }

        // Borrow immutably to clone the reader - safe because we just initialized it above
        self.reader.borrow().as_ref().expect("Reader should be initialized").clone()
    }

    pub fn reset(&self) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!("Resetting device '{}'", self.serial);
        deck.reset().map_err(|e| format!("Failed to reset device '{}': {}", self.serial, e))
    }

    pub fn clear_button_image(&self, button_idx: u8) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!("Clearing button image on device '{}' from button {}", self.serial, button_idx);
        deck.clear_button_image(button_idx).map_err(|e| format!("Failed to clear button image on device '{}' from button {}: {}", self.serial, button_idx, e))
    }

    pub fn set_button_image(&self, button_idx: u8, image: DynamicImage) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!("Setting button image on device '{}' to button {}", self.serial, button_idx);
        deck.set_button_image(button_idx, image).map_err(|e| format!("Failed to set button image on device '{}' to button {}: {}", self.serial, button_idx, e))
    }

    pub fn flush(&self) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!("Flushing device '{}'", self.serial);
        deck.flush().map_err(|e| format!("Failed to flush device '{}': {}", self.serial, e))
    }

    pub fn set_brightness(&self, brightness: u8) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!("Setting brightness {} on device '{}'", brightness, self.serial);
        deck.set_brightness(brightness).map_err(|e| format!("Failed to set brightness on device '{}': {}", self.serial, e))
    }

    pub fn clear_all_button_images(&self) -> Result<(), String> {
        let deck = self.get_deck();
        verbose_log!("Cleared all button images on device '{}'", self.serial);
        deck.clear_all_button_images().map_err(|e| format!("Failed to clear all button images on device '{}': {}", self.serial, e))
    }
}

/// Wrapper for elgato StreamDeck DeviceStateReader to implement our DeviceReader trait
struct ElgatoDeviceReader {
    reader: Arc<DeviceStateReader>,
}

// SAFETY: ElgatoDeviceReader is safe to Send/Sync because Arc<DeviceStateReader> is internally thread-safe
unsafe impl Send for ElgatoDeviceReader {}
unsafe impl Sync for ElgatoDeviceReader {}

impl DeviceReader for ElgatoDeviceReader {
    fn read(&self, timeout: Option<Duration>) -> Result<Vec<DeviceStateUpdate>, DeviceError> {
        let updates = self.reader.read(timeout)
            .map_err(|e| DeviceError::LibraryError(format!("Failed to read device state: {}", e)))?;

        // Convert elgato-streamdeck's DeviceStateUpdate to our DeviceStateUpdate
        let converted = updates.into_iter().map(|update| {
            match update {
                elgato_streamdeck::DeviceStateUpdate::ButtonDown(key) => DeviceStateUpdate::ButtonDown(key),
                elgato_streamdeck::DeviceStateUpdate::ButtonUp(key) => DeviceStateUpdate::ButtonUp(key),
                elgato_streamdeck::DeviceStateUpdate::EncoderDown(encoder) => DeviceStateUpdate::EncoderDown(encoder),
                elgato_streamdeck::DeviceStateUpdate::EncoderUp(encoder) => DeviceStateUpdate::EncoderUp(encoder),
                elgato_streamdeck::DeviceStateUpdate::EncoderTwist(encoder, ticks) => {
                    DeviceStateUpdate::EncoderTwist { encoder, ticks }
                }
                elgato_streamdeck::DeviceStateUpdate::TouchPointDown(point) => DeviceStateUpdate::TouchPointDown(point),
                elgato_streamdeck::DeviceStateUpdate::TouchPointUp(point) => DeviceStateUpdate::TouchPointUp(point),
                elgato_streamdeck::DeviceStateUpdate::TouchScreenPress(x, y) => {
                    DeviceStateUpdate::TouchScreenPress { x, y }
                }
                elgato_streamdeck::DeviceStateUpdate::TouchScreenLongPress(x, y) => {
                    DeviceStateUpdate::TouchScreenLongPress { x, y }
                }
                elgato_streamdeck::DeviceStateUpdate::TouchScreenSwipe(start, end) => {
                    // Unpack the tuples
                    DeviceStateUpdate::TouchScreenSwipe { x: start.0, y: start.1, target_x: end.0, target_y: end.1 }
                }
            }
        }).collect();

        Ok(converted)
    }
}

/// Implement KeydeckDevice trait for ElgatoDevice
impl KeydeckDevice for ElgatoDevice {
    fn serial_number(&self) -> Result<String, DeviceError> {
        Ok(self.serial.clone())
    }

    fn firmware_version(&self) -> Result<String, DeviceError> {
        let deck = self.get_deck();
        deck.firmware_version()
            .map_err(|e| DeviceError::LibraryError(format!("Failed to get firmware version: {}", e)))
    }

    fn manufacturer(&self) -> String {
        "Elgato".to_string()
    }

    fn kind_name(&self) -> String {
        format!("{:?}", self.kind)
    }

    fn button_count(&self) -> u8 {
        self.kind.key_count()
    }

    fn has_screen(&self) -> bool {
        self.kind.is_visual()
    }

    fn button_image_size(&self) -> (u16, u16) {
        let (w, h) = self.kind.key_image_format().size;
        (w as u16, h as u16)
    }

    fn button_layout(&self) -> (usize, usize) {
        let (rows, cols) = self.kind.key_layout();
        (rows as usize, cols as usize)
    }

    fn reset(&self) -> Result<(), DeviceError> {
        ElgatoDevice::reset(self).map_err(DeviceError::from)
    }

    fn set_brightness(&self, brightness: u8) -> Result<(), DeviceError> {
        ElgatoDevice::set_brightness(self, brightness).map_err(DeviceError::from)
    }

    fn set_button_image(&self, button_idx: u8, image: DynamicImage) -> Result<(), DeviceError> {
        ElgatoDevice::set_button_image(self, button_idx, image).map_err(DeviceError::from)
    }

    fn clear_button_image(&self, button_idx: u8) -> Result<(), DeviceError> {
        ElgatoDevice::clear_button_image(self, button_idx).map_err(DeviceError::from)
    }

    fn clear_all_button_images(&self) -> Result<(), DeviceError> {
        ElgatoDevice::clear_all_button_images(self).map_err(DeviceError::from)
    }

    fn flush(&self) -> Result<(), DeviceError> {
        ElgatoDevice::flush(self).map_err(DeviceError::from)
    }

    fn get_reader(&self) -> Arc<dyn DeviceReader> {
        Arc::new(ElgatoDeviceReader {
            reader: self.get_reader_arc(),
        })
    }

    // Lifecycle methods use default no-op implementations from trait
    // shutdown(), sleep(), keep_alive() - all no-op for Elgato devices
}
