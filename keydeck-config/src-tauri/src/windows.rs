// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

#[cfg(target_os = "linux")]
use std::collections::HashSet;
#[cfg(target_os = "linux")]
use std::sync::mpsc;
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
use dbus::blocking::Connection as DBusConnection;
#[cfg(target_os = "linux")]
use dbus::channel::MatchingReceiver;
#[cfg(target_os = "linux")]
use dbus::message::MatchRule;
#[cfg(target_os = "linux")]
use uuid::Uuid;
#[cfg(target_os = "linux")]
use x11rb::connection::Connection;
#[cfg(target_os = "linux")]
use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt, Window};
#[cfg(target_os = "linux")]
use x11rb::rust_connection::RustConnection;

/// Collect unique window classes from the active X11 session.
#[cfg(target_os = "linux")]
pub fn list_window_classes() -> Result<Vec<String>, String> {
    let session_type = std::env::var("XDG_SESSION_TYPE")
        .unwrap_or_default()
        .to_lowercase();

    if session_type == "wayland" {
        return list_window_classes_wayland();
    }

    list_window_classes_x11()
}

/// Stub for unsupported platforms.
#[cfg(not(target_os = "linux"))]
pub fn list_window_classes() -> Result<Vec<String>, String> {
    Err("Window class enumeration is only supported on Linux.".to_string())
}

#[cfg(target_os = "linux")]
fn list_window_classes_x11() -> Result<Vec<String>, String> {
    let (conn, screen_num) = RustConnection::connect(None)
        .map_err(|e| format!("Failed to connect to X server: {}", e))?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let net_client_list_atom = intern_atom(&conn, b"_NET_CLIENT_LIST")?;
    let wm_class_atom = AtomEnum::WM_CLASS.into();

    let client_list = conn
        .get_property::<Atom, Atom>(
            false,
            root,
            net_client_list_atom,
            AtomEnum::WINDOW.into(),
            0,
            u32::MAX,
        )
        .map_err(|e| format!("Failed to query _NET_CLIENT_LIST: {}", e))?
        .reply()
        .map_err(|e| format!("Failed to read _NET_CLIENT_LIST reply: {}", e))?;

    if client_list.format != 32 {
        return Err("Unexpected format for _NET_CLIENT_LIST".into());
    }

    let mut seen = HashSet::new();
    let mut classes = Vec::new();

    if let Some(ids) = client_list.value32() {
        for window in ids {
            if let Some((res_name, res_class)) = get_wm_class(&conn, window, wm_class_atom)? {
                let class_name = compose_class_string(&res_name, &res_class);
                if !class_name.is_empty() && seen.insert(class_name.clone()) {
                    classes.push(class_name);
                }
            }
        }
    }

    Ok(classes)
}

