// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! Shared types and data structures for KeyDeck
//!
//! This crate contains the core data structures used by both the KeyDeck daemon
//! and the KeyDeck configuration UI. It has minimal dependencies to keep build
//! times fast and binary sizes small.

use std::path::PathBuf;

pub mod pages;
pub mod device_info;

// Re-export commonly used types from pages module
pub use pages::{
    KeyDeckConf, Pages, Page, Button, ButtonConfig, Action, TextConfig, DrawConfig,
    ServiceConfig, Macro, MacroCall, FocusChangeRestorePolicy, GraphicType, Direction,
    ColorMapEntry, RefreshTarget, PressEffectConfig, Encoder,
};

// Re-export device info types
pub use device_info::{
    DeviceInfo, ButtonLayout, ButtonImage, LcdStrip,
};

/// Default icon directory path relative to home directory (Linux legacy layout).
pub const DEFAULT_ICON_DIR_REL: &str = ".config/keydeck/icons";

/// Returns the KeyDeck configuration directory for the current platform.
///
/// * Linux: `~/.config/keydeck`
/// * Windows: `%APPDATA%\keydeck`
/// * macOS: `~/Library/Application Support/keydeck`
///
/// Falls back to `~/.config/keydeck` (or a relative path) if the platform
/// config directory cannot be determined.
pub fn get_config_dir() -> PathBuf {
    if let Some(dir) = dirs::config_dir() {
        dir.join("keydeck")
    } else if let Some(home) = dirs::home_dir() {
        home.join(".config").join("keydeck")
    } else {
        PathBuf::from(".config").join("keydeck")
    }
}

/// Absolute path to the configuration file (`config.yaml`) in the config dir.
pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.yaml")
}

/// Get the absolute path to the default icon directory.
pub fn get_icon_dir() -> String {
    get_icon_dir_path().to_string_lossy().into_owned()
}

/// Get the icon directory as PathBuf.
pub fn get_icon_dir_path() -> PathBuf {
    get_config_dir().join("icons")
}
