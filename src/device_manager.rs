use crate::{error_log, info_log, verbose_log};
use crate::device_info::{DeviceInfo, ButtonLayout, ButtonImage};
use crate::device_trait::{DeviceError, DeviceReader, KeydeckDevice};
use crate::elgato_device::ElgatoDevice;
use crate::mirajazz_device::MirajazzDevice;
use elgato_streamdeck::{list_devices, new_hidapi};
use image::{open, DynamicImage};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// Enum wrapper for different device types
pub enum Device {
    Elgato(ElgatoDevice),
    Mirajazz(MirajazzDevice),
}

impl Device {
    pub fn serial(&self) -> &str {
        match self {
            Device::Elgato(d) => &d.serial,
            Device::Mirajazz(d) => &d.serial,
        }
    }

    pub fn device_id(&self) -> &str {
        match self {
            Device::Elgato(d) => &d.device_id,
            Device::Mirajazz(d) => &d.device_id,
        }
    }

    pub fn is_enabled(&self) -> bool {
        match self {
            Device::Elgato(d) => d.enabled,
            Device::Mirajazz(d) => d.enabled,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        match self {
            Device::Elgato(d) => d.enabled = enabled,
            Device::Mirajazz(d) => d.enabled = enabled,
        }
    }
}


// Implement KeydeckDevice for Device enum by delegating to inner types
impl KeydeckDevice for Device {
    fn serial_number(&self) -> Result<String, DeviceError> {
        match self {
            Device::Elgato(d) => d.serial_number(),
            Device::Mirajazz(d) => d.serial_number(),
        }
    }

    fn firmware_version(&self) -> Result<String, DeviceError> {
        match self {
            Device::Elgato(d) => d.firmware_version(),
            Device::Mirajazz(d) => d.firmware_version(),
        }
    }

    fn manufacturer(&self) -> String {
        match self {
            Device::Elgato(d) => d.manufacturer(),
            Device::Mirajazz(d) => d.manufacturer(),
        }
    }

    fn kind_name(&self) -> String {
        match self {
            Device::Elgato(d) => d.kind_name(),
            Device::Mirajazz(d) => d.kind_name(),
        }
    }

    fn button_count(&self) -> u8 {
        match self {
            Device::Elgato(d) => d.button_count(),
            Device::Mirajazz(d) => d.button_count(),
        }
    }

    fn has_screen(&self) -> bool {
        match self {
            Device::Elgato(d) => d.has_screen(),
            Device::Mirajazz(d) => d.has_screen(),
        }
    }

    fn button_image_size(&self) -> (u16, u16) {
        match self {
            Device::Elgato(d) => d.button_image_size(),
            Device::Mirajazz(d) => d.button_image_size(),
        }
    }

    fn button_layout(&self) -> (usize, usize) {
        match self {
            Device::Elgato(d) => d.button_layout(),
            Device::Mirajazz(d) => d.button_layout(),
        }
    }

    fn encoder_count(&self) -> usize {
        match self {
            Device::Elgato(d) => d.encoder_count(),
            Device::Mirajazz(d) => d.encoder_count(),
        }
    }

    fn reset(&self) -> Result<(), DeviceError> {
        match self {
            Device::Elgato(d) => d.reset().map_err(DeviceError::from),
            Device::Mirajazz(d) => d.reset(),
        }
    }

    fn set_brightness(&self, brightness: u8) -> Result<(), DeviceError> {
        match self {
            Device::Elgato(d) => d.set_brightness(brightness).map_err(DeviceError::from),
            Device::Mirajazz(d) => d.set_brightness(brightness),
        }
    }

    fn set_button_image(&self, button_idx: u8, image: DynamicImage) -> Result<(), DeviceError> {
        match self {
            Device::Elgato(d) => d.set_button_image(button_idx, image).map_err(DeviceError::from),
            Device::Mirajazz(d) => d.set_button_image(button_idx, image),
        }
    }

