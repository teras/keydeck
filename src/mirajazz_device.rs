use crate::device_trait::{DeviceError, DeviceReader, DeviceStateUpdate, KeydeckDevice};
use crate::{error_log, verbose_log};
use hidapi::HidApi;
use image::DynamicImage;
use mirajazz_json::{
    device::Device,
    registry::{DeviceDefinition, DeviceRegistry, ImageMode, Rotation, Mirror},
    state::DeviceStateReader,
};
use std::cell::RefCell;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// Global registry instance (loaded once at startup)
static REGISTRY: OnceLock<Option<DeviceRegistry>> = OnceLock::new();

/// Initialize the device registry with the given search paths.
/// Paths are searched in order, with later paths overriding earlier ones.
/// Should be called once at startup before creating any devices.
pub fn init_registry(paths: &[String]) -> Result<(), String> {
    verbose_log!("Initializing Mirajazz device registry with {} search paths:", paths.len());
    for (i, path) in paths.iter().enumerate() {
        verbose_log!("  [{}] {}", i, path);
    }

    REGISTRY.get_or_init(|| {
        DeviceRegistry::load_from_paths(paths).ok()
    });

    if REGISTRY.get().and_then(|r| r.as_ref()).is_none() {
        return Err("Failed to load device registry from any provided path".to_string());
    }

    verbose_log!("Mirajazz device registry loaded successfully");
    Ok(())
}

/// Get the device registry (must be initialized first via init_registry)
/// Returns None if registry fails to load (e.g., JSON files not found)
pub fn get_registry() -> Option<&'static DeviceRegistry> {
    REGISTRY.get().and_then(|r| r.as_ref())
}

/// Wrapper for Mirajazz devices to implement KeydeckDevice trait
pub struct MirajazzDevice {
    hid_api: Arc<HidApi>,
    vid: u16,
    pid: u16,
    pub serial: String,
    pub(crate) device_id: String,
    device_def: &'static DeviceDefinition,
    device: RefCell<Option<Arc<Device>>>,
    reader: RefCell<Option<Arc<DeviceStateReader>>>,
    pub(crate) enabled: bool,
}

// SAFETY: MirajazzDevice is safe to Send/Sync because:
// - RefCell is only used for lazy initialization (get_device, get_reader_arc)
// - The actual Device/DeviceStateReader are Arc-wrapped and can be safely shared
// - Access patterns ensure no concurrent mutation (initialized once per access)
unsafe impl Send for MirajazzDevice {}
unsafe impl Sync for MirajazzDevice {}

impl MirajazzDevice {
    /// Create a new MirajazzDevice wrapper
    pub fn new(
        hid_api: Arc<HidApi>,
        vid: u16,
        pid: u16,
        usb_serial: String,
        device_id: String,
    ) -> Result<Self, DeviceError> {
        let registry = get_registry()
            .ok_or_else(|| DeviceError::LibraryError(
                "Mirajazz device registry not loaded".to_string()
            ))?;

        let device_def = registry
            .find_by_vid_pid(vid, pid)
            .ok_or_else(|| DeviceError::UnsupportedOperation(
                format!("Device {:04X}:{:04X} not found in registry", vid, pid)
            ))?;

        // Generate unique serial if force_serial is enabled
        let serial = if device_def.quirks.force_serial {
            let generated = format!("{}-{:04X}{:04X}", usb_serial, vid, pid);
            verbose_log!("Generated serial for device (USB serial was '{}'): {}", usb_serial, generated);
            generated
        } else {
            usb_serial
        };

        Ok(MirajazzDevice {
            hid_api,
            vid,
            pid,
            serial,
            device_id,
            device_def,
            device: RefCell::new(None),
            reader: RefCell::new(None),
            enabled: true,
        })
    }

    /// Check if a device with given VID/PID is supported by mirajazz library
    pub fn is_supported(vid: u16, pid: u16) -> bool {
        get_registry()
            .map(|registry| registry.is_supported(vid, pid))
            .unwrap_or(false)
    }

    fn get_device(&self) -> Arc<Device> {
        self.device.borrow_mut().get_or_insert_with(|| {
            Arc::new(
                Device::connect(
                    &self.hid_api,
                    self.vid,
                    self.pid,
                    &self.serial,
                    self.device_def.protocol.protocol_version >= 2, // is_v2
                    self.device_def.protocol.protocol_version >= 3, // supports_both_states
                    self.device_def.layout.key_count(),
                    self.device_def.layout.encoder_count,
                )
                .unwrap_or_else(|e| {
                    error_log!("Failed to connect to Mirajazz device '{}': {}", self.serial, e);
                    error_log!("This may be due to:");
                    error_log!("  - Device was unplugged");
                    error_log!("  - Insufficient USB permissions");
                    error_log!("  - Device busy/in use by another process");
                    panic!("Cannot continue without device connection");
                }),
            )
        }).clone()
    }