#[cfg(target_os = "linux")]
fn list_window_classes_wayland() -> Result<Vec<String>, String> {
    // Assume KDE Plasma / KWin availability
    let conn = DBusConnection::new_session()
        .map_err(|e| format!("Failed to connect to D-Bus session bus: {}", e))?;

    let script_name = format!("keydeck-window-list-{}", Uuid::new_v4());
    let method_name = format!(
        "keydeck_window_list_{}",
        Uuid::new_v4().to_string().replace('-', "")
    );

    // We'll listen for the script response on a dedicated D-Bus connection.
    let mut match_conn = DBusConnection::new_session()
        .map_err(|e| format!("Failed to create D-Bus match connection: {}", e))?;
    let match_unique = match_conn.unique_name().to_string();

    let script = format!(
        r#"
            var data = [];
            var clients = workspace.windowList();
            for (var i = 0; i < clients.length; i++) {{
                var client = clients[i];
                data.push(client.resourceClass || "");
            }}
            callDBus("{}", "/", "", "{}", JSON.stringify(data));
        "#,
        match_unique, method_name
    );

    let script_path = std::env::temp_dir().join(format!("{}.js", script_name));
    std::fs::write(&script_path, script)
        .map_err(|e| format!("Failed to write temporary script: {}", e))?;

    let kwin_proxy = conn.with_proxy("org.kde.KWin", "/Scripting", Duration::from_secs(5));
    let script_path_str = script_path
        .to_str()
        .ok_or_else(|| "Temporary script path contains invalid UTF-8".to_string())?;
    let (script_id,): (i32,) = kwin_proxy
        .method_call(
            "org.kde.kwin.Scripting",
            "loadScript",
            (script_path_str, script_name.as_str()),
        )
        .map_err(|e| format!("Failed to load KWin script: {}", e))?;

    let script_object_path = format!("/Scripting/Script{}", script_id);
    let script_proxy = conn.with_proxy("org.kde.KWin", &script_object_path, Duration::from_secs(5));
    let (tx, rx) = mpsc::channel();
    let mut rule = MatchRule::new_method_call();
    rule.member = Some(method_name.clone().into());
    let token = match_conn.start_receive(
        rule,
        Box::new(move |message, _| {
            if let Some(arg) = message.get1::<&str>() {
                let _ = tx.send(arg.to_string());
                return true;
            }
            true
        }),
    );

    script_proxy
        .method_call::<(), _, _, _>("org.kde.kwin.Script", "run", ())
        .map_err(|e| format!("Failed to run KWin script: {}", e))?;

    let payload = wait_for_payload(&mut match_conn, &rx)?;

    // Cleanup
    let _ = script_proxy.method_call::<(), _, _, _>("org.kde.kwin.Script", "stop", ());
    let _ = kwin_proxy.method_call::<(), _, _, _>(
        "org.kde.kwin.Scripting",
        "unloadScript",
        (script_name.as_str(),),
    );
    let _ = std::fs::remove_file(&script_path);
    match_conn.stop_receive(token);

    let mut seen = HashSet::new();
    let mut classes = Vec::new();

    let parsed: Vec<String> = serde_json::from_str(&payload)
        .map_err(|e| format!("Failed to parse window list: {}", e))?;
    for entry in parsed {
        let trimmed = entry.trim();
        if !trimmed.is_empty() && seen.insert(trimmed.to_string()) {
            classes.push(trimmed.to_string());
        }
    }

    Ok(classes)
}

#[cfg(target_os = "linux")]
fn wait_for_payload(
    conn: &mut DBusConnection,
    rx: &mpsc::Receiver<String>,
) -> Result<String, String> {
    let timeout = Duration::from_secs(2);
    let start = Instant::now();

    loop {
        if let Ok(payload) = rx.try_recv() {
            return Ok(payload);
        }

        if start.elapsed() >= timeout {
            return Err("Timed out waiting for KWin response".into());
        }

        let _ = conn.process(Duration::from_millis(50));
    }
}

#[cfg(target_os = "linux")]
fn compose_class_string(res_name: &str, res_class: &str) -> String {
    match (res_name.is_empty(), res_class.is_empty()) {
        (true, true) => String::new(),
        (true, false) => res_class.to_string(),
        (false, true) => res_name.to_string(),
        (false, false) => format!("{}.{}", res_name, res_class),
    }
}

#[cfg(target_os = "linux")]
fn intern_atom(conn: &RustConnection, name: &[u8]) -> Result<Atom, String> {
    conn.intern_atom(false, name)
        .map_err(|e| format!("Failed to intern atom {:?}: {}", name, e))?
        .reply()
        .map_err(|e| format!("Failed to fetch atom {:?}: {}", name, e))
        .map(|r| r.atom)
}

#[cfg(target_os = "linux")]
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
        .map_err(|e| format!("Failed to get WM_CLASS: {}", e))?
        .reply()
        .map_err(|e| format!("Failed to read WM_CLASS reply: {}", e))?;

    if reply.value_len == 0 {
        return Ok(None);
    }

    let value = reply.value;
    let parts = value.split(|&b| b == 0).collect::<Vec<_>>();
    if parts.len() < 2 {
        return Ok(None);
    }

    let res_name = String::from_utf8(parts[0].to_vec())
        .map_err(|e| format!("Failed to parse resource name: {}", e))?;
    let res_class = String::from_utf8(parts[1].to_vec())
        .map_err(|e| format!("Failed to parse resource class: {}", e))?;

    Ok(Some((res_name, res_class)))
}
