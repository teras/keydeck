// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

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
    fn read(&self, timeout: Option<std::time::Duration>) -> Result<Vec<DeviceStateUpdate>, DeviceError>;
}

/// Device state update events
#[derive(Debug, Clone)]
pub enum DeviceStateUpdate {
    ButtonDown(u8),
    ButtonUp(u8),
    EncoderDown(u8),
    EncoderUp(u8),
    EncoderTwist { encoder: u8, ticks: i8 },
    TouchPointDown(u8),
    TouchPointUp(u8),
    TouchScreenPress { x: u16, y: u16 },
    TouchScreenLongPress { x: u16, y: u16 },
    TouchScreenSwipe { x: u16, y: u16, target_x: u16, target_y: u16 },
}

/// Main device abstraction trait
/// All KeyDeck-compatible devices must implement this trait
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

    // === Optional Device Lifecycle (default no-op) ===

    /// Shutdown device (no-op for most devices)
    /// Used by Ajazz/Mirabox devices to gracefully shutdown
    fn shutdown(&self) -> Result<(), DeviceError> {
        // No-op by default - Elgato devices don't need this
        Ok(())
    }

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

    // === Optional LCD/Display Features ===

    /// Write to LCD strip (e.g., Stream Deck Neo/Plus)
    /// Default implementation logs warning and does nothing
    fn write_lcd_fill(&self, _x: u16, _y: u16, _image: &DynamicImage) -> Result<(), DeviceError> {
        warn_log!("write_lcd_fill() not supported on this device");
        Ok(())
    }

    /// Write to LCD strip with specific region
    /// Default implementation logs warning and does nothing
    fn write_lcd(&self, _x: u16, _y: u16, _width: u16, _height: u16, _image: &DynamicImage) -> Result<(), DeviceError> {
        warn_log!("write_lcd() not supported on this device");
        Ok(())
    }

    /// Set logo/background image (e.g., Ajazz fullscreen background)
    /// Default implementation logs warning and does nothing
    fn set_logo_image(&self, _image: DynamicImage) -> Result<(), DeviceError> {
        warn_log!("set_logo_image() not supported on this device");
        Ok(())
    }
}
