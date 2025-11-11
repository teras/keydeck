// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ClientMessageData, ClientMessageEvent, ConnectionExt, EventMask, Window,
    CLIENT_MESSAGE_EVENT,
};
use x11rb::rust_connection::RustConnection;
use crate::session::{detect_session_type, SessionType};

pub fn set_focus(class: &String, title: &String) -> Result<(), String> {
    match detect_session_type() {
        SessionType::X11 => set_focus_x11(class, title),
        SessionType::Wayland => crate::focus_property_wayland::set_focus(class, title),
    }
}

fn set_focus_x11(class: &String, title: &String) -> Result<(), String> {
    // Ensure at least one of class or title is specified
    if class.is_empty() && title.is_empty() {
        return Err("At least one of class or title must be specified".to_string());
    }

    // Connect to the X server
    let (conn, screen_num) = RustConnection::connect(None)
        .map_err(|e| format!("Failed to connect to X server: {}", e))?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    // Intern necessary atoms
    let net_client_list_atom = intern_atom(&conn, b"_NET_CLIENT_LIST")?;
    let net_wm_name_atom = intern_atom(&conn, b"_NET_WM_NAME")?;
    let utf8_string_atom = intern_atom(&conn, b"UTF8_STRING")?;
    let net_active_window_atom = intern_atom(&conn, b"_NET_ACTIVE_WINDOW")?;
    let wm_class_atom = AtomEnum::WM_CLASS.into();
    let wm_name_atom = AtomEnum::WM_NAME.into();

    // Get the list of top-level windows
    let client_list = conn
        .get_property::<Atom, Atom>(
            false,
            root,
            net_client_list_atom,
            AtomEnum::WINDOW.into(),
            0,
            u32::MAX,
        )
        .map_err(|e| format!("Failed to get _NET_CLIENT_LIST property: {}", e))?
        .reply()
        .map_err(|e| format!("Failed to get reply for _NET_CLIENT_LIST property: {}", e))?;

    if client_list.format != 32 {
        return Err("Invalid format for _NET_CLIENT_LIST property".to_string());
    }

    let window_ids = client_list
        .value32()
        .ok_or_else(|| "Failed to parse _NET_CLIENT_LIST property".to_string())?;

    // Iterate over each window and attempt to find a match
    for window in window_ids {
        // Get WM_CLASS
        let wm_class = get_wm_class(&conn, window, wm_class_atom)?;
        // Get window title
        let window_title = get_window_title(
            &conn,
            window,
            net_wm_name_atom,
            utf8_string_atom,
            wm_name_atom,
        )?;

        // Perform matching
        // If both class and title are empty, skip (shouldn't happen due to earlier check)
        // If title is empty, match by class only
        // If class is empty, match by title only
        // If both are provided AND they're the same string, use OR logic (either matches)
        // If both are provided AND they're different, use AND logic (both must match)

        let use_or_logic = !class.is_empty() && !title.is_empty() && class == title;

        let class_match = if !class.is_empty() {
            if let Some((res_name, res_class)) = &wm_class {
                let class_target_lower = class.to_lowercase();
                res_name.to_lowercase().contains(&class_target_lower)
                    || res_class.to_lowercase().contains(&class_target_lower)
            } else {
                false
            }
        } else {
            !use_or_logic // If class empty and using OR logic, don't automatically match
        };

        let title_match = if !title.is_empty() {
            if let Some(window_title) = &window_title {
                let title_target_lower = title.to_lowercase();
                window_title.to_lowercase().contains(&title_target_lower)
            } else {
                false
            }
        } else {
            !use_or_logic // If title empty and using OR logic, don't automatically match
        };

        // Use OR logic if both parameters are the same string, AND logic otherwise
        let matches = if use_or_logic {
            class_match || title_match
        } else {
            class_match && title_match
        };

        if matches {
            // Construct the ClientMessage data
            let data32 = [
                2, // Source indication (2 = pager)
                0, // Timestamp (0 means CurrentTime)
                0, // Flags (set to 0)
                0,
                0,
            ];

            let data: ClientMessageData = data32.into();

            let event = ClientMessageEvent {
                response_type: CLIENT_MESSAGE_EVENT,
                format: 32,
                sequence: 0,
                window, // The window we want to activate
                type_: net_active_window_atom,
                data,
            };

            // Send the event to the root window
            conn.send_event(
                false,
                root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                &event,
            )
                .map_err(|e| format!("Failed to send _NET_ACTIVE_WINDOW event: {}", e))?;

            // Flush the request to ensure it's sent
            conn.flush()
                .map_err(|e| format!("Failed to flush X connection: {}", e))?;

            return Ok(());
        }
    }

    Err(format!("No matching window found using class '{}' and title '{}'", class, title))
}

