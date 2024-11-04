use std::str::FromStr;
use std::{thread, time::Duration};
use strum_macros::{Display, EnumString};
use x11rb::protocol::xproto::{GetKeyboardMappingReply, Keycode, Keysym};
use x11rb::{
    connection::Connection,
    protocol::{xproto::ConnectionExt, xtest},
    rust_connection::RustConnection,
    NONE,
};

const KEY_PRESS: u8 = 2;
const KEY_RELEASE: u8 = 3;

// Modifier keys as an enum with direct mapping to keysyms
#[derive(Debug, PartialEq, Eq)]
#[repr(u32)]
#[derive(EnumString, Display)]
#[strum(serialize_all = "lowercase")] // Converts to lowercase for matching
enum Keys {
    #[strum(serialize = "esc", serialize = "escape")]
    Esc = 0xff1b,                // XK_Escape
    #[strum(serialize = "ctrl", serialize = "lctrl", serialize = "control", serialize = "lcontrol")]
    LCtrl = 0xffe3,              // XK_Control_L
    #[strum(serialize = "rctrl", serialize = "rcontrol")]
    RCtrl = 0xffe4,              // XK_Control_R
    #[strum(serialize = "alt", serialize = "lalt")]
    LAlt = 0xffe9,               // XK_Alt_L
    RAlt = 0xffea,               // XK_Alt_R
    #[strum(serialize = "shift", serialize = "lshift")]
    LShift = 0xffe1,             // XK_Shift_L
    RShift = 0xffe2,             // XK_Shift_R
    #[strum(serialize = "super", serialize = "lsuper")]
    LSuper = 0xffeb,             // XK_Super_L
    RSuper = 0xffec,             // XK_Super_R
    AltGr = 0xfe03,              // XK_ISO_Level3_Shift
    F1 = 0xffbe,                 // XK_F1
    F2 = 0xffbf,                 // XK_F2
    F3 = 0xffc0,                 // XK_F3
    F4 = 0xffc1,                 // XK_F4
    F5 = 0xffc2,                 // XK_F5
    F6 = 0xffc3,                 // XK_F6
    F7 = 0xffc4,                 // XK_F7
    F8 = 0xffc5,                 // XK_F8
    F9 = 0xffc6,                 // XK_F9
    F10 = 0xffc7,                // XK_F10
    F11 = 0xffc8,                // XK_F11
    F12 = 0xffc9,                // XK_F12
    NumLock = 0xff7f,            // XK_Num_Lock
    ScrollLock = 0xff14,         // XK_Scroll_Lock
    CapsLock = 0xffe5,           // XK_Caps_Lock
    Insert = 0xff63,             // XK_Insert
    Delete = 0xffff,             // XK_Delete
    Home = 0xff50,               // XK_Home
    End = 0xff57,                // XK_End
    PageUp = 0xff55,             // XK_Page_Up
    PageDown = 0xff56,           // XK_Page_Down
    PrintScreen = 0xff61,        // XK_Print
    Pause = 0xff13,              // XK_Pause
    Menu = 0xff67,               // XK_Menu
    Space = 0x0020,              // XK_space
    Tab = 0xff09,                // XK_Tab
    Backspace = 0xff08,          // XK_BackSpace
    Enter = 0xff0d,              // XK_Return
    ArrowUp = 0xff52,            // XK_Up
    ArrowDown = 0xff54,          // XK_Down
    ArrowLeft = 0xff51,          // XK_Left
    ArrowRight = 0xff53,         // XK_Right
}