    fn clear_button_image(&self, button_idx: u8) -> Result<(), DeviceError> {
        match self {
            Device::Elgato(d) => d.clear_button_image(button_idx).map_err(DeviceError::from),
            Device::Mirajazz(d) => d.clear_button_image(button_idx),
        }
    }

    fn clear_all_button_images(&self) -> Result<(), DeviceError> {
        match self {
            Device::Elgato(d) => d.clear_all_button_images().map_err(DeviceError::from),
            Device::Mirajazz(d) => d.clear_all_button_images(),
        }
    }

    fn flush(&self) -> Result<(), DeviceError> {
        match self {
            Device::Elgato(d) => d.flush().map_err(DeviceError::from),
            Device::Mirajazz(d) => d.flush(),
        }
    }

    fn get_reader(&self) -> Arc<dyn DeviceReader> {
        match self {
            Device::Elgato(d) => d.get_reader(),
            Device::Mirajazz(d) => d.get_reader(),
        }
    }

    fn shutdown(&self) -> Result<(), DeviceError> {
        match self {
            Device::Elgato(d) => d.shutdown(),
            Device::Mirajazz(d) => d.shutdown(),
        }
    }

    fn sleep(&self) -> Result<(), DeviceError> {
        match self {
            Device::Elgato(d) => d.sleep(),
            Device::Mirajazz(d) => d.sleep(),
        }
    }

    fn keep_alive(&self) {
        match self {
            Device::Elgato(d) => d.keep_alive(),
            Device::Mirajazz(d) => d.keep_alive(),
        }
    }
}

pub struct DeviceManager {
    devices: Vec<Device>,
    image_dir: Option<String>,
    auto_added: bool,
}

impl DeviceManager {
    pub fn new() -> Self {
        let hidapi = Arc::new(new_hidapi().ok().expect("Failed to create hidapi context"));
        let mut devices: Vec<Device> = vec![];

        // Priority-based device detection:
        // 1. Check if mirajazz supports the device
        // 2. Fallback to elgato if not supported by mirajazz

        // First, enumerate all HID devices to check VID/PID
        // Use HashSet to deduplicate devices (a single physical device may have multiple HID interfaces)
        use std::collections::HashSet;
        let mut mirajazz_devices = HashSet::new();
        let mut elgato_devices = Vec::new();

        for device_info in hidapi.device_list() {
            let vid = device_info.vendor_id();
            let pid = device_info.product_id();

            // Priority 1: Check mirajazz support
            if MirajazzDevice::is_supported(vid, pid) {
                if let Some(serial) = device_info.serial_number() {
                    // Deduplicate by (vid, pid, serial) - single physical device may have multiple interfaces
                    if mirajazz_devices.insert((vid, pid, serial.to_string())) {
                        verbose_log!("Detected Mirajazz device (VID:{:04X} PID:{:04X} Serial:{})", vid, pid, serial);
                    }
                }
                continue;
            }

            // Priority 2: Check elgato support (fallback)
            if ElgatoDevice::is_supported(vid, pid) {
                elgato_devices.push((vid, pid));
            }
        }

        // Create Mirajazz device instances
        for (vid, pid, usb_serial) in mirajazz_devices {
            let device_id = format!("{:04X}:{:04X}", vid, pid);
            match MirajazzDevice::new(Arc::clone(&hidapi), vid, pid, usb_serial.clone(), device_id.clone()) {
                Ok(mirajazz_device) => {
                    let display_serial = mirajazz_device.serial.clone();
                    verbose_log!("Adding Mirajazz device: {} ({})", display_serial, device_id);
                    devices.push(Device::Mirajazz(mirajazz_device));
                }
                Err(e) => {
                    error_log!("Failed to create Mirajazz device {}: {:?}", usb_serial, e);
                }
            }
        }

        // Create ElgatoDevice instances for supported devices
        // Use the elgato library's list_devices to get Kind and serial
        for (kind, serial) in list_devices(&hidapi) {
            let vid = kind.vendor_id();
            let pid = kind.product_id();

            // Only add if it was detected as elgato-supported in our priority check
            if elgato_devices.contains(&(vid, pid)) {
                let device_id = format!("{:04X}:{:04X}", vid, pid);
                verbose_log!("Adding Elgato device: {} (VID:{:04X} PID:{:04X})", serial, vid, pid);
                devices.push(
                    Device::Elgato(ElgatoDevice::new(
                        Arc::clone(&hidapi),
                        kind,
                        serial,
                        device_id,
                    ))
                );
            }
        }

        if devices.is_empty() {
            error_log!("No supported devices found");
        }

        Self {
            devices,
            image_dir: None,
            auto_added: true,
        }
    }

