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

// Maps ASCII characters to keysyms and returns whether Shift is needed
fn keysym_for_char(ch: char) -> Result<(Keysym, bool), String> {
    match ch {
        'a'..='z' => Ok((ch as u32 - 'a' as u32 + 0x61, false)),
        'A'..='Z' => Ok((ch as u32 - 'A' as u32 + 0x61, true)),  // Uppercase: map to lowercase keysym + Shift
        '0'..='9' => Ok((ch as u32 - '0' as u32 + 0x30, false)),
        ' ' => Ok((0x20, false)),
        // Shifted symbols - these need Shift pressed
        '!' => Ok((0x31, true)),  // '1' -> '!'
        '@' => Ok((0x32, true)),  // '2' -> '@'
        '#' => Ok((0x33, true)),  // '3' -> '#'
        '$' => Ok((0x34, true)),  // '4' -> '$'
        '%' => Ok((0x35, true)),  // '5' -> '%'
        '^' => Ok((0x36, true)),  // '6' -> '^'
        '&' => Ok((0x37, true)),  // '7' -> '&'
        '*' => Ok((0x38, true)),  // '8' -> '*'
        '(' => Ok((0x39, true)),  // '9' -> '('
        ')' => Ok((0x30, true)),  // '0' -> ')'
        '_' => Ok((0x2D, true)),  // '-' -> '_'
        '+' => Ok((0x3D, true)),  // '=' -> '+'
        '{' => Ok((0x5B, true)),  // '[' -> '{'
        '}' => Ok((0x5D, true)),  // ']' -> '}'
        ':' => Ok((0x3B, true)),  // ';' -> ':'
        '"' => Ok((0x27, true)),  // '\'' -> '"'
        '<' => Ok((0x2C, true)),  // ',' -> '<'
        '>' => Ok((0x2E, true)),  // '.' -> '>'
        '?' => Ok((0x2F, true)),  // '/' -> '?'
        '~' => Ok((0x60, true)),  // '`' -> '~'
        '|' => Ok((0x5C, true)),  // '\' -> '|'
        // Unshifted symbols
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
        _ => Err(format!("Unsupported character: {}", ch).to_string()),
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
            let (keysym, _) = keysym_for_char(part.chars().next().unwrap())?;
            keysym
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

/// Processes escape sequences in a string and returns the actual character to send.
/// Supported escape sequences: \n (Enter), \t (Tab), \r (Enter), \\ (backslash), \e (Escape)
fn process_escape_sequences(text: &str) -> Vec<char> {
    let mut result = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next_ch) = chars.peek() {
                match next_ch {
                    'n' => {
                        chars.next(); // consume the 'n'
                        result.push('\n'); // newline -> Enter key
                    }
                    't' => {
                        chars.next(); // consume the 't'
                        result.push('\t'); // tab -> Tab key
                    }
                    'r' => {
                        chars.next(); // consume the 'r'
                        result.push('\r'); // carriage return -> Enter key
                    }
                    '\\' => {
                        chars.next(); // consume the second backslash
                        result.push('\\'); // literal backslash
                    }
                    'e' => {
                        chars.next(); // consume the 'e'
                        result.push('\x1b'); // escape character
                    }
                    _ => {
                        // Unknown escape sequence, keep the backslash
                        result.push(ch);
                    }
                }
            } else {
                // Backslash at end of string
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Maps special control characters to their corresponding keysyms
fn keysym_for_control_char(ch: char) -> Option<Keysym> {
    match ch {
        '\n' | '\r' => Some(0xff0d), // XK_Return (Enter key)
        '\t' => Some(0xff09),         // XK_Tab
        '\x1b' => Some(0xff1b),       // XK_Escape
        _ => None,
    }
}

/// Sends a string of ASCII characters as individual keystrokes.
/// Each character in the string is converted to its corresponding keysym and sent as a key press/release event.
/// Supports escape sequences: \n, \t, \r, \\, \e
/// Properly handles shifted characters (uppercase letters and symbols that require Shift)
pub fn send_string(text: &str) -> Result<(), String> {
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

    // Get the Shift key's keycode
    let shift_keysym = Keys::LShift as u32;
    let shift_keycode = keysym_to_keycode(shift_keysym, &keysym_mapping, min_keycode)?;

    // Process escape sequences
    let processed_chars = process_escape_sequences(text);

    // Send each character as a keystroke
    for ch in processed_chars {
        // Check if it's a control character first
        if let Some(control_keysym) = keysym_for_control_char(ch) {
            // Control characters don't need Shift
            let keycode = keysym_to_keycode(control_keysym, &keysym_mapping, min_keycode)?;
            send_key_event(&keycode, &conn, KEY_PRESS)?;
            thread::sleep(Duration::from_millis(10));
            send_key_event(&keycode, &conn, KEY_RELEASE)?;
            thread::sleep(Duration::from_millis(10));
        } else {
            // Regular characters - check if Shift is needed
            let (keysym, needs_shift) = keysym_for_char(ch)?;
            let keycode = keysym_to_keycode(keysym, &keysym_mapping, min_keycode)?;

            if needs_shift {
                // Press Shift first
                send_key_event(&shift_keycode, &conn, KEY_PRESS)?;
                thread::sleep(Duration::from_millis(10));
            }

            // Press and release the key
            send_key_event(&keycode, &conn, KEY_PRESS)?;
            thread::sleep(Duration::from_millis(10));
            send_key_event(&keycode, &conn, KEY_RELEASE)?;
            thread::sleep(Duration::from_millis(10));

            if needs_shift {
                // Release Shift
                send_key_event(&shift_keycode, &conn, KEY_RELEASE)?;
                thread::sleep(Duration::from_millis(10));
            }
        }
    }

    Ok(())
}
