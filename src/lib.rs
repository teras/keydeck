// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

// Public library interface for keydeck
// This allows other crates (like keydeck-config) to use keydeck's types

use std::sync::atomic::AtomicU8;

/// Verbosity level: 0 = normal, 1 = detailed, 2 = verbose/debug
pub static VERBOSITY: AtomicU8 = AtomicU8::new(0);

#[macro_use]
pub mod utils;

pub mod device_info;
pub mod device_trait;
pub mod dynamic_detection;
pub mod elgato_device;
pub mod mirajazz_device;
pub mod pages;
pub mod system_info;
pub mod text_renderer;

// Re-export types from keydeck-types
pub use keydeck_types::{
    get_icon_dir, get_icon_dir_path, Action, Button, ButtonConfig, ButtonImage, ButtonLayout,
    ColorMapEntry, DeviceInfo, Direction, DrawConfig, FocusChangeRestorePolicy, GraphicType,
    KeyDeckConf, LcdStrip, Macro, MacroCall, Page, Pages, RefreshTarget, ServiceConfig, TextConfig,
    DEFAULT_ICON_DIR_REL,
};

// Re-export backend-specific loader
pub use pages::KeyDeckConfLoader;
