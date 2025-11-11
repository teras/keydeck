// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use std::env;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionType {
    X11,
    Wayland,
}

/// Detects whether we're running under X11 or Wayland
pub fn detect_session_type() -> SessionType {
    match env::var("XDG_SESSION_TYPE") {
        Ok(session_type) => {
            if session_type.to_lowercase() == "wayland" {
                SessionType::Wayland
            } else {
                SessionType::X11
            }
        }
        Err(_) => {
            // Fallback: check if WAYLAND_DISPLAY is set
            if env::var("WAYLAND_DISPLAY").is_ok() {
                SessionType::Wayland
            } else {
                // Default to X11
                SessionType::X11
            }
        }
    }
}
