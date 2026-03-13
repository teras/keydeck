// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::keyboard_wayland::WaylandKeyboardSession;
use crate::session::{detect_session_type, SessionType};
use std::str::FromStr;
use std::sync::LazyLock;
use std::{thread, time::Duration};
use strum::{Display, EnumString};
use x11rb::protocol::xproto::{GetKeyboardMappingReply, Keycode, Keysym};
use x11rb::{
    connection::Connection,
    protocol::{xproto::ConnectionExt, xtest},
    rust_connection::RustConnection,
    NONE,
};

const KEY_PRESS: u8 = 2;
const KEY_RELEASE: u8 = 3;

/// Delay in milliseconds between key press/release events.
/// This ensures the X server has time to process each key event properly.
/// Reducing this value makes key sending faster but may cause issues on slower systems.
const KEY_EVENT_DELAY_MS: u64 = 5;

// --- Dispatch layer ---

enum KeyboardBackend {
    X11,
    Wayland(WaylandKeyboardSession),
}

static KEYBOARD: LazyLock<KeyboardBackend> = LazyLock::new(|| {
    match detect_session_type() {
        SessionType::X11 => KeyboardBackend::X11,
        SessionType::Wayland => match WaylandKeyboardSession::new() {
            Ok(session) => {
                eprintln!("Using Wayland RemoteDesktop portal for keyboard input");
                KeyboardBackend::Wayland(session)
            }
            Err(e) => {
                eprintln!(
                    "Failed to create Wayland keyboard session: {}, falling back to X11/XWayland",
                    e
                );
                KeyboardBackend::X11
            }
        },
    }
});

pub fn send_key_combination(combination: &str) -> Result<(), String> {
    match &*KEYBOARD {
        KeyboardBackend::X11 => send_key_combination_x11(combination),
        KeyboardBackend::Wayland(session) => session.send_key_combination(combination),
    }
}

pub fn send_string(text: &str) -> Result<(), String> {
    match &*KEYBOARD {
        KeyboardBackend::X11 => send_string_x11(text),
        KeyboardBackend::Wayland(session) => session.send_string(text),
    }
}

// --- Shared utilities ---

/// Processes escape sequences in a string and returns the actual characters.
/// Supported escape sequences: \n (newline/Enter), \t (Tab), \r (carriage return), \\ (backslash), \e (Escape)
/// This is used both for keyboard input and for text display rendering.
pub fn process_escape_sequences(text: &str) -> Vec<char> {
    let mut result = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next_ch) = chars.peek() {
                match next_ch {
                    'n' => {
                        chars.next();
                        result.push('\n');
                    }
                    't' => {
                        chars.next();
                        result.push('\t');
                    }
                    'r' => {
                        chars.next();
                        result.push('\r');
                    }
                    '\\' => {
                        chars.next();
                        result.push('\\');
                    }
                    'e' => {
                        chars.next();
                        result.push('\x1b');
                    }
                    _ => {
                        result.push(ch);
                    }
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

// --- X11 backend ---

// Modifier keys as an enum with direct mapping to keysyms
#[derive(Debug, PartialEq, Eq)]
#[repr(u32)]
#[derive(EnumString, Display)]
#[strum(serialize_all = "lowercase")]
enum Keys {
    #[strum(serialize = "esc", serialize = "escape")]
    Esc = 0xff1b,
    #[strum(
        serialize = "ctrl",
        serialize = "lctrl",
        serialize = "control",
        serialize = "lcontrol"
    )]
    LCtrl = 0xffe3,
    #[strum(serialize = "rctrl", serialize = "rcontrol")]
    RCtrl = 0xffe4,
    #[strum(serialize = "alt", serialize = "lalt")]
    LAlt = 0xffe9,
    RAlt = 0xffea,
    #[strum(serialize = "shift", serialize = "lshift")]
    LShift = 0xffe1,
    RShift = 0xffe2,
    #[strum(serialize = "super", serialize = "lsuper")]
    LSuper = 0xffeb,
    RSuper = 0xffec,
    AltGr = 0xfe03,
    F1 = 0xffbe,
    F2 = 0xffbf,
    F3 = 0xffc0,
    F4 = 0xffc1,
    F5 = 0xffc2,
    F6 = 0xffc3,
    F7 = 0xffc4,
    F8 = 0xffc5,
    F9 = 0xffc6,
    F10 = 0xffc7,
    F11 = 0xffc8,
    F12 = 0xffc9,
    NumLock = 0xff7f,
    ScrollLock = 0xff14,
    CapsLock = 0xffe5,
    Insert = 0xff63,
    Delete = 0xffff,
    Home = 0xff50,
    End = 0xff57,
    PageUp = 0xff55,
    PageDown = 0xff56,
    PrintScreen = 0xff61,
    Pause = 0xff13,
    Menu = 0xff67,
    Space = 0x0020,
    Tab = 0xff09,
    Backspace = 0xff08,
    Enter = 0xff0d,
    ArrowUp = 0xff52,
    ArrowDown = 0xff54,
    ArrowLeft = 0xff51,
    ArrowRight = 0xff53,
    #[strum(serialize = "volumeup", serialize = "audiovolumeup")]
    VolumeUp = 0x1008ff13,
    #[strum(serialize = "volumedown", serialize = "audiovolumedown")]
    VolumeDown = 0x1008ff11,
    #[strum(serialize = "volumemute", serialize = "audiovolumemute")]
    VolumeMute = 0x1008ff12,
    #[strum(serialize = "micmute", serialize = "audiomicmute")]
    MicMute = 0x1008ffb2,
    #[strum(serialize = "mediaplaypause", serialize = "playpause")]
    MediaPlayPause = 0x1008ff14,
    MediaStop = 0x1008ff15,
    #[strum(serialize = "medianext", serialize = "nexttrack")]
    MediaNext = 0x1008ff17,
    #[strum(
        serialize = "mediaprev",
        serialize = "prevtrack",
        serialize = "mediatrackprevious"
    )]
    MediaPrev = 0x1008ff16,
    #[strum(serialize = "brightnessup", serialize = "monbrightnessup")]
    BrightnessUp = 0x1008ff02,
    #[strum(serialize = "brightnessdown", serialize = "monbrightnessdown")]
    BrightnessDown = 0x1008ff03,
    BrowserBack = 0x1008ff26,
    BrowserForward = 0x1008ff27,
    BrowserRefresh = 0x1008ff29,
    BrowserHome = 0x1008ff18,
    BrowserSearch = 0x1008ff1b,
    BrowserFavorites = 0x1008ff30,
    LaunchMail = 0x1008ff19,
    LaunchCalculator = 0x1008ff1d,
    LaunchExplorer = 0x1008ff5d,
    #[strum(serialize = "sleep", serialize = "standby")]
    Sleep = 0x1008ff2f,
    Eject = 0x1008ff2c,
}