    fn get_reader_arc(&self) -> Arc<DeviceStateReader> {
        if self.reader.borrow().is_none() {
            let device = self.get_device();
            *self.reader.borrow_mut() = Some(device.get_reader());
        }

        self.reader.borrow().as_ref().expect("Reader should be initialized").clone()
    }

    /// Convert mirajazz ImageFormat from registry definition
    fn get_image_format_for_button(&self, button_idx: u8) -> mirajazz_json::types::ImageFormat {
        let button_format = self.device_def.image_format_for_button(button_idx);

        mirajazz_json::types::ImageFormat {
            mode: match button_format.mode {
                ImageMode::BMP => mirajazz_json::types::ImageMode::BMP,
                ImageMode::JPEG => mirajazz_json::types::ImageMode::JPEG,
            },
            size: (button_format.size[0] as usize, button_format.size[1] as usize),
            rotation: match button_format.rotation {
                Rotation::Rot0 => mirajazz_json::types::ImageRotation::Rot0,
                Rotation::Rot90 => mirajazz_json::types::ImageRotation::Rot90,
                Rotation::Rot180 => mirajazz_json::types::ImageRotation::Rot180,
                Rotation::Rot270 => mirajazz_json::types::ImageRotation::Rot270,
            },
            mirror: match button_format.mirror {
                Mirror::None => mirajazz_json::types::ImageMirroring::None,
                Mirror::X => mirajazz_json::types::ImageMirroring::X,
                Mirror::Y => mirajazz_json::types::ImageMirroring::Y,
                Mirror::Both => mirajazz_json::types::ImageMirroring::Both,
            },
        }
    }

    /// Map keydeck button index to device button index (handles remapping quirk)
    fn map_button_index(&self, button_idx: u8) -> u8 {
        self.device_def.opendeck_to_device_button(button_idx)
    }
}

impl KeydeckDevice for MirajazzDevice {
    fn serial_number(&self) -> Result<String, DeviceError> {
        let device = self.get_device();
        device.serial_number()
            .map_err(|e| DeviceError::LibraryError(format!("Failed to get serial number: {}", e)))
    }

    fn firmware_version(&self) -> Result<String, DeviceError> {
        let device = self.get_device();
        device.firmware_version()
            .map_err(|e| DeviceError::LibraryError(format!("Failed to get firmware version: {}", e)))
    }

    fn manufacturer(&self) -> String {
        self.device_def.info.manufacturer.clone()
            .unwrap_or_else(|| "Unknown".to_string())
    }

    fn kind_name(&self) -> String {
        self.device_def.info.human_name.clone()
    }

    fn button_count(&self) -> u8 {
        self.device_def.layout.key_count() as u8
    }

    fn has_screen(&self) -> bool {
        // Mirajazz devices have button screens
        true
    }

    fn button_image_size(&self) -> (u16, u16) {
        let size = self.device_def.image_format.default_size;
        (size[0], size[1])
    }

    fn button_layout(&self) -> (usize, usize) {
        (self.device_def.layout.rows, self.device_def.layout.cols)
    }

    fn encoder_count(&self) -> usize {
        self.device_def.layout.encoder_count
    }

    fn reset(&self) -> Result<(), DeviceError> {
        let device = self.get_device();
        verbose_log!("Resetting device '{}' (set brightness 100% and clear all images)", self.serial);
        device.reset()
            .map_err(|e| DeviceError::LibraryError(format!("Failed to reset: {}", e)))
    }

    fn set_brightness(&self, brightness: u8) -> Result<(), DeviceError> {
        let device = self.get_device();
        verbose_log!("Setting brightness {} on device '{}'", brightness, self.serial);
        device.set_brightness(brightness)
            .map_err(|e| DeviceError::LibraryError(format!("Failed to set brightness: {}", e)))
    }

    fn set_button_image(&self, button_idx: u8, image: DynamicImage) -> Result<(), DeviceError> {
        let device = self.get_device();
        let mapped_idx = self.map_button_index(button_idx);
        let format = self.get_image_format_for_button(mapped_idx);

        verbose_log!("Setting button image on device '{}' to button {}", self.serial, button_idx);
        device.set_button_image(mapped_idx, format, image)
            .map_err(|e| DeviceError::LibraryError(format!("Failed to set button image: {}", e)))
    }

    fn clear_button_image(&self, button_idx: u8) -> Result<(), DeviceError> {
        let device = self.get_device();
        let mapped_idx = self.map_button_index(button_idx);

        verbose_log!("Clearing button image on device '{}' from button {}", self.serial, button_idx);
        device.clear_button_image(mapped_idx)
            .map_err(|e| DeviceError::LibraryError(format!("Failed to clear button image: {}", e)))
    }

