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
    ColorMapEntry, RefreshTarget,
};

// Re-export device info types
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