fn keysym_for_char(ch: char) -> Result<(Keysym, bool), String> {
    match ch {
        'a'..='z' => Ok((ch as u32 - 'a' as u32 + 0x61, false)),
        'A'..='Z' => Ok((ch as u32 - 'A' as u32 + 0x61, true)),
        '0'..='9' => Ok((ch as u32 - '0' as u32 + 0x30, false)),
        ' ' => Ok((0x20, false)),
        '!' => Ok((0x31, true)),
        '@' => Ok((0x32, true)),
        '#' => Ok((0x33, true)),
        '$' => Ok((0x34, true)),
        '%' => Ok((0x35, true)),
        '^' => Ok((0x36, true)),
        '&' => Ok((0x37, true)),
        '*' => Ok((0x38, true)),
        '(' => Ok((0x39, true)),
        ')' => Ok((0x30, true)),
        '_' => Ok((0x2D, true)),
        '+' => Ok((0x3D, true)),
        '{' => Ok((0x5B, true)),
        '}' => Ok((0x5D, true)),
        ':' => Ok((0x3B, true)),
        '"' => Ok((0x27, true)),
        '<' => Ok((0x2C, true)),
        '>' => Ok((0x2E, true)),
        '?' => Ok((0x2F, true)),
        '~' => Ok((0x60, true)),
        '|' => Ok((0x5C, true)),
        '-' => Ok((0x2D, false)),
        '=' => Ok((0x3D, false)),
        '[' => Ok((0x5B, false)),
        ']' => Ok((0x5D, false)),
        ';' => Ok((0x3B, false)),
        '\'' => Ok((0x27, false)),
        ',' => Ok((0x2C, false)),
        '.' => Ok((0x2E, false)),
        '/' => Ok((0x2F, false)),
        '\\' => Ok((0x5C, false)),
        '`' => Ok((0x60, false)),
        _ => Err(format!("Unsupported character: {}", ch)),
    }
}

fn keysym_to_keycode(
    keysym: Keysym,
    keysym_mapping: &GetKeyboardMappingReply,
    min_keycode: Keycode,
) -> Result<u8, String> {
    for (i, keysym_list) in keysym_mapping
        .keysyms
        .chunks(keysym_mapping.keysyms_per_keycode as usize)
        .enumerate()
    {
        for &mapped_keysym in keysym_list {
            if mapped_keysym == keysym {
                return Ok(min_keycode + i as u8);
            }
        }
    }
    Err(format!("Keysym not found: {}", keysym))
}

fn send_key_event(keycode: &u8, conn: &RustConnection, event_type: u8) -> Result<(), String> {
    let device_id = 0;
    if let Err(e) = xtest::fake_input(
        conn,
        event_type,
        *keycode,
        x11rb::CURRENT_TIME,
        NONE,
        0,
        0,
        device_id,
    ) {
        return Err(format!("Error sending key event: {}", e));
    }
    if let Err(e) = conn.flush() {
        return Err(format!("Error flushing connection: {}", e));
    }
    Ok(())
}