/// Helper function to intern an atom and return its identifier.
fn intern_atom(conn: &RustConnection, name: &[u8]) -> Result<Atom, String> {
    conn.intern_atom(false, name)
        .map_err(|e| format!("Failed to intern atom {:?}: {}", name, e))?
        .reply()
        .map_err(|e| format!("Failed to get reply for atom {:?}: {}", name, e))
        .map(|r| r.atom)
}

/// Retrieves the `WM_CLASS` property of a window.
fn get_wm_class(
    conn: &RustConnection,
    window: Window,
    wm_class_atom: Atom,
) -> Result<Option<(String, String)>, String> {
    let reply = conn
        .get_property::<Atom, Atom>(
            false,
            window,
            wm_class_atom,
            AtomEnum::STRING.into(),
            0,
            u32::MAX,
        )
        .map_err(|e| format!("Failed to get WM_CLASS property: {}", e))?
        .reply()
        .map_err(|e| format!("Failed to get reply for WM_CLASS property: {}", e))?;

    if reply.value_len == 0 {
        return Ok(None);
    }

    let value = reply.value;
    let parts = value.split(|&b| b == 0).collect::<Vec<_>>();

    if parts.len() < 2 {
        return Ok(None);
    }

    let res_name = String::from_utf8(parts[0].to_vec())
        .map_err(|e| format!("Failed to parse res_name from WM_CLASS: {}", e))?;
    let res_class = String::from_utf8(parts[1].to_vec())
        .map_err(|e| format!("Failed to parse res_class from WM_CLASS: {}", e))?;

    Ok(Some((res_name, res_class)))
}

/// Retrieves the window title using `_NET_WM_NAME` or falls back to `WM_NAME`.
fn get_window_title(
    conn: &RustConnection,
    window: Window,
    net_wm_name_atom: Atom,
    utf8_string_atom: Atom,
    wm_name_atom: Atom,
) -> Result<Option<String>, String> {
    // Attempt to get _NET_WM_NAME with UTF8_STRING encoding
    if let Ok(cookie) = conn.get_property::<Atom, Atom>(
        false,
        window,
        net_wm_name_atom,
        utf8_string_atom,
        0,
        u32::MAX,
    ) {
        if let Ok(reply) = cookie.reply() {
            if reply.value_len > 0 {
                if let Ok(title) = String::from_utf8(reply.value) {
                    return Ok(Some(title));
                }
            }
        }
    }

    // Attempt to get _NET_WM_NAME with STRING encoding
    if let Ok(cookie) = conn.get_property::<Atom, Atom>(
        false,
        window,
        net_wm_name_atom,
        AtomEnum::STRING.into(), // Added `.into()`
        0,
        u32::MAX,
    ) {
        if let Ok(reply) = cookie.reply() {
            if reply.value_len > 0 {
                if let Ok(title) = String::from_utf8(reply.value) {
                    return Ok(Some(title));
                }
            }
        }
    }

    // Fallback to WM_NAME with STRING encoding
    if let Ok(cookie) = conn.get_property::<Atom, Atom>(
        false,
        window,
        wm_name_atom,
        AtomEnum::STRING.into(), // Added `.into()`
        0,
        u32::MAX,
    ) {
        if let Ok(reply) = cookie.reply() {
            if reply.value_len > 0 {
                if let Ok(title) = String::from_utf8(reply.value) {
                    return Ok(Some(title));
                }
            }
        }
    }

    Ok(None)
}
