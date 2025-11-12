// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::kwin_script::KWinScriptClient;

/// Sets focus to a window matching the given class and title on Wayland
///
/// Uses KWin D-Bus scripting API for event-driven window activation.
/// The matching is case-insensitive substring matching.
pub fn set_focus(class: &String, title: &String) -> Result<(), String> {
    // Create KWin client for this activation
    let client = KWinScriptClient::new()
        .map_err(|e| format!("Failed to create KWin client: {}", e))?;

    // Use the event-driven activate_window method
    // If window is not found, log a warning but don't fail
    if let Err(e) = client.activate_window(class, title) {
        crate::verbose_log!("Warning: Failed to activate window: {}", e);
    }

    Ok(())
}
