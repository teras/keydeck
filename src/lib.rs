// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

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

// Re-export types from keydeck-types
pub use keydeck_types::{
    KeyDeckConf, Pages, Page, Button, ButtonConfig, Action, TextConfig, DrawConfig,
    ServiceConfig, Macro, MacroCall, FocusChangeRestorePolicy, GraphicType, Direction,
    ColorMapEntry, RefreshTarget, DeviceInfo, ButtonLayout, ButtonImage, LcdStrip,
    DEFAULT_ICON_DIR_REL, get_icon_dir, get_icon_dir_path,
};

// Re-export backend-specific loader
pub use pages::KeyDeckConfLoader;