// Maps ASCII characters to keysyms
fn keysym_for_char(ch: char) -> Result<Keysym, String> {
    match ch {
        'a'..='z' => Ok(ch as u32 - 'a' as u32 + 0x61),
        'A'..='Z' => Ok(ch as u32 - 'A' as u32 + 0x41),
        '0'..='9' => Ok(ch as u32 - '0' as u32 + 0x30),
        ' ' => Ok(0x20),
        '!' => Ok(0x21),
        '@' => Ok(0x40),
        '#' => Ok(0x23),
        '$' => Ok(0x24),
        '%' => Ok(0x25),
        '^' => Ok(0x5E),
        '&' => Ok(0x26),
        '*' => Ok(0x2A),
        '(' => Ok(0x28),
        ')' => Ok(0x29),
        '-' => Ok(0x2D),
        '_' => Ok(0x5F),
        '=' => Ok(0x3D),
        '+' => Ok(0x2B),
        '[' => Ok(0x5B),
        ']' => Ok(0x5D),
        '{' => Ok(0x7B),
        '}' => Ok(0x7D),
        ';' => Ok(0x3B),
        ':' => Ok(0x3A),
        '\'' => Ok(0x27),
        '"' => Ok(0x22),
        ',' => Ok(0x2C),
        '.' => Ok(0x2E),
        '/' => Ok(0x2F),
        '\\' => Ok(0x5C),
        '?' => Ok(0x3F),
        '`' => Ok(0x60),
        '~' => Ok(0x7E),
        '|' => Ok(0x7C),
        '<' => Ok(0x3C),
        '>' => Ok(0x3E),
        _ => Err(format!("Unsupported character: {}", ch).to_string()), // Return an error for unsupported characters
    }
}

/// Converts a Keysym to a Keycode by scanning the keyboard mapping.
fn keysym_to_keycode(keysym: Keysym, keysym_mapping: &GetKeyboardMappingReply, min_keycode: Keycode) -> Result<u8, String> {
    // Scan through the Keysyms to find the corresponding Keycode
    for (i, keysym_list) in keysym_mapping.keysyms.chunks(keysym_mapping.keysyms_per_keycode as usize).enumerate() {
        for &mapped_keysym in keysym_list {
            if mapped_keysym == keysym {
                return Ok(min_keycode + i as u8); // Compute Keycode based on offset
            }
        }
    }
    Err(format!("Keysym not found: {}", keysym))
}

/// Sends a key event using the XTest extension.
fn send_key_event(keycode: &u8, conn: &RustConnection, event_type: u8) -> Result<(), String> {
    let device_id = 0;
    if let Err(e) = xtest::fake_input(conn, event_type, *keycode, x11rb::CURRENT_TIME, NONE, 0, 0, device_id) {
        return Err(format!("Error sending key event: {}", e));
    }
    if let Err(e) = conn.flush() {
        return Err(format!("Error flushing connection: {}", e));
    }
    Ok(())
}

/// Parses and presses a key combination string like "LCtrl+LShift+z"
fn press_key_combination(combo: &str, conn: &RustConnection, keyboard_mapping: &GetKeyboardMappingReply, min_keycode: Keycode) -> Result<(), String> {
    let parts: Vec<&str> = combo.split('+').collect();
    let mut keycodes: Vec<u8> = Vec::new();
    for part in parts {
        let keysym = if part.len() == 1 {
            keysym_for_char(part.chars().next().unwrap())?
        } else {
            Keys::from_str(&part.to_lowercase()).map_err(|e| e.to_string())? as u32
        };
        keycodes.push(keysym_to_keycode(keysym, keyboard_mapping, min_keycode)?);
    }

    // Press each modifier key in the specified order
    for keycode in &keycodes {
        send_key_event(keycode, conn, KEY_PRESS)?;
        thread::sleep(Duration::from_millis(10));
    }
    for keycode in keycodes.iter().rev() {
        send_key_event(keycode, conn, KEY_RELEASE)?;
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

pub fn send_key_combination(combination: &str) -> Result<(), String> {
    // Connect to the X11 server
    let (conn, _) = RustConnection::connect(None).map_err(|e| e.to_string())?;

    // Ensure that the XTest extension is available
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

    let keyboard_mapping = conn.get_keyboard_mapping(min_keycode, max_keycode - min_keycode + 1).map_err(|e| e.to_string())?;
    let keysym_mapping = keyboard_mapping.reply().map_err(|e| e.to_string())?;

    press_key_combination(combination, &conn, &keysym_mapping, min_keycode).unwrap_or_else(|e| eprintln!("{}", e));
    Ok(())
}
