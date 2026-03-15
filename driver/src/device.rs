use std::{
    collections::HashSet,
    str::{from_utf8, Utf8Error},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock,
    },
    time::Duration,
};

use hidapi::{HidApi, HidDevice, HidResult};
use image::DynamicImage;

use crate::{
    error::MirajazzError,
    images::convert_image_with_format,
    state::{DeviceState, DeviceStateReader},
    types::{DeviceInput, ImageFormat},
};

/// Device query for filtering HID devices by USB identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceQuery {
    pub usage_page: u16,
    pub usage_id: u16,
    pub vendor_id: u16,
    pub product_id: u16,
}

impl DeviceQuery {
    pub const fn new(usage_page: u16, usage_id: u16, vendor_id: u16, product_id: u16) -> Self {
        Self {
            usage_page,
            usage_id,
            vendor_id,
            product_id,
        }
    }
}

/// Creates an instance of the HidApi
///
/// Can be used if you don't want to link hidapi crate into your project
pub fn new_hidapi() -> HidResult<HidApi> {
    HidApi::new()
}

/// Actually refreshes the device list
pub fn refresh_device_list(hidapi: &mut HidApi) -> HidResult<()> {
    hidapi.refresh_devices()
}

/// Returns a list of devices as (Kind, Serial Number) that could be found using HidApi.
///
/// **WARNING:** To refresh the list, use [refresh_device_list]
pub fn list_devices(hidapi: &HidApi, vids: &[u16]) -> Vec<(u16, u16, String)> {
    hidapi
        .device_list()
        .filter_map(|d| {
            if !vids.contains(&d.vendor_id()) {
                return None;
            }

            if let Some(serial) = d.serial_number() {
                Some((d.vendor_id(), d.product_id(), serial.to_string()))
            } else {
                None
            }
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

/// Extracts string from byte array, removing \0 symbols
pub fn extract_str(bytes: &[u8]) -> Result<String, Utf8Error> {
    Ok(from_utf8(bytes)?.replace('\0', "").to_string())
}

struct ImageCache {
    key: u8,
    image_data: Vec<u8>,
}

/// Interface for a device
pub struct Device {
    /// Vendor ID of the device
    pub vid: u16,
    /// Product ID of the device
    pub pid: u16,
    /// Protocol version: 0 (legacy), 1 (old), 2 (v2), 3 (encoder states)
    protocol_version: usize,
    /// Emits separate press/release events for buttons
    supports_both_states: bool,
    /// Emits separate press/release events for encoders (can differ from buttons)
    supports_both_encoder_states: bool,
    /// Number of keys
    key_count: usize,
    /// Number of encoders
    encoder_count: usize,
    /// Packet size
    packet_size: usize,
    /// HID report ID (0x00 for most devices, 0x04 for K1Pro)
    report_id: u8,
    /// Connected HIDDevice
    hid_device: HidDevice,
    /// Temporarily cache the image before sending it to the device
    image_cache: RwLock<Vec<ImageCache>>,
    /// Device needs to be initialized
    initialized: AtomicBool,
}

/// Static functions of the struct
impl Device {
    /// Attempts to connect to the device
    pub fn connect(
        hidapi: &HidApi,
        vid: u16,
        pid: u16,
        serial: &str,
        protocol_version: usize,
        supports_both_states: bool,
        key_count: usize,
        encoder_count: usize,
    ) -> Result<Device, MirajazzError> {
        Self::connect_with_report_id(hidapi, vid, pid, serial, protocol_version, supports_both_states, key_count, encoder_count, 0x00)
    }

    /// Attempts to connect to the device with a custom HID report ID
    pub fn connect_with_report_id(
        hidapi: &HidApi,
        vid: u16,
        pid: u16,
        serial: &str,
        protocol_version: usize,
        supports_both_states: bool,
        key_count: usize,
        encoder_count: usize,
        report_id: u8,
    ) -> Result<Device, MirajazzError> {
        let hid_device = hidapi.open_serial(vid, pid, serial)?;

        Ok(Device {
            vid,
            pid,
            protocol_version,
            supports_both_states,
            supports_both_encoder_states: supports_both_states,
            key_count,
            encoder_count,
            packet_size: if protocol_version >= 2 { 1024 } else { 512 },
            report_id,
            hid_device,
            image_cache: RwLock::new(vec![]),
            initialized: false.into(),
        })
    }
}

/// Instance methods of the struct
impl Device {
    /// Returns key count
    pub fn key_count(&self) -> usize {
        self.key_count
    }

    /// Returns encoder count
    pub fn encoder_count(&self) -> usize {
        self.encoder_count
    }

    /// Returns manufacturer string of the device
    pub fn manufacturer(&self) -> Result<String, MirajazzError> {
        Ok(self
            .hid_device
            .get_manufacturer_string()?
            .unwrap_or_else(|| "Unknown".to_string()))
    }

    /// Returns product string of the device
    pub fn product(&self) -> Result<String, MirajazzError> {
        Ok(self
            .hid_device
            .get_product_string()?
            .unwrap_or_else(|| "Unknown".to_string()))
    }

    /// Returns serial number of the device
    pub fn serial_number(&self) -> Result<String, MirajazzError> {
        let serial = self.hid_device.get_serial_number_string()?;
        match serial {
            Some(serial) => {
                if serial.is_empty() {
                    Ok("Unknown".to_string())
                } else {
                    Ok(serial)
                }
            }
            None => Ok("Unknown".to_string()),
        }
        .map(|s| s.replace('\u{0001}', ""))
    }

    /// Returns firmware version of the device
    pub fn firmware_version(&self) -> Result<String, MirajazzError> {
        let bytes = self.get_feature_report(0x01, 20)?;

        Ok(extract_str(&bytes[0..])?)
    }

    /// Initializes the device
    fn initialize(&self) -> Result<(), MirajazzError> {
        if self.initialized.load(Ordering::Acquire) {
            return Ok(());
        }

        self.initialized.store(true, Ordering::Release);

        let mut buf = vec![self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00, 0x44, 0x49, 0x53];
        self.write_extended_data(&mut buf)?;

        let mut buf = vec![
            self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00, 0x4c, 0x49, 0x47, 0x00, 0x00, 0x00, 0x00,
        ];
        self.write_extended_data(&mut buf)?;

        Ok(())
    }

    /// Returns value of `supports_both_states` (for buttons)
    pub fn supports_both_states(&self) -> bool {
        self.supports_both_states
    }

    /// Returns value of `supports_both_encoder_states`
    pub fn supports_both_encoder_states(&self) -> bool {
        self.supports_both_encoder_states
    }

    /// Override the auto-detected encoder state capability
    pub fn with_supports_both_encoder_states(mut self, supports: bool) -> Self {
        self.supports_both_encoder_states = supports;
        self
    }

    /// Sends MOD command to switch device mode (for multimodal devices)
    pub fn set_mode(&self, mode: u8) -> Result<(), MirajazzError> {
        let mut buf = vec![
            self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00, 0x4D, 0x4F, 0x44, 0x30 + mode,
        ];
        self.write_extended_data(&mut buf)?;
        Ok(())
    }

    /// Reads current input state from the device and calls provided function for processing
    pub fn read_input(
        &self,
        timeout: Option<Duration>,
        process_input: impl Fn(u8, u8) -> Result<DeviceInput, MirajazzError>,
    ) -> Result<DeviceInput, MirajazzError> {
        self.initialize()?;

        let data = self.read_data(512, timeout)?;

        // For devices with non-zero report ID (e.g., K1Pro uses 0x04),
        // the response data is shifted by 1 byte
        let offset: usize = if self.report_id != 0x00 { 1 } else { 0 };

        if data[offset] == 0 {
            return Ok(DeviceInput::NoData);
        }

        // Validate ACK prefix [65, 67, 75] = "ACK" (skip for protocol_version 0)
        if self.protocol_version > 0 && !data[offset..].starts_with(&[65, 67, 75]) {
            return Ok(DeviceInput::NoData);
        }

        let state = if self.supports_both_states() {
            data[10 + offset]
        } else {
            0x1u8
        };

        Ok(process_input(data[9 + offset], state)?)
    }

    /// Resets the device
    pub fn reset(&self) -> Result<(), MirajazzError> {
        self.initialize()?;

        self.set_brightness(100)?;
        self.clear_all_button_images()?;

        Ok(())
    }

    /// Sets brightness of the device, value range is 0 - 100
    pub fn set_brightness(&self, percent: u8) -> Result<(), MirajazzError> {
        self.initialize()?;

        let percent = percent.clamp(0, 100);

        let mut buf = vec![
            self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00, 0x4c, 0x49, 0x47, 0x00, 0x00, percent,
        ];

        self.write_extended_data(&mut buf)?;

        Ok(())
    }

    fn send_image(&self, key: u8, image_data: &[u8]) -> Result<(), MirajazzError> {
        let mut buf = vec![
            self.report_id,
            0x43,
            0x52,
            0x54,
            0x00,
            0x00,
            0x42,
            0x41,
            0x54,
            0x00,
            0x00,
            (image_data.len() >> 8) as u8,
            image_data.len() as u8,
            key + 1,
        ];

        self.write_extended_data(&mut buf)?;

        self.write_image_data_reports(image_data)?;

        Ok(())
    }

    /// Sends persistent boot logo to device flash using CRT LOG command.
    /// The image survives power cycles. The image data should already be encoded (JPEG/PNG).
    /// Sends CRT_STP (refresh) after the image data, as required by the device protocol.
    pub fn send_boot_logo(&self, image_data: &[u8]) -> Result<(), MirajazzError> {
        self.initialize()?;

        let len = image_data.len();
        let mut buf = vec![
            self.report_id,
            0x43, 0x52, 0x54,          // "CRT"
            0x00, 0x00,                 // padding
            0x4c, 0x4f, 0x47,          // "LOG"
            0x00,                       // padding
            (len >> 16) as u8,          // size high
            (len >> 8) as u8,           // size mid
            len as u8,                  // size low
            0x01,                       // target (background)
        ];

        self.write_extended_data(&mut buf)?;
        self.write_image_data_reports(image_data)?;

        // Flush (CRT_STP) — required before sending key images
        let mut buf = vec![self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00, 0x53, 0x54, 0x50];
        self.write_extended_data(&mut buf)?;

        Ok(())
    }

    /// Send runtime background frame using CRT BGPIC command.
    /// Unlike CRT LOG (persistent boot logo), BGPIC is a runtime overlay on a framebuffer layer.
    /// The image is placed at position (x, y) with given dimensions on the specified layer.
    pub fn send_background_image(
        &self,
        image_data: &[u8],
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        fb_layer: u8,
    ) -> Result<(), MirajazzError> {
        self.initialize()?;

        let len = image_data.len() as u32;
        let mut buf = vec![
            self.report_id,
            0x43, 0x52, 0x54,                     // "CRT"
            0x00, 0x00,                             // padding
            0x42, 0x47, 0x50, 0x49, 0x43,          // "BGPIC"
            (len >> 24) as u8,                      // data length (uint32 BE)
            (len >> 16) as u8,
            (len >> 8) as u8,
            len as u8,
            (x >> 8) as u8, x as u8,               // X position (uint16 BE)
            (y >> 8) as u8, y as u8,               // Y position (uint16 BE)
            (width >> 8) as u8, width as u8,       // Width (uint16 BE)
            (height >> 8) as u8, height as u8,     // Height (uint16 BE)
            0x00,                                   // reserved
            fb_layer,                               // framebuffer layer
        ];

        self.write_extended_data(&mut buf)?;
        self.write_image_data_reports(image_data)?;

        // Flush (CRT_STP)
        let mut buf = vec![self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00, 0x53, 0x54, 0x50];
        self.write_extended_data(&mut buf)?;

        Ok(())
    }

    /// Clear runtime background frame layer using CRT BGCLE command.
    /// position: 0x01 = keys only, 0x02 = touchscreen only, 0x03 = all
    pub fn clear_background_image(&self, position: u8) -> Result<(), MirajazzError> {
        self.initialize()?;

        let mut buf = vec![
            self.report_id,
            0x43, 0x52, 0x54,                     // "CRT"
            0x00, 0x00,                             // padding
            0x42, 0x47, 0x43, 0x4c, 0x45,          // "BGCLE"
            position,                               // target (0x01/0x02/0x03)
        ];

        self.write_extended_data(&mut buf)?;

        Ok(())
    }

    /// Writes image data to device, changes must be flushed with `.flush()` before
    /// they will appear on the device!
    pub fn write_image(&self, key: u8, image_data: &[u8]) -> Result<(), MirajazzError> {
        let cache_entry = ImageCache {
            key,
            image_data: image_data.to_vec(), // Convert &[u8] to Vec<u8>
        };

        self.image_cache.write()?.push(cache_entry);

        Ok(())
    }

    /// Sets button's image to blank, changes must be flushed with `.flush()` before
    /// they will appear on the device!
    pub fn clear_button_image(&self, key: u8) -> Result<(), MirajazzError> {
        self.initialize()?;

        let mut buf = vec![
            self.report_id,
            0x43,
            0x52,
            0x54,
            0x00,
            0x00,
            0x43,
            0x4c,
            0x45,
            0x00,
            0x00,
            0x00,
            if key == 0xff { 0xff } else { key + 1 },
        ];

        self.write_extended_data(&mut buf)?;

        Ok(())
    }

    /// Sets blank images to every button, changes must be flushed with `.flush()` before
    /// they will appear on the device!
    pub fn clear_all_button_images(&self) -> Result<(), MirajazzError> {
        self.initialize()?;

        self.clear_button_image(0xFF)?;

        if self.protocol_version >= 2 {
            // Mirabox v2+ requires STP to commit clearing the screen
            let mut buf = vec![self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00, 0x53, 0x54, 0x50];

            self.write_extended_data(&mut buf)?;
        }

        Ok(())
    }

    /// Sets specified button's image, changes must be flushed with `.flush()` before
    /// they will appear on the device!
    pub fn set_button_image(
        &self,
        key: u8,
        image_format: ImageFormat,
        image: DynamicImage,
    ) -> Result<(), MirajazzError> {
        self.initialize()?;

        let image_data = convert_image_with_format(image_format, image)?;

        self.write_image(key, &image_data)?;

        Ok(())
    }

    /// Sleeps the device
    pub fn sleep(&self) -> Result<(), MirajazzError> {
        self.initialize()?;

        let mut buf = vec![self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00, 0x48, 0x41, 0x4e];
        self.write_extended_data(&mut buf)?;

        Ok(())
    }

    /// Make periodic events to the device, to keep it alive
    pub fn keep_alive(&self) -> Result<(), MirajazzError> {
        self.initialize()?;

        let mut buf = vec![
            self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00, 0x43, 0x4F, 0x4E, 0x4E, 0x45, 0x43, 0x54,
        ];

        self.write_extended_data(&mut buf)?;

        Ok(())
    }

    /// Shutdown the device
    pub fn shutdown(&self) -> Result<(), MirajazzError> {
        self.initialize()?;

        let mut buf = vec![
            self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00, 0x43, 0x4c, 0x45, 0x00, 0x00, 0x44, 0x43,
        ];
        self.write_extended_data(&mut buf)?;

        let mut buf = vec![self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00, 0x48, 0x41, 0x4E];
        self.write_extended_data(&mut buf)?;

        Ok(())
    }

    /// Wakes the device screen (sends CRT DIS, same as display init)
    pub fn wakeup(&self) -> Result<(), MirajazzError> {
        let mut buf = vec![self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00, 0x44, 0x49, 0x53];
        self.write_extended_data(&mut buf)?;
        self.initialized.store(true, Ordering::Release);
        Ok(())
    }

    /// Sets RGB LED strip brightness (0-100) using CRT LBLIG command.
    /// Supported on devices with RGB LED strips (e.g., N4Pro, XL).
    pub fn set_led_brightness(&self, brightness: u8) -> Result<(), MirajazzError> {
        self.initialize()?;
        let brightness = brightness.clamp(0, 100);
        let mut buf = vec![
            self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00,
            0x4c, 0x42, 0x4c, 0x49, 0x47, // "LBLIG"
            brightness,
        ];
        self.write_extended_data(&mut buf)?;
        Ok(())
    }

    /// Sets RGB LED strip colors using CRT SETLB command.
    /// Each tuple is (r, g, b) for one LED. Multiple LEDs are set in sequence.
    pub fn set_led_color(&self, colors: &[(u8, u8, u8)]) -> Result<(), MirajazzError> {
        self.initialize()?;
        let mut buf = vec![
            self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00,
            0x53, 0x45, 0x54, 0x4c, 0x42, // "SETLB"
        ];
        for &(r, g, b) in colors {
            buf.push(r);
            buf.push(g);
            buf.push(b);
        }
        self.write_extended_data(&mut buf)?;
        Ok(())
    }

    /// Resets RGB LED strip to default state using CRT DELED command.
    pub fn reset_led_color(&self) -> Result<(), MirajazzError> {
        self.initialize()?;
        let mut buf = vec![
            self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00,
            0x44, 0x45, 0x4c, 0x45, 0x44, // "DELED"
        ];
        self.write_extended_data(&mut buf)?;
        Ok(())
    }

    /// Sets device configuration flags using CRT QUCMD command.
    /// Config bytes: each byte is 0x11 (on), 0xFF (off), or 0x1F (follow/default).
    /// N4Pro/XL layout: [LedFollowKeyLight, KeyLightOnDisconnect, CheckUsbPower,
    ///                    EnableVibration, ResetUsbReport, EnableBootVideo]
    pub fn set_device_config(&self, config: &[u8]) -> Result<(), MirajazzError> {
        self.initialize()?;
        let mut buf = vec![
            self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00,
            0x51, 0x55, 0x43, 0x4d, 0x44, // "QUCMD"
        ];
        buf.extend_from_slice(config);
        self.write_extended_data(&mut buf)?;
        Ok(())
    }

    /// Sets keyboard backlight brightness (0-6) using CRT LLUM command.
    /// K1Pro devices only.
    pub fn set_keyboard_backlight_brightness(&self, brightness: u8) -> Result<(), MirajazzError> {
        self.initialize()?;
        let mut buf = vec![
            self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00,
            0x4c, 0x4c, 0x55, 0x4d, // "LLUM"
            0x00, brightness,
        ];
        self.write_extended_data(&mut buf)?;
        Ok(())
    }

    /// Sets keyboard lighting mode/effect/speed using CRT LMOD command.
    /// Effects: 0-9, Speed: 0-7. K1Pro devices only.
    pub fn set_keyboard_lighting_mode(&self, value: u8) -> Result<(), MirajazzError> {
        self.initialize()?;
        let mut buf = vec![
            self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00,
            0x4c, 0x4d, 0x4f, 0x44, // "LMOD"
            0x00, value,
        ];
        self.write_extended_data(&mut buf)?;
        Ok(())
    }

    /// Sets keyboard RGB backlight color using CRT COLOR command.
    /// K1Pro devices only.
    pub fn set_keyboard_rgb(&self, r: u8, g: u8, b: u8) -> Result<(), MirajazzError> {
        self.initialize()?;
        let mut buf = vec![
            self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00,
            0x43, 0x4f, 0x4c, 0x4f, 0x52, // "COLOR"
            r, g, b,
        ];
        self.write_extended_data(&mut buf)?;
        Ok(())
    }

    /// Sets keyboard OS mode using CRT CPOS command.
    /// mac=true sends 'M' (0x4d), mac=false sends 'W' (0x57). K1Pro devices only.
    pub fn set_keyboard_os_mode(&self, mac: bool) -> Result<(), MirajazzError> {
        self.initialize()?;
        let mode = if mac { 0x4d } else { 0x57 };
        let mut buf = vec![
            self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00,
            0x43, 0x50, 0x4f, 0x53, // "CPOS"
            0x00, mode,
        ];
        self.write_extended_data(&mut buf)?;
        Ok(())
    }

    /// Flushes the button's image to the device
    pub fn flush(&self) -> Result<(), MirajazzError> {
        self.initialize()?;

        if self.image_cache.write()?.is_empty() {
            return Ok(());
        }

        for image in self.image_cache.read()?.iter() {
            self.send_image(image.key, &image.image_data)?;
        }

        let mut buf = vec![self.report_id, 0x43, 0x52, 0x54, 0x00, 0x00, 0x53, 0x54, 0x50];
        self.write_extended_data(&mut buf)?;

        self.image_cache.write()?.clear();

        Ok(())
    }

    /// Returns button state reader for this device
    pub fn get_reader(self: &Arc<Self>) -> Arc<DeviceStateReader> {
        #[allow(clippy::arc_with_non_send_sync)]
        Arc::new(DeviceStateReader {
            device: self.clone(),
            states: Mutex::new(DeviceState {
                buttons: vec![false; self.key_count],
                encoders: vec![false; self.encoder_count],
            }),
        })
    }

    fn write_image_data_reports(&self, image_data: &[u8]) -> Result<(), MirajazzError> {
        let image_report_length = self.packet_size + 1;
        let image_report_payload_length = self.packet_size; // image_report_length - 1 (header byte)

        let mut buf = vec![0u8; image_report_length];
        buf[0] = self.report_id; // Header byte (report ID)

        let mut page_number = 0;
        let mut bytes_remaining = image_data.len();

        while bytes_remaining > 0 {
            let this_length = bytes_remaining.min(image_report_payload_length);
            let bytes_sent = page_number * image_report_payload_length;

            buf[1..1 + this_length].copy_from_slice(&image_data[bytes_sent..bytes_sent + this_length]);
            // Zero-fill padding
            buf[1 + this_length..].fill(0);

            self.write_data(&buf)?;

            bytes_remaining -= this_length;
            page_number += 1;
        }

        Ok(())
    }

    /// Performs get_feature_report on [HidDevice]
    pub fn get_feature_report(
        &self,
        report_id: u8,
        length: usize,
    ) -> Result<Vec<u8>, MirajazzError> {
        let mut buff = vec![0u8; length];

        // Inserting report id byte
        buff.insert(0, report_id);

        // Getting feature report
        self.hid_device.get_feature_report(buff.as_mut_slice())?;

        Ok(buff)
    }

    /// Performs send_feature_report on [HidDevice]
    pub fn send_feature_report(&self, payload: &[u8]) -> Result<(), MirajazzError> {
        self.hid_device.send_feature_report(payload)?;

        Ok(())
    }

    /// Reads data from [HidDevice]. Blocking mode is used if timeout is specified
    pub fn read_data(
        &self,
        length: usize,
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>, MirajazzError> {
        self.hid_device.set_blocking_mode(timeout.is_some())?;

        let mut buf = vec![0u8; length];

        match timeout {
            Some(timeout) => self
                .hid_device
                .read_timeout(buf.as_mut_slice(), timeout.as_millis() as i32),
            None => self.hid_device.read(buf.as_mut_slice()),
        }?;

        Ok(buf)
    }

    /// Writes data to [HidDevice]
    pub fn write_data(&self, payload: &[u8]) -> Result<usize, MirajazzError> {
        Ok(self.hid_device.write(payload)?)
    }

    /// Writes data to [HidDevice]
    pub fn write_extended_data(&self, payload: &mut Vec<u8>) -> Result<usize, MirajazzError> {
        payload.extend(vec![0u8; 1 + self.packet_size - payload.len()]);

        Ok(self.hid_device.write(payload)?)
    }
}
