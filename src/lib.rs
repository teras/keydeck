// Public library interface for keydeck
// This allows other crates (like keydeck-config) to use keydeck's types

use std::sync::atomic::AtomicBool;

/// Debug flag for verbose logging
pub static DEBUG: AtomicBool = AtomicBool::new(false);

#[macro_use]
pub mod utils;

pub mod pages;
pub mod device_info;
pub mod dynamic_detection;
pub mod text_renderer;
pub mod device_trait;
pub mod elgato_device;
pub mod mirajazz_device;

use std::path::PathBuf;

// Re-export commonly used types
pub use pages::{
    KeyDeckConf, Pages, Page, Button, ButtonConfig, Action, TextConfig, DrawConfig,
    ServiceConfig, Macro, MacroCall, FocusChangeRestorePolicy, GraphicType, Direction,
    ColorMapEntry, RefreshTarget,
};

pub use device_info::{
    DeviceInfo, ButtonLayout, ButtonImage, LcdStrip,
};

/// Default icon directory path relative to home directory
pub const DEFAULT_ICON_DIR_REL: &str = ".config/keydeck/icons";

/// Get the absolute path to the default icon directory
pub fn get_icon_dir() -> String {
    if let Ok(home) = std::env::var("HOME") {
        format!("{}/{}", home, DEFAULT_ICON_DIR_REL)
    } else {
        DEFAULT_ICON_DIR_REL.to_string()
    }
}

/// Get the icon directory as PathBuf
pub fn get_icon_dir_path() -> PathBuf {
    PathBuf::from(get_icon_dir())
}