    /// Enumerate all connected devices and return their serials.
    /// This is used by the device listener to detect hotplug events.
    /// Note: For Mirajazz devices with force_serial enabled, this returns the USB serial.
    /// The actual serial transformation happens in MirajazzDevice::new().
    pub fn enumerate_connected_devices() -> Vec<String> {
        let hidapi = match new_hidapi().ok() {
            Some(api) => Arc::new(api),
            None => return Vec::new(),
        };

        let mut serials = Vec::new();
        use std::collections::HashSet;
        let mut mirajazz_serials = HashSet::new();

        // Check all HID devices for Mirajazz support first
        for device_info in hidapi.device_list() {
            let vid = device_info.vendor_id();
            let pid = device_info.product_id();

            if MirajazzDevice::is_supported(vid, pid) {
                if let Some(serial) = device_info.serial_number() {
                    if mirajazz_serials.insert(serial.to_string()) {
                        serials.push(serial.to_string());
                    }
                }
            }
        }

        // Add Elgato devices
        for (_, serial) in list_devices(&hidapi) {
            serials.push(serial);
        }

        serials
    }

    pub fn grab_event(&mut self) -> Result<(), String> {
        let active = self.count_active_devices();
        if self.count_active_devices() != 1 {
            return Err(format!("Only one active device is allowed to grab events, found {}", active));
        }
        for device in self.iter_active_devices() {
            // Use trait method get_reader() which returns Arc<dyn DeviceReader>
            let reader = device.get_reader();
            if let Ok(updates) = reader.read(Some(Duration::from_secs_f64(100.0))) {
                for update in updates {
                    info_log!("{:?}", update);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn reset_devices(&mut self) -> Result<(), String> {
        for device in self.iter_active_devices() {
            device.reset()?;
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
        let image_data = match find_path(&img_path, self.image_dir.clone()) {
            Some(image_path) => open(image_path).map_err(|e| format!("Failed to open image '{}': {}", &img_path, e))?,
            None => return Err(format!("Image '{}' not found", img_path)),
        };
        for device in self.iter_active_devices() {
            device.set_button_image(button_idx, image_data.clone())?;
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
            info_log!("{} {} {}", device.device_id(), device.serial(), device.kind_name());
        }
        info_log!("Total devices: {}", self.count_active_devices());
    }

    pub fn info_device(&mut self, identifier: String) -> Result<(), String> {
        for device in &mut self.devices {
            if device.device_id() == identifier || device.serial().trim() == identifier {
                let button_count = device.button_count();
                let (img_width, img_height) = device.button_image_size();
                let (rows, cols) = device.button_layout();
                let encoders = device.encoder_count();

                let device_info = DeviceInfo {
                    device_id: device.device_id().to_string(),
                    serial: device.serial_number().unwrap_or_else(|_| "Unknown".to_string()),
                    model: device.kind_name(),
                    button_layout: ButtonLayout {
                        rows: rows as u8,
                        columns: cols as u8,
                        total: button_count,
                    },
                    button_image: ButtonImage {
                        width: img_width as usize,
                        height: img_height as usize,
                        format: "JPEG".to_string(), // Assuming JPEG for now
                    },
                    encoders: encoders as u8,
                    touchpoints: 0, // Not available in trait
                    lcd_strip: None, // Not available in trait
                    is_visual: device.has_screen(),
                };

                match serde_yaml_ng::to_string(&device_info) {
                    Ok(yaml) => {
                        info_log!("{}", yaml);
                        return Ok(());
                    }
                    Err(e) => return Err(format!("Failed to serialize device info: {}", e)),
                }
            }
        }
        Err(format!("Device with id '{}' not found", identifier))
    }

    pub fn enable_device(&mut self, identifier: String) -> Result<(), String> {
        if self.auto_added {
            self.set_state_all_devices(false);
            self.auto_added = false;
        }
        for device in &mut self.devices {
            if device.device_id() == identifier || device.serial().trim() == identifier {
                device.set_enabled(true);
                return Ok(());
            }
        }
        Err(format!("Enabling device with id '{}' not found", identifier))
    }

    pub fn disable_device(&mut self, device_id: String) -> Result<(), String> {
        self.auto_added = false;
        for device in &mut self.devices {
            if device.device_id() == device_id || device.serial().trim() == device_id {
                device.set_enabled(false);
                return Ok(());
            }
        }
        Err(format!("Disabling device with id '{}' not found", device_id))
    }

    fn set_state_all_devices(&mut self, state: bool) {
        for device in &mut self.devices {
            device.set_enabled(state);
        }
    }

    fn count_active_devices(&self) -> usize {
        let mut count = 0;
        for device in self.devices.iter() {
            if device.is_enabled() {
                count += 1;
            }
        }
        count
    }

    pub fn iter_active_devices(&mut self) -> impl Iterator<Item=&mut Device> {
        self.devices.iter_mut().filter(|device| device.is_enabled())
    }
}

pub fn find_path(file: &str, dir: Option<String>) -> Option<String> {
    if Path::new(file).exists() {
        Some(file.to_string())
    } else {
        let other_path = format!("{}/{}", dir.clone().unwrap_or_else(|| ".".to_string()), file.replace("\\", "/"));
        if Path::new(&other_path).exists() {
            Some(other_path)
        } else {
            None
        }
    }
}

pub fn find_device_by_serial(device_sn: &str) -> Option<Device> {
    let hidapi = match new_hidapi().ok() {
        Some(api) => Arc::new(api),
        None => {
            error_log!("Failed to create hidapi context when searching for device");
            return None;
        }
    };

    // Priority-based device detection: mirajazz first, then elgato fallback
    // First check mirajazz devices
    for device_info in hidapi.device_list() {
        if let Some(serial) = device_info.serial_number() {
            if serial == device_sn {
                let vid = device_info.vendor_id();
                let pid = device_info.product_id();

                if MirajazzDevice::is_supported(vid, pid) {
                    let device_id = format!("{:04X}:{:04X}", vid, pid);
                    match MirajazzDevice::new(Arc::clone(&hidapi), vid, pid, serial.to_string(), device_id) {
                        Ok(mirajazz_device) => {
                            return Some(Device::Mirajazz(mirajazz_device));
                        }
                        Err(e) => {
                            error_log!("Failed to create Mirajazz device {}: {:?}", device_sn, e);
                            return None;
                        }
                    }
                }
            }
        }
    }

    // Fallback: check elgato devices
    for (kind, serial) in list_devices(&hidapi) {
        if serial == device_sn {
            let vid = kind.vendor_id();
            let pid = kind.product_id();

            if ElgatoDevice::is_supported(vid, pid) {
                let device_id = format!("{:04X}:{:04X}", vid, pid);
                return Some(Device::Elgato(ElgatoDevice::new(
                    Arc::clone(&hidapi),
                    kind,
                    serial,
                    device_id,
                )));
            }
        }
    }
    None
}