use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete device definition loaded from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDefinition {
    /// Hardware identification
    pub hardware: HardwareId,

    /// Human-readable information
    pub info: DeviceInfo,

    /// Protocol configuration
    pub protocol: ProtocolConfig,

    /// Physical layout
    pub layout: Layout,

    /// Image format specifications
    pub image_format: ImageFormatConfig,

    /// Input mapping quirks
    #[serde(default)]
    pub input_mapping: InputMapping,

    /// Background/fullscreen image configuration (if device supports it)
    #[serde(default)]
    pub background: Option<BackgroundConfig>,

    /// Device-specific quirks and workarounds
    #[serde(default)]
    pub quirks: Quirks,

    /// RGB LED strip configuration (if device has LEDs)
    #[serde(default)]
    pub led: Option<LedConfig>,
}

/// RGB LED strip configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedConfig {
    /// Number of individually addressable RGB LEDs
    pub count: u8,
}

/// Background/fullscreen LCD image configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundConfig {
    pub resolution: [u16; 2],       // [width, height]
    pub mode: ImageMode,            // JPEG or PNG
    pub rotation: Rotation,
    pub mirror: Mirror,
}

/// HID Usage Page and ID constants (shared by all Mirabox/Ajazz devices)
pub const MIRAJAZZ_USAGE_PAGE: u16 = 65440; // 0xFF00 - Vendor-specific
pub const MIRAJAZZ_USAGE_ID: u16 = 1;

/// Hardware identification (VID/PID)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareId {
    pub vendor_id: String,      // Hex string like "0x0300"
    pub product_id: String,     // Hex string like "0x1020"
}

impl HardwareId {
    /// Parse hex string to u16
    pub fn vendor_id_u16(&self) -> Result<u16, std::num::ParseIntError> {
        u16::from_str_radix(self.vendor_id.trim_start_matches("0x"), 16)
    }

    pub fn product_id_u16(&self) -> Result<u16, std::num::ParseIntError> {
        u16::from_str_radix(self.product_id.trim_start_matches("0x"), 16)
    }
}

/// Human-readable device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub human_name: String,         // "Ajazz AKP153R"
    pub device_namespace: String,   // "99" or "n3" - 2 char plugin identifier
    #[serde(default)]
    pub manufacturer: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// Protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    pub protocol_version: usize,    // 0, 1, 2, or 3

    /// Device mode to set on initialization (for multimodal devices).
    /// Sends MOD command with this value before other commands.
    #[serde(default)]
    pub device_mode: Option<u8>,

    /// HID report ID override (default 0x00).
    /// K1Pro devices use 0x04.
    #[serde(default)]
    pub report_id: Option<u8>,
}

/// Physical device layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub rows: usize,
    pub cols: usize,
    #[serde(default)]
    pub encoder_count: usize,
}

impl Layout {
    /// Calculate total key count (always rows × cols)
    pub fn key_count(&self) -> usize {
        self.rows * self.cols
    }
}