fn press_key_combination(
    combo: &str,
    conn: &RustConnection,
    keyboard_mapping: &GetKeyboardMappingReply,
    min_keycode: Keycode,
) -> Result<(), String> {
    let parts: Vec<&str> = combo.split('+').collect();
    let mut keycodes: Vec<u8> = Vec::new();
    for part in parts {
        if part.is_empty() {
            return Err("Empty key part in key combination".to_string());
        }
        let keysym = if part.len() == 1 {
            let (keysym, _) = keysym_for_char(part.chars().next().unwrap())?;
            keysym
        } else {
            Keys::from_str(&part.to_lowercase()).map_err(|e| e.to_string())? as u32
        };
        keycodes.push(keysym_to_keycode(keysym, keyboard_mapping, min_keycode)?);
    }

    for keycode in &keycodes {
        send_key_event(keycode, conn, KEY_PRESS)?;
        thread::sleep(Duration::from_millis(KEY_EVENT_DELAY_MS));
    }
    for keycode in keycodes.iter().rev() {
        send_key_event(keycode, conn, KEY_RELEASE)?;
        thread::sleep(Duration::from_millis(KEY_EVENT_DELAY_MS));
    }
    Ok(())
}

fn send_key_combination_x11(combination: &str) -> Result<(), String> {
    let (conn, _) = RustConnection::connect(None).map_err(|e| e.to_string())?;

    let xtest_available = conn
        .query_extension(b"XTEST")
        .map(|r| r.reply().map(|x| x.present).unwrap_or(false))
        .unwrap_or(false);
    if !xtest_available {
        return Err("XTest extension is not available".to_string());
    }

    let setup = conn.setup();
    let min_keycode = setup.min_keycode;
    let max_keycode = setup.max_keycode;

    let keyboard_mapping = conn
        .get_keyboard_mapping(min_keycode, max_keycode - min_keycode + 1)
        .map_err(|e| e.to_string())?;
    let keysym_mapping = keyboard_mapping.reply().map_err(|e| e.to_string())?;

    press_key_combination(combination, &conn, &keysym_mapping, min_keycode)
        .unwrap_or_else(|e| eprintln!("{}", e));
    Ok(())
}

fn keysym_for_control_char(ch: char) -> Option<Keysym> {
    match ch {
        '\n' | '\r' => Some(0xff0d),
        '\t' => Some(0xff09),
        '\x1b' => Some(0xff1b),
        _ => None,
    }
}

fn send_string_x11(text: &str) -> Result<(), String> {
    let (conn, _) = RustConnection::connect(None).map_err(|e| e.to_string())?;

    let xtest_available = conn
        .query_extension(b"XTEST")
        .map(|r| r.reply().map(|x| x.present).unwrap_or(false))
        .unwrap_or(false);
    if !xtest_available {
        return Err("XTest extension is not available".to_string());
    }

    let setup = conn.setup();
    let min_keycode = setup.min_keycode;
    let max_keycode = setup.max_keycode;

    let keyboard_mapping = conn
        .get_keyboard_mapping(min_keycode, max_keycode - min_keycode + 1)
        .map_err(|e| e.to_string())?;
    let keysym_mapping = keyboard_mapping.reply().map_err(|e| e.to_string())?;

    let shift_keysym = Keys::LShift as u32;
    let shift_keycode = keysym_to_keycode(shift_keysym, &keysym_mapping, min_keycode)?;

    let processed_chars = process_escape_sequences(text);

    for ch in processed_chars {
        if let Some(control_keysym) = keysym_for_control_char(ch) {
            let keycode = keysym_to_keycode(control_keysym, &keysym_mapping, min_keycode)?;
            send_key_event(&keycode, &conn, KEY_PRESS)?;
            thread::sleep(Duration::from_millis(KEY_EVENT_DELAY_MS));
            send_key_event(&keycode, &conn, KEY_RELEASE)?;
            thread::sleep(Duration::from_millis(KEY_EVENT_DELAY_MS));
        } else {
            let (keysym, needs_shift) = keysym_for_char(ch)?;
            let keycode = keysym_to_keycode(keysym, &keysym_mapping, min_keycode)?;

            if needs_shift {
                send_key_event(&shift_keycode, &conn, KEY_PRESS)?;
                thread::sleep(Duration::from_millis(KEY_EVENT_DELAY_MS));
            }

            send_key_event(&keycode, &conn, KEY_PRESS)?;
            thread::sleep(Duration::from_millis(KEY_EVENT_DELAY_MS));
            send_key_event(&keycode, &conn, KEY_RELEASE)?;
            thread::sleep(Duration::from_millis(KEY_EVENT_DELAY_MS));

            if needs_shift {
                send_key_event(&shift_keycode, &conn, KEY_RELEASE)?;
                thread::sleep(Duration::from_millis(KEY_EVENT_DELAY_MS));
            }
        }
    }

    Ok(())
}
