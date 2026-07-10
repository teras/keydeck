// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! Cross-platform keyboard injection for the Windows and macOS backends,
//! built on the [`enigo`] crate.
//!
//! Parses the same key-name grammar the Linux X11 backend understands
//! (e.g. `"LCtrl+LShift+z"`, `"F12"`, `"volumeup"`) into `enigo::Key` values
//! so configurations are portable across all platforms.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use crate::platform::process_escape_sequences;

/// Runs `f` with a fresh `Enigo` instance.
///
/// A new instance is created per call rather than cached in a `static`: on
/// macOS `Enigo` owns a `CGEventSource` which is not `Send`, so it cannot live
/// in a shared static. Key events fire only on user interaction (never a hot
/// path), so the small setup cost is irrelevant.
fn with_enigo<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&mut Enigo) -> Result<R, String>,
{
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Failed to initialize input backend: {e}"))?;
    f(&mut enigo)
}

/// Platform-specific raw key code for exotic keys with no portable
/// `enigo::Key` variant (media, browser, launcher keys). Returns `None`
/// when the key is not expressible on the current platform.
fn raw_key(name: &str) -> Option<Key> {
    #[cfg(target_os = "windows")]
    {
        // Windows Virtual-Key codes.
        let vk: u16 = match name {
            "volumeup" | "audiovolumeup" => 0xAF,
            "volumedown" | "audiovolumedown" => 0xAE,
            "volumemute" | "audiovolumemute" => 0xAD,
            "mediaplaypause" | "playpause" => 0xB3,
            "mediastop" => 0xB2,
            "medianext" | "nexttrack" => 0xB0,
            "mediaprev" | "prevtrack" | "mediatrackprevious" => 0xB1,
            "browserback" => 0xA6,
            "browserforward" => 0xA7,
            "browserrefresh" => 0xA8,
            "browserhome" => 0xAC,
            "browsersearch" => 0xAA,
            "browserfavorites" => 0xAB,
            "launchmail" => 0xB4,
            "launchcalculator" => 0xB7,
            "launchexplorer" => 0xB6,
            "sleep" | "standby" => 0x5F,
            _ => return None,
        };
        Some(Key::Other(vk as u32))
    }
    #[cfg(target_os = "macos")]
    {
        // macOS: media keys are delivered as NSSystemDefined events, which
        // `enigo` cannot emit via Key::Other (which expects a CGKeyCode).
        // Only the keys with a real virtual keycode are expressible here.
        let _ = name;
        None
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = name;
        None
    }
}

/// Parses a single key token into an `enigo::Key`.
fn parse_key(token: &str) -> Result<Key, String> {
    // Single printable character → Unicode key (handles letters, digits,
    // symbols and their implicit shifting via the OS layout).
    let mut chars = token.chars();
    if let (Some(c), None) = (chars.next(), chars.clone().next()) {
        return Ok(Key::Unicode(c));
    }

    let lower = token.to_lowercase();
    let key = match lower.as_str() {
        "esc" | "escape" => Key::Escape,
        "ctrl" | "lctrl" | "control" | "lcontrol" => Key::Control,
        "rctrl" | "rcontrol" => {
            #[cfg(target_os = "windows")]
            {
                Key::RControl
            }
            #[cfg(not(target_os = "windows"))]
            {
                Key::Control
            }
        }
        "alt" | "lalt" => Key::Alt,
        "ralt" | "altgr" => {
            #[cfg(target_os = "windows")]
            {
                Key::RMenu
            }
            #[cfg(not(target_os = "windows"))]
            {
                Key::Alt
            }
        }
        "shift" | "lshift" => Key::Shift,
        "rshift" => {
            #[cfg(target_os = "windows")]
            {
                Key::RShift
            }
            #[cfg(not(target_os = "windows"))]
            {
                Key::Shift
            }
        }
        "super" | "lsuper" | "rsuper" | "win" | "cmd" | "meta" => Key::Meta,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        "capslock" => Key::CapsLock,
        "insert" => {
            #[cfg(target_os = "windows")]
            {
                Key::Insert
            }
            #[cfg(not(target_os = "windows"))]
            {
                Key::Other(0x72)
            }
        }
        "delete" => Key::Delete,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "space" => Key::Space,
        "tab" => Key::Tab,
        "backspace" => Key::Backspace,
        "enter" | "return" => Key::Return,
        "arrowup" | "up" => Key::UpArrow,
        "arrowdown" | "down" => Key::DownArrow,
        "arrowleft" | "left" => Key::LeftArrow,
        "arrowright" | "right" => Key::RightArrow,
        _ => raw_key(&lower).ok_or_else(|| format!("Unsupported key: '{token}'"))?,
    };
    Ok(key)
}

/// Sends a key combination such as `"LCtrl+LShift+z"`.
/// All keys are pressed in order, then released in reverse order.
pub fn send_key_combination(combination: &str) -> Result<(), String> {
    let keys: Vec<Key> = combination
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(parse_key)
        .collect::<Result<_, _>>()?;

    if keys.is_empty() {
        return Err("Empty key combination".to_string());
    }

    with_enigo(|enigo| {
        for key in &keys {
            enigo
                .key(*key, Direction::Press)
                .map_err(|e| format!("Failed to press key: {e}"))?;
        }
        for key in keys.iter().rev() {
            enigo
                .key(*key, Direction::Release)
                .map_err(|e| format!("Failed to release key: {e}"))?;
        }
        Ok(())
    })
}

/// Sends a string as individual keystrokes, honouring escape sequences
/// (`\n`, `\t`, `\r`, `\\`, `\e`).
///
/// Printable runs are typed with `enigo.text()`, which uses layout-independent
/// Unicode input and therefore reproduces capitals and shifted symbols
/// correctly (unlike per-character `Key::Unicode`, which sends the bare
/// physical key without its shift modifier). Control characters are emitted as
/// dedicated key presses.
pub fn send_string(text: &str) -> Result<(), String> {
    let chars = process_escape_sequences(text);
    with_enigo(|enigo| {
        let mut buffer = String::new();
        for ch in chars {
            let control = match ch {
                '\n' | '\r' => Some(Key::Return),
                '\t' => Some(Key::Tab),
                '\x1b' => Some(Key::Escape),
                _ => None,
            };
            match control {
                Some(key) => {
                    if !buffer.is_empty() {
                        enigo
                            .text(&buffer)
                            .map_err(|e| format!("Failed to type text: {e}"))?;
                        buffer.clear();
                    }
                    enigo
                        .key(key, Direction::Click)
                        .map_err(|e| format!("Failed to send control key: {e}"))?;
                }
                None => buffer.push(ch),
            }
        }
        if !buffer.is_empty() {
            enigo
                .text(&buffer)
                .map_err(|e| format!("Failed to type text: {e}"))?;
        }
        Ok(())
    })
}