/// Image format configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageFormatConfig {
    pub mode: ImageMode,            // "BMP" or "JPEG"
    pub default_size: [u16; 2],     // [width, height]
    pub rotation: Rotation,         // "Rot0", "Rot90", "Rot180", "Rot270"
    pub mirror: Mirror,             // "None", "X", "Y", "Both"

    /// Per-button image format overrides
    /// Key = button index (0-based), Value = override for that button
    #[serde(default)]
    pub per_button_overrides: HashMap<u8, ImageFormatOverride>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ImageMode {
    BMP,
    JPEG,
    PNG,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Rotation {
    Rot0,
    Rot90,
    Rot180,
    Rot270,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Mirror {
    None,
    X,
    Y,
    Both,
}

/// Per-button format override
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageFormatOverride {
    #[serde(default)]
    pub size: Option<[u16; 2]>,
    #[serde(default)]
    pub rotation: Option<Rotation>,
    #[serde(default)]
    pub mirror: Option<Mirror>,
}

/// Input mapping quirks and translations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputMapping {
    /// Button remapping array (AKP153 quirk)
    /// Maps OpenDeck sequential index → device's actual button ID
    #[serde(default)]
    pub button_remap: Option<Vec<u8>>,

    /// Encoder twist event mapping (AKP03 quirk)
    /// Maps raw input byte → (encoder_index, direction)
    #[serde(default)]
    pub encoder_twist_map: HashMap<u8, EncoderTwist>,

    /// Encoder press event mapping (AKP03 quirk)
    /// Maps raw input byte → encoder_index
    #[serde(default)]
    pub encoder_press_map: HashMap<u8, u8>,

    /// Buttons without displays that should skip image commands (AKP03 quirk)
    #[serde(default)]
    pub non_display_buttons: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderTwist {
    pub encoder: u8,        // Encoder index (0-2)
    pub direction: i8,      // -1 for CCW, +1 for CW
}

/// Device quirks and workarounds
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Quirks {
    /// Device reports non-sequential button IDs that need remapping
    #[serde(default)]
    pub needs_button_remapping: bool,

    /// Device has buttons without displays
    #[serde(default)]
    pub has_non_display_buttons: bool,

    /// Force generation of unique serial instead of reading from USB
    /// (for v1 devices with hardcoded serial "355499441494")
    /// Generated format: "{usb_serial}-{vid:04X}{pid:04X}"
    /// Example: "355499441494-03001010"
    #[serde(default)]
    pub force_serial: bool,

    /// Device image slots and button press keys use different physical orderings.
    /// When true, button_remap is applied only to images (not to input events),
    /// because the device already reports button presses in opendeck order.
    #[serde(default)]
    pub image_remap_only: bool,

    /// Override encoder state detection.
    /// When true, forces encoder toggle mode (synthesize press+release)
    /// even if protocol_version > 2 would normally enable dual states.
    #[serde(default)]
    pub force_encoder_toggle: bool,
}

impl DeviceDefinition {
    /// Get the image format for a specific button index
    pub fn image_format_for_button(&self, button_index: u8) -> ButtonImageFormat {
        let default_format = ButtonImageFormat {
            mode: self.image_format.mode,
            size: self.image_format.default_size,
            rotation: self.image_format.rotation,
            mirror: self.image_format.mirror,
        };

        // Check for override
        if let Some(override_fmt) = self.image_format.per_button_overrides.get(&button_index) {
            ButtonImageFormat {
                mode: self.image_format.mode, // Mode is never overridden
                size: override_fmt.size.unwrap_or(default_format.size),
                rotation: override_fmt.rotation.unwrap_or(default_format.rotation),
                mirror: override_fmt.mirror.unwrap_or(default_format.mirror),
            }
        } else {
            default_format
        }
    }

    /// Check if a button has a display (not in non_display_buttons list)
    pub fn button_has_display(&self, button_index: u8) -> bool {
        !self.input_mapping.non_display_buttons.contains(&button_index)
    }

    /// Map OpenDeck button index to device button index (handles remapping quirk)
    pub fn opendeck_to_device_button(&self, opendeck_index: u8) -> u8 {
        if let Some(remap) = &self.input_mapping.button_remap {
            remap.get(opendeck_index as usize).copied().unwrap_or(opendeck_index)
        } else {
            opendeck_index
        }
    }

    /// Map device button index back to OpenDeck index (reverse mapping)
    /// Skipped when `image_remap_only` quirk is set (device input already uses opendeck order)
    pub fn device_to_opendeck_button(&self, device_index: u8) -> u8 {
        if self.quirks.image_remap_only {
            return device_index;
        }
        if let Some(remap) = &self.input_mapping.button_remap {
            remap.iter()
                .position(|&idx| idx == device_index)
                .map(|pos| pos as u8)
                .unwrap_or(device_index)
        } else {
            device_index
        }
    }
}

/// Resolved image format for a specific button
#[derive(Debug, Clone, Copy)]
pub struct ButtonImageFormat {
    pub mode: ImageMode,
    pub size: [u16; 2],
    pub rotation: Rotation,
    pub mirror: Mirror,
}

// Conversions from registry types to driver ImageFormat

fn convert_mode(mode: ImageMode) -> crate::types::ImageMode {
    match mode {
        ImageMode::BMP => crate::types::ImageMode::BMP,
        ImageMode::JPEG => crate::types::ImageMode::JPEG,
        ImageMode::PNG => crate::types::ImageMode::PNG,
    }
}

fn convert_rotation(rotation: Rotation) -> crate::types::ImageRotation {
    match rotation {
        Rotation::Rot0 => crate::types::ImageRotation::Rot0,
        Rotation::Rot90 => crate::types::ImageRotation::Rot90,
        Rotation::Rot180 => crate::types::ImageRotation::Rot180,
        Rotation::Rot270 => crate::types::ImageRotation::Rot270,
    }
}

fn convert_mirror(mirror: Mirror) -> crate::types::ImageMirroring {
    match mirror {
        Mirror::None => crate::types::ImageMirroring::None,
        Mirror::X => crate::types::ImageMirroring::X,
        Mirror::Y => crate::types::ImageMirroring::Y,
        Mirror::Both => crate::types::ImageMirroring::Both,
    }
}

impl From<ButtonImageFormat> for crate::types::ImageFormat {
    fn from(f: ButtonImageFormat) -> Self {
        Self {
            mode: convert_mode(f.mode),
            size: (f.size[0] as usize, f.size[1] as usize),
            rotation: convert_rotation(f.rotation),
            mirror: convert_mirror(f.mirror),
        }
    }
}

impl From<&BackgroundConfig> for crate::types::ImageFormat {
    fn from(bg: &BackgroundConfig) -> Self {
        Self {
            mode: convert_mode(bg.mode),
            size: (bg.resolution[0] as usize, bg.resolution[1] as usize),
            rotation: convert_rotation(bg.rotation),
            mirror: convert_mirror(bg.mirror),
        }
    }
}
