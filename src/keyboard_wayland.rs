// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop};
use ashpd::desktop::PersistMode;
use std::str::FromStr;
use std::sync::Mutex;
use strum::EnumString;
use tokio::runtime::Runtime;

use crate::keyboard::process_escape_sequences;

/// Wayland keyboard session using xdg-desktop-portal RemoteDesktop.
/// Created once at startup, reused for all keyboard events.
pub struct WaylandKeyboardSession {
    runtime: Runtime,
    proxy: Mutex<Option<SessionInner>>,
}

struct SessionInner {
    proxy: RemoteDesktop<'static>,
    session: ashpd::desktop::Session<'static, RemoteDesktop<'static>>,
}

// Safety: ashpd types are Send, we protect with Mutex
unsafe impl Sync for WaylandKeyboardSession {}

impl WaylandKeyboardSession {
    pub fn new() -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|e| format!("Failed to create tokio runtime: {}", e))?;
        let session = Self {
            runtime,
            proxy: Mutex::new(None),
        };
        session.ensure_session()?;
        Ok(session)
    }

    fn ensure_session(&self) -> Result<(), String> {
        let mut guard = self.proxy.lock().map_err(|e| format!("Lock error: {}", e))?;
        if guard.is_some() {
            return Ok(());
        }

        let inner = self.runtime.block_on(async {
            let proxy = RemoteDesktop::new()
                .await
                .map_err(|e| format!("Failed to create RemoteDesktop proxy: {}", e))?;

            let session = proxy
                .create_session()
                .await
                .map_err(|e| format!("Failed to create session: {}", e))?;

            proxy
                .select_devices(&session, DeviceType::Keyboard.into(), None, PersistMode::DoNot)
                .await
                .map_err(|e| format!("Failed to select devices: {}", e))?;

            let _response = proxy
                .start(&session, None)
                .await
                .map_err(|e| format!("Failed to start session (user denied?): {}", e))?
                .response()
                .map_err(|e| format!("Failed to start session response: {}", e))?;

            Ok::<SessionInner, String>(SessionInner { proxy, session })
        })?;

        *guard = Some(inner);
        Ok(())
    }

    fn send_keycode(&self, keycode: i32, pressed: bool) -> Result<(), String> {
        let guard = self.proxy.lock().map_err(|e| format!("Lock error: {}", e))?;
        let inner = guard.as_ref().ok_or("No active session")?;
        let state = if pressed {
            KeyState::Pressed
        } else {
            KeyState::Released
        };

        self.runtime.block_on(async {
            inner
                .proxy
                .notify_keyboard_keycode(&inner.session, keycode, state)
                .await
                .map_err(|e| format!("Failed to send keycode: {}", e))
        })
    }

    pub fn send_key_combination(&self, combination: &str) -> Result<(), String> {
        let parts: Vec<&str> = combination.split('+').collect();
        let mut keycodes: Vec<i32> = Vec::new();

        for part in parts {
            if part.is_empty() {
                return Err("Empty key part in key combination".to_string());
            }
            let keycode = if part.len() == 1 {
                let ch = part.chars().next().unwrap();
                let (code, _) = evdev_for_char(ch)?;
                code
            } else {
                key_name_to_evdev(&part.to_lowercase())?
            };
            keycodes.push(keycode);
        }

        // Press all keys
        for &keycode in &keycodes {
            self.send_keycode(keycode, true)?;
        }
        // Release in reverse order
        for &keycode in keycodes.iter().rev() {
            self.send_keycode(keycode, false)?;
        }

        Ok(())
    }

    pub fn send_string(&self, text: &str) -> Result<(), String> {
        let processed_chars = process_escape_sequences(text);
        let shift_keycode = EvdevKeys::LShift as i32;

        for ch in processed_chars {
            if let Some(keycode) = evdev_for_control_char(ch) {
                self.send_keycode(keycode, true)?;
                self.send_keycode(keycode, false)?;
            } else {
                let (keycode, needs_shift) = evdev_for_char(ch)?;

                if needs_shift {
                    self.send_keycode(shift_keycode, true)?;
                }
                self.send_keycode(keycode, true)?;
                self.send_keycode(keycode, false)?;
                if needs_shift {
                    self.send_keycode(shift_keycode, false)?;
                }
            }
        }

        Ok(())
    }
}

/// Maps control characters to evdev keycodes
fn evdev_for_control_char(ch: char) -> Option<i32> {
    match ch {
        '\n' | '\r' => Some(EvdevKeys::Enter as i32),
        '\t' => Some(EvdevKeys::Tab as i32),
        '\x1b' => Some(EvdevKeys::Esc as i32),
        _ => None,
    }
}

// Evdev key constants (from linux/input-event-codes.h)