    fn clear_all_button_images(&self) -> Result<(), DeviceError> {
        let device = self.get_device();
        verbose_log!("Cleared all button images on device '{}'", self.serial);
        device.clear_all_button_images()
            .map_err(|e| DeviceError::LibraryError(format!("Failed to clear all button images: {}", e)))
    }

    fn flush(&self) -> Result<(), DeviceError> {
        let device = self.get_device();
        verbose_log!("Flushing device '{}'", self.serial);
        device.flush()
            .map_err(|e| DeviceError::LibraryError(format!("Failed to flush: {}", e)))
    }

    fn sleep(&self) -> Result<(), DeviceError> {
        let device = self.get_device();
        verbose_log!("Putting device '{}' to sleep", self.serial);
        device.sleep()
            .map_err(|e| DeviceError::LibraryError(format!("Failed to sleep device: {}", e)))
    }

    fn keep_alive(&self) {
        let device = self.get_device();
        verbose_log!("Sending keep-alive to device '{}'", self.serial);
        let _ = device.keep_alive(); // Ignore errors for keep_alive
    }

    fn get_reader(&self) -> Arc<dyn DeviceReader> {
        Arc::new(MirajazzDeviceReader {
            reader: self.get_reader_arc(),
            device_def: self.device_def,
        })
    }

    fn shutdown(&self) -> Result<(), DeviceError> {
        let device = self.get_device();
        verbose_log!("Shutting down device '{}'", self.serial);
        device.shutdown()
            .map_err(|e| DeviceError::LibraryError(format!("Failed to shutdown: {}", e)))
    }
}

/// Wrapper for mirajazz DeviceStateReader to implement our DeviceReader trait
struct MirajazzDeviceReader {
    reader: Arc<DeviceStateReader>,
    device_def: &'static DeviceDefinition,
}

// SAFETY: MirajazzDeviceReader is safe to Send/Sync because Arc<DeviceStateReader> is internally thread-safe
unsafe impl Send for MirajazzDeviceReader {}
unsafe impl Sync for MirajazzDeviceReader {}

impl DeviceReader for MirajazzDeviceReader {
    fn read(&self, timeout: Option<Duration>) -> Result<Vec<DeviceStateUpdate>, DeviceError> {
        use mirajazz_json::types::DeviceInput;

        // Create the process_input closure that converts HID data to DeviceInput
        // key = button index from USB report (device native index), state = button state (0 or 1)
        let button_count = self.device_def.layout.key_count();
        let updates = self.reader.read(timeout, move |key, state| {
            // Device reports button indices starting from 1, so subtract 1 to get 0-indexed
            // This quirk is documented in legacy opendeck-akp153 plugin
            let key_0indexed = if key > 0 { key - 1 } else { 0 };

            // Build button state vector for all buttons using DEVICE NATIVE indices
            // The vector index represents the device button position
            // Mapping to opendeck logical indices happens later when processing events
            let mut buttons = vec![false; button_count];
            if key_0indexed < button_count as u8 {
                buttons[key_0indexed as usize] = state != 0;
            }
            Ok(DeviceInput::ButtonStateChange(buttons))
        }).map_err(|e| DeviceError::LibraryError(format!("Failed to read device state: {}", e)))?;

        if !updates.is_empty() {
            verbose_log!("Received {} updates from mirajazz device", updates.len());
        }

        // Convert mirajazz DeviceStateUpdate to keydeck DeviceStateUpdate
        // Map button indices from device native order to opendeck logical order
        let keydeck_updates: Vec<DeviceStateUpdate> = updates.iter().filter_map(|update| {
            use mirajazz_json::state::DeviceStateUpdate as MirajazzUpdate;
            match update {
                MirajazzUpdate::ButtonDown(device_idx) => {
                    let opendeck_idx = self.device_def.device_to_opendeck_button(*device_idx);
                    verbose_log!("Button down: device idx {} -> opendeck idx {}", device_idx, opendeck_idx);
                    Some(DeviceStateUpdate::ButtonDown(opendeck_idx))
                }
                MirajazzUpdate::ButtonUp(device_idx) => {
                    let opendeck_idx = self.device_def.device_to_opendeck_button(*device_idx);
                    verbose_log!("Button up: device idx {} -> opendeck idx {}", device_idx, opendeck_idx);
                    Some(DeviceStateUpdate::ButtonUp(opendeck_idx))
                }
                MirajazzUpdate::EncoderDown(idx) => Some(DeviceStateUpdate::EncoderDown(*idx)),
                MirajazzUpdate::EncoderUp(idx) => Some(DeviceStateUpdate::EncoderUp(*idx)),
                MirajazzUpdate::EncoderTwist(encoder, ticks) => Some(DeviceStateUpdate::EncoderTwist {
                    encoder: *encoder,
                    ticks: *ticks,
                }),
            }
        }).collect();

        Ok(keydeck_updates)
    }
}
