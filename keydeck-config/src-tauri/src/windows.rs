// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
use std::collections::HashSet;
#[cfg(target_os = "linux")]
use std::sync::mpsc;
#[cfg(target_os = "linux")]
use std::time::Duration;

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

/// Enumerate the process names of visible top-level windows.
///
/// Mirrors the daemon's Windows focus matching, which uses the owning
/// process's executable base name (without `.exe`) as the window "class".
#[cfg(target_os = "windows")]
pub fn list_window_classes() -> Result<Vec<String>, String> {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, IsWindowVisible,
    };

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: lparam carries a &mut Vec<String> supplied by the caller below.
        let names = unsafe { &mut *(lparam.0 as *mut Vec<String>) };
        unsafe {
            // Only visible, titled windows — skips invisible helper/tool windows.
            if !IsWindowVisible(hwnd).as_bool() || GetWindowTextLengthW(hwnd) == 0 {
                return TRUE;
            }
            if let Some(name) = window_process_name(hwnd) {
                if !name.is_empty() {
                    names.push(name);
                }
            }
        }
        TRUE
    }

    let mut names: Vec<String> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(enum_cb), LPARAM(&mut names as *mut _ as isize));
    }

    let mut seen = HashSet::new();
    let mut classes = Vec::new();
    for name in names {
        if seen.insert(name.clone()) {
            classes.push(name);
        }
    }
    classes.sort_by_key(|s| s.to_lowercase());
    Ok(classes)
}

/// Returns the executable base name (without `.exe`) of the process owning a
/// window — the Windows analogue of X11's `WM_CLASS`.
#[cfg(target_os = "windows")]
fn window_process_name(hwnd: windows::Win32::Foundation::HWND) -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u16; 512];
        let mut size = buf.len() as u32;
        let result =
            QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut size);
        let _ = CloseHandle(handle);
        result.ok()?;

        let path = String::from_utf16_lossy(&buf[..size as usize]);
        let name = path.rsplit(['\\', '/']).next().unwrap_or(&path);
        let name = name
            .strip_suffix(".exe")
            .or_else(|| name.strip_suffix(".EXE"))
            .unwrap_or(name);
        Some(name.to_string())
    }
}

/// Enumerate the localized names of running applications with a normal UI
/// presence.
///
/// Mirrors the daemon's macOS focus matching, which matches against the
/// frontmost application's localized name (or bundle identifier).
#[cfg(target_os = "macos")]
pub fn list_window_classes() -> Result<Vec<String>, String> {
    use objc2_app_kit::{NSApplicationActivationPolicy, NSWorkspace};

    let mut seen = HashSet::new();
    let mut classes = Vec::new();

    unsafe {
        let workspace = NSWorkspace::sharedWorkspace();
        let apps = workspace.runningApplications();
        for i in 0..apps.count() {
            let app = apps.objectAtIndex(i);
            // Only "regular" apps have a Dock icon / normal windows; skip
            // background agents and UI-element-only processes.
            if app.activationPolicy() != NSApplicationActivationPolicy::Regular {
                continue;
            }
            if let Some(name) = app.localizedName() {
                let name = name.to_string();
                if !name.is_empty() && seen.insert(name.clone()) {
                    classes.push(name);
                }
            }
        }
    }

    classes.sort_by_key(|s| s.to_lowercase());
    Ok(classes)
}

/// Stub for any other platform.
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub fn list_window_classes() -> Result<Vec<String>, String> {
    Err("Window class enumeration is not supported on this platform.".to_string())
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
    use std::sync::Arc;

    // D-Bus callback handler to receive KWin script results
    struct WindowListHandler {
        tx: mpsc::Sender<String>,
        method_name: Arc<String>,
    }

    #[zbus::interface(name = "onl.ycode.keydeck.WindowList")]
    impl WindowListHandler {
        async fn window_list_result(
            &self,
            #[zbus(header)] header: zbus::message::Header<'_>,
            data: &str,
        ) {
            if let Some(member) = header.member() {
                if member.as_str() == self.method_name.as_str() {
                    let _ = self.tx.send(data.to_string());
                }
            }
        }
    }

    // zbus::blocking requires a tokio runtime context
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;
    let _guard = rt.enter();

    let conn = zbus::blocking::Connection::session()
        .map_err(|e| format!("Failed to connect to D-Bus session bus: {}", e))?;

    let dbus_addr = conn
        .unique_name()
        .ok_or_else(|| "No unique bus name".to_string())?
        .to_string();

    let script_name = format!("keydeck-window-list-{}", Uuid::new_v4());
    let method_uuid = Uuid::new_v4().to_string().replace('-', "");

    // Create channel to receive result
    let (tx, rx) = mpsc::channel();
    let method_name = Arc::new(format!("keydeck_window_list_{}", method_uuid));

    // Register callback handler
    conn.object_server()
        .at(
            "/onl/ycode/keydeck/windowlist",
            WindowListHandler {
                tx,
                method_name: method_name.clone(),
            },
        )
        .map_err(|e| format!("Failed to register callback handler: {}", e))?;

    let script = format!(
        r#"
            var data = [];
            var clients = workspace.windowList();
            for (var i = 0; i < clients.length; i++) {{
                var client = clients[i];
                data.push(client.resourceClass || "");
            }}
            callDBus("{}",
                    "/onl/ycode/keydeck/windowlist",
                    "onl.ycode.keydeck.WindowList",
                    "WindowListResult",
                    JSON.stringify(data));
        "#,
        dbus_addr
    );

    let script_path = std::env::temp_dir().join(format!("{}.js", script_name));
    std::fs::write(&script_path, &script)
        .map_err(|e| format!("Failed to write temporary script: {}", e))?;

    let script_path_str = script_path
        .to_str()
        .ok_or_else(|| "Temporary script path contains invalid UTF-8".to_string())?;

    // Load the script
    let reply = conn
        .call_method(
            Some("org.kde.KWin"),
            "/Scripting",
            Some("org.kde.kwin.Scripting"),
            "loadScript",
            &(script_path_str, script_name.as_str()),
        )
        .map_err(|e| format!("Failed to load KWin script: {}", e))?;
    let script_id: i32 = reply
        .body()
        .deserialize()
        .map_err(|e| format!("Failed to read script ID: {}", e))?;

    let script_object_path = format!("/Scripting/Script{}", script_id);

    // Run the script
    conn.call_method(
        Some("org.kde.KWin"),
        script_object_path.as_str(),
        Some("org.kde.kwin.Script"),
        "run",
        &(),
    )
    .map_err(|e| format!("Failed to run KWin script: {}", e))?;

    // Wait for result with timeout
    let payload = rx
        .recv_timeout(Duration::from_secs(2))
        .map_err(|_| "Timed out waiting for KWin response".to_string())?;

    // Cleanup
    let _ = conn.call_method(
        Some("org.kde.KWin"),
        script_object_path.as_str(),
        Some("org.kde.kwin.Script"),
        "stop",
        &(),
    );
    let _ = conn.call_method(
        Some("org.kde.KWin"),
        "/Scripting",
        Some("org.kde.kwin.Scripting"),
        "unloadScript",
        &(script_name.as_str(),),
    );
    let _ = std::fs::remove_file(&script_path);
    let _ = conn
        .object_server()
        .remove::<WindowListHandler, _>("/onl/ycode/keydeck/windowlist");

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