// Key name to evdev keycode mapping
#[derive(Debug, PartialEq, Eq)]
#[repr(i32)]
#[derive(EnumString)]
#[strum(serialize_all = "lowercase")]
enum EvdevKeys {
    #[strum(serialize = "esc", serialize = "escape")]
    Esc = 1,
    #[strum(
        serialize = "ctrl",
        serialize = "lctrl",
        serialize = "control",
        serialize = "lcontrol"
    )]
    LCtrl = 29,
    #[strum(serialize = "rctrl", serialize = "rcontrol")]
    RCtrl = 97,
    #[strum(serialize = "alt", serialize = "lalt")]
    LAlt = 56,
    #[strum(serialize = "ralt", serialize = "altgr")]
    RAlt = 100,
    #[strum(serialize = "shift", serialize = "lshift")]
    LShift = 42,
    RShift = 54,
    #[strum(serialize = "super", serialize = "lsuper")]
    LSuper = 125,
    RSuper = 126,
    F1 = 59,
    F2 = 60,
    F3 = 61,
    F4 = 62,
    F5 = 63,
    F6 = 64,
    F7 = 65,
    F8 = 66,
    F9 = 67,
    F10 = 68,
    F11 = 87,
    F12 = 88,
    NumLock = 69,
    ScrollLock = 70,
    CapsLock = 58,
    Insert = 110,
    Delete = 111,
    Home = 102,
    End = 107,
    PageUp = 104,
    PageDown = 109,
    PrintScreen = 99,
    Pause = 119,
    Menu = 127,
    Space = 57,
    Tab = 15,
    Backspace = 14,
    Enter = 28,
    ArrowUp = 103,
    ArrowDown = 108,
    ArrowLeft = 105,
    ArrowRight = 106,
    // Media keys
    #[strum(serialize = "volumeup", serialize = "audiovolumeup")]
    VolumeUp = 115,
    #[strum(serialize = "volumedown", serialize = "audiovolumedown")]
    VolumeDown = 114,
    #[strum(serialize = "volumemute", serialize = "audiovolumemute")]
    VolumeMute = 113,
    #[strum(serialize = "micmute", serialize = "audiomicmute")]
    MicMute = 248,
    #[strum(serialize = "mediaplaypause", serialize = "playpause")]
    MediaPlayPause = 164,
    MediaStop = 166,
    #[strum(serialize = "medianext", serialize = "nexttrack")]
    MediaNext = 163,
    #[strum(
        serialize = "mediaprev",
        serialize = "prevtrack",
        serialize = "mediatrackprevious"
    )]
    MediaPrev = 165,
    // Brightness keys
    #[strum(serialize = "brightnessup", serialize = "monbrightnessup")]
    BrightnessUp = 225,
    #[strum(serialize = "brightnessdown", serialize = "monbrightnessdown")]
    BrightnessDown = 224,
    // Browser keys
    BrowserBack = 158,
    BrowserForward = 159,
    BrowserRefresh = 173,
    BrowserHome = 172,
    BrowserSearch = 217,
    BrowserFavorites = 156,
    // Application keys
    LaunchMail = 155,
    LaunchCalculator = 140,
    LaunchExplorer = 150,
    // System keys
    #[strum(serialize = "sleep", serialize = "standby")]
    Sleep = 142,
    Eject = 161,
}

fn key_name_to_evdev(name: &str) -> Result<i32, String> {
    EvdevKeys::from_str(name)
        .map(|k| k as i32)
        .map_err(|e| format!("Unknown key '{}': {}", name, e))
}

/// Maps ASCII characters to evdev keycodes (US keyboard layout).
/// Returns (evdev_keycode, needs_shift).
fn evdev_for_char(ch: char) -> Result<(i32, bool), String> {
    match ch {
        'a' => Ok((30, false)),
        'b' => Ok((48, false)),
        'c' => Ok((46, false)),
        'd' => Ok((32, false)),
        'e' => Ok((18, false)),
        'f' => Ok((33, false)),
        'g' => Ok((34, false)),
        'h' => Ok((35, false)),
        'i' => Ok((23, false)),
        'j' => Ok((36, false)),
        'k' => Ok((37, false)),
        'l' => Ok((38, false)),
        'm' => Ok((50, false)),
        'n' => Ok((49, false)),
        'o' => Ok((24, false)),
        'p' => Ok((25, false)),
        'q' => Ok((16, false)),
        'r' => Ok((19, false)),
        's' => Ok((31, false)),
        't' => Ok((20, false)),
        'u' => Ok((22, false)),
        'v' => Ok((47, false)),
        'w' => Ok((17, false)),
        'x' => Ok((45, false)),
        'y' => Ok((21, false)),
        'z' => Ok((44, false)),
        'A'..='Z' => {
            let (code, _) = evdev_for_char(ch.to_ascii_lowercase())?;
            Ok((code, true))
        }
        '1' => Ok((2, false)),
        '2' => Ok((3, false)),
        '3' => Ok((4, false)),
        '4' => Ok((5, false)),
        '5' => Ok((6, false)),
        '6' => Ok((7, false)),
        '7' => Ok((8, false)),
        '8' => Ok((9, false)),
        '9' => Ok((10, false)),
        '0' => Ok((11, false)),
        ' ' => Ok((57, false)),
        // Shifted symbols
        '!' => Ok((2, true)),
        '@' => Ok((3, true)),
        '#' => Ok((4, true)),
        '$' => Ok((5, true)),
        '%' => Ok((6, true)),
        '^' => Ok((7, true)),
        '&' => Ok((8, true)),
        '*' => Ok((9, true)),
        '(' => Ok((10, true)),
        ')' => Ok((11, true)),
        '_' => Ok((12, true)),
        '+' => Ok((13, true)),
        '{' => Ok((26, true)),
        '}' => Ok((27, true)),
        ':' => Ok((39, true)),
        '"' => Ok((40, true)),
        '<' => Ok((51, true)),
        '>' => Ok((52, true)),
        '?' => Ok((53, true)),
        '~' => Ok((41, true)),
        '|' => Ok((43, true)),
        // Unshifted symbols
        '-' => Ok((12, false)),
        '=' => Ok((13, false)),
        '[' => Ok((26, false)),
        ']' => Ok((27, false)),
        ';' => Ok((39, false)),
        '\'' => Ok((40, false)),
        ',' => Ok((51, false)),
        '.' => Ok((52, false)),
        '/' => Ok((53, false)),
        '\\' => Ok((43, false)),
        '`' => Ok((41, false)),
        _ => Err(format!("Unsupported character: {}", ch)),
    }
}

