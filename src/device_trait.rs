// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

#[allow(unused_imports)]
use crate::warn_log;
use image::DynamicImage;
use std::fmt;
use std::sync::Arc;

/// Unified error type for all device implementations
#[derive(Debug)]
pub enum DeviceError {
    ConnectionFailed(String),
    UnsupportedOperation(String),
    InvalidParameter(String),
    IoError(String),
    LibraryError(String),
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            DeviceError::UnsupportedOperation(msg) => write!(f, "Unsupported operation: {}", msg),
            DeviceError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            DeviceError::IoError(msg) => write!(f, "I/O error: {}", msg),
            DeviceError::LibraryError(msg) => write!(f, "Library error: {}", msg),
        }
    }
}

impl std::error::Error for DeviceError {}

/// Convert from String error (used by current device_manager.rs)
impl From<String> for DeviceError {
    fn from(s: String) -> Self {
        DeviceError::LibraryError(s)
    }
}

/// Convert to String error (for compatibility with existing code)
impl From<DeviceError> for String {
    fn from(e: DeviceError) -> Self {
        e.to_string()
    }
}

/// Trait for any device reader that can provide button events
pub trait DeviceReader: Send + Sync {
    fn read(
        &self,
        timeout: Option<std::time::Duration>,
    ) -> Result<Vec<DeviceStateUpdate>, DeviceError>;
}

/// Device state update events
#[derive(Debug, Clone)]
pub enum DeviceStateUpdate {
    ButtonDown(u8),
    ButtonUp(u8),
    EncoderDown(u8),
    EncoderUp(u8),
    EncoderTwist {
        encoder: u8,
        ticks: i8,
    },
    TouchPointDown(u8),
    TouchPointUp(u8),
    TouchScreenPress {
        x: u16,
        y: u16,
    },
    TouchScreenLongPress {
        x: u16,
        y: u16,
    },
    TouchScreenSwipe {
        x: u16,
        y: u16,
        target_x: u16,
        target_y: u16,
    },
}

/// Main device abstraction trait
/// All KeyDeck-compatible devices must implement this trait
#[allow(dead_code)]
pub trait KeydeckDevice: Send + Sync {
    // === Required Device Information ===

    /// Get device serial number
    fn serial_number(&self) -> Result<String, DeviceError>;

    /// Get firmware version
    fn firmware_version(&self) -> Result<String, DeviceError>;

    /// Get device manufacturer name
    fn manufacturer(&self) -> String;

    /// Get device kind/model name
    fn kind_name(&self) -> String;

    /// Get number of buttons on this device
    fn button_count(&self) -> u8;

    /// Check if device has a screen
    fn has_screen(&self) -> bool;

    /// Get button image dimensions (width, height) in pixels
    fn button_image_size(&self) -> (u16, u16);

    /// Get button layout (rows, columns) - optional, returns (0, 0) if not available
    fn button_layout(&self) -> (usize, usize) {
        (0, 0)
    }

    /// Get number of encoders/knobs - optional, returns 0 if not available
    fn encoder_count(&self) -> usize {
        0
    }

    // === Required Device Operations ===

    /// Reset device to factory defaults
    fn reset(&self) -> Result<(), DeviceError>;

    /// Set device brightness (0-100)
    fn set_brightness(&self, brightness: u8) -> Result<(), DeviceError>;

    /// Set image on a specific button
    fn set_button_image(&self, button_idx: u8, image: DynamicImage) -> Result<(), DeviceError>;

    /// Clear image from a specific button
    fn clear_button_image(&self, button_idx: u8) -> Result<(), DeviceError>;

    /// Clear all button images
    fn clear_all_button_images(&self) -> Result<(), DeviceError>;

    /// Flush pending operations to device
    fn flush(&self) -> Result<(), DeviceError>;

    /// Get event reader for this device
    fn get_reader(&self) -> Arc<dyn DeviceReader>;

    /// Whether the device supports separate button down/up events.
    /// When false, button_down visual feedback is skipped (no point sending images).
    fn supports_button_press_feedback(&self) -> bool {
        true // Most devices support this
    }

    // === Optional Device Lifecycle (default no-op) ===

    /// Shutdown device (no-op for most devices)
    /// Used by Ajazz/Mirabox devices to gracefully shutdown
    fn shutdown(&self) -> Result<(), DeviceError> {
        // No-op by default - Elgato devices don't need this
        Ok(())
    }

    // TODO: Call sleep() on idle timeout or system suspend to save power on Ajazz/Mirabox devices
    /// Put device to sleep (no-op for most devices)
    /// Used by Ajazz/Mirabox devices for power management
    fn sleep(&self) -> Result<(), DeviceError> {
        // No-op by default - Elgato devices don't need this
        Ok(())
    }

    /// Send keep-alive signal (no-op for most devices)
    /// Used by Ajazz/Mirabox devices to prevent auto-sleep
    fn keep_alive(&self) {
        // No-op by default - Elgato devices don't need this
    }

    // === LCD/Display Features ===

    // TODO: Implement LCD strip support for Stream Deck Neo/Plus devices
    /// Write to LCD strip (e.g., Stream Deck Neo/Plus)
    fn write_lcd_fill(&self, _x: u16, _y: u16, _image: &DynamicImage) -> Result<(), DeviceError> {
        warn_log!("write_lcd_fill() not supported on this device");
        Ok(())
    }

    /// Write to LCD strip with specific region
    fn write_lcd(
        &self,
        _x: u16,
        _y: u16,
        _width: u16,
        _height: u16,
        _image: &DynamicImage,
    ) -> Result<(), DeviceError> {
        warn_log!("write_lcd() not supported on this device");
        Ok(())
    }

    /// Get background image resolution (width, height), or None if not supported
    fn background_image_size(&self) -> Option<(u16, u16)> {
        None
    }

    /// Set runtime background image displayed behind the keys (CRT BGPIC).
    /// Buttons cleared with `clear_button_image` will show this background through.
    fn set_background_image(&self, _image: DynamicImage) -> Result<(), DeviceError> {
        Ok(()) // No-op for devices without background support
    }

    /// Clear runtime background image (CRT BGCLE).
    fn clear_background_image(&self) -> Result<(), DeviceError> {
        Ok(()) // No-op for devices without background support
    }

    /// Set persistent boot logo written to device flash (CRT LOG).
    /// Survives power cycles. Not the same as runtime background.
    fn set_boot_logo(&self, _image: DynamicImage) -> Result<(), DeviceError> {
        Ok(()) // No-op for devices without boot logo support
    }

    // === RGB LED Strip Control ===

    /// Get number of RGB LEDs, or 0 if device has no LED strip
    fn led_count(&self) -> u8 {
        0
    }

    /// Set RGB LED strip brightness (0-100). No-op for devices without RGB LEDs.
    fn set_led_brightness(&self, _brightness: u8) -> Result<(), DeviceError> {
        Ok(())
    }

    /// Set RGB LED strip colors. Each tuple is (r, g, b) for one LED.
    /// No-op for devices without RGB LEDs.
    fn set_led_color(&self, _colors: &[(u8, u8, u8)]) -> Result<(), DeviceError> {
        Ok(())
    }

    /// Reset RGB LED strip to default state. No-op for devices without RGB LEDs.
    fn reset_led_color(&self) -> Result<(), DeviceError> {
        Ok(())
    }

    // === Power Management ===

    /// Wake device from sleep (CRT DIS).
    fn wakeup(&self) -> Result<(), DeviceError> {
        Ok(())
    }
}
