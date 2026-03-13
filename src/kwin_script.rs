// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! KWin Scripting API for Wayland
//!
//! This module provides a Rust interface to KWin's JavaScript scripting API via D-Bus.
//! It allows querying active window information and activating windows without
//! requiring external tools like kdotool.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver as MpscReceiver, Sender as MpscSender};
use std::sync::{Arc, LazyLock, RwLock};
use tokio::runtime::Runtime;

// Global channels for routing D-Bus callbacks to the correct client
static ACTIVATE_CHANNELS: LazyLock<Arc<RwLock<HashMap<String, MpscSender<String>>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));
static LISTENER_CHANNELS: LazyLock<Arc<RwLock<HashMap<String, MpscSender<WindowInfo>>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Information about a window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub title: String,
    pub class: String,
}

/// Errors that can occur during KWin scripting operations
#[derive(Debug)]
pub enum Error {
    DBusError(String),
    ScriptError(String),
    IOError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DBusError(e) => write!(f, "D-Bus error: {}", e),
            Error::ScriptError(e) => write!(f, "Script error: {}", e),
            Error::IOError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

// Fixed script names for single-instance server
const LISTENER_SCRIPT_NAME: &str = "keydeck-focus-listener";
const LISTENER_METHOD_NAME: &str = "keydeck_windowActivated";

/// D-Bus callback handler for KWin script responses.
/// Registered on the ObjectServer at /org/keydeck to receive callDBus callbacks.
struct CallbackHandler;

#[zbus::interface(name = "org.keydeck.Callback")]
impl CallbackHandler {
    async fn activate_result(&self, id: &str, result: &str) {
        let mut channels = ACTIVATE_CHANNELS.write().unwrap();
        if let Some(sender) = channels.remove(id) {
            let _ = sender.send(result.to_string());
        }
    }

    async fn window_activated(&self, data: &str) {
        let parts: Vec<&str> = data.split('|').collect();
        if parts.len() >= 2 {
            let info = WindowInfo {
                title: parts[0].to_string(),
                class: parts[1].to_string(),
            };
            let channels = LISTENER_CHANNELS.read().unwrap();
            if let Some(sender) = channels.get(LISTENER_METHOD_NAME) {
                let _ = sender.send(info);
            }
        }
    }
}

/// KWin scripting client
///
/// Can be used for both persistent listeners and one-shot operations.
/// Multiple instances can coexist - only the instance that started the listener will clean it up.
pub struct KWinScriptClient {
    _runtime: Runtime, // keeps tokio runtime alive for zbus::blocking
    conn: zbus::blocking::Connection,
    temp_dir: PathBuf,
    dbus_addr: String,
    listener_script_id: Arc<RwLock<Option<i32>>>,
    owns_listener: bool,
}

impl KWinScriptClient {
    /// Create a new KWin scripting client
    pub fn new() -> Result<Self, Error> {
        // zbus::blocking requires a tokio runtime context on the current thread
        let runtime = Runtime::new()
            .map_err(|e| Error::DBusError(format!("Failed to create tokio runtime: {}", e)))?;
        let _guard = runtime.enter();

        let conn = zbus::blocking::Connection::session()
            .map_err(|e| Error::DBusError(format!("Failed to connect to session bus: {}", e)))?;

        let dbus_addr = conn
            .unique_name()
            .ok_or_else(|| Error::DBusError("No unique bus name".to_string()))?
            .to_string();

        // Register callback handler for incoming KWin script responses
        conn.object_server()
            .at("/org/keydeck", CallbackHandler)
            .map_err(|e| {
                Error::DBusError(format!("Failed to register callback handler: {}", e))
            })?;

        Ok(Self {
            _runtime: runtime,
            conn,
            temp_dir: std::env::temp_dir(),
            dbus_addr,
            listener_script_id: Arc::new(RwLock::new(None)),
            owns_listener: false,
        })
    }

    /// Clean up any stale scripts from previous runs
    fn cleanup_stale_scripts(&self) {
        Self::cleanup_stale_scripts_static();
    }

    /// Static cleanup method that can be called without an instance
    pub fn cleanup_stale_scripts_static() {
        let rt = Runtime::new().ok();
        let _guard = rt.as_ref().map(|r| r.enter());
        if let Ok(conn) = zbus::blocking::Connection::session() {
            let _ = conn.call_method(
                Some("org.kde.KWin"),
                "/Scripting",
                Some("org.kde.kwin.Scripting"),
                "unloadScript",
                &(LISTENER_SCRIPT_NAME,),
            );
        }

        let temp_dir = std::env::temp_dir();
        let listener_file = temp_dir.join(format!("{}.js", LISTENER_SCRIPT_NAME));
        let _ = fs::remove_file(&listener_file);
    }

    /// Call a KWin D-Bus method and deserialize the response
    fn kwin_call<B: serde::Serialize + zbus::zvariant::DynamicType>(
        &self,
        path: &str,
        interface: &str,
        method: &str,
        body: &B,
    ) -> Result<zbus::Message, Error> {
        self.conn
            .call_method(
                Some("org.kde.KWin"),
                path,
                Some(interface),
                method,
                body,
            )
            .map_err(|e| Error::DBusError(format!("Failed to call {}.{}: {}", interface, method, e)))
    }

    /// Activate a window matching the given class and title
    pub fn activate_window(&self, class: &str, title: &str) -> Result<(), Error> {
        let script_name = format!("focus2-activate-{}", uuid::Uuid::new_v4());
        let method_uuid = uuid::Uuid::new_v4().to_string().replace("-", "");

        let script = format!(
            r#"
                var targetClass = "{}";
                var targetTitle = "{}";
                var clients = workspace.windowList();
                var found = false;
                var useOrLogic = targetClass !== "" && targetTitle !== "" && targetClass === targetTitle;

                for (var i = 0; i < clients.length; i++) {{
                    var client = clients[i];
                    var classMatch = targetClass === "" ? !useOrLogic :
                                   client.resourceClass.toLowerCase().indexOf(targetClass.toLowerCase()) >= 0;
                    var titleMatch = targetTitle === "" ? !useOrLogic :
                                   client.caption.toLowerCase().indexOf(targetTitle.toLowerCase()) >= 0;

                    var matches = useOrLogic ? (classMatch || titleMatch) : (classMatch && titleMatch);

                    if (matches) {{
                        workspace.activeWindow = client;
                        found = true;
                        break;
                    }}
                }}

                if (found) {{
                    callDBus("{}",
                            "/org/keydeck",
                            "org.keydeck.Callback",
                            "ActivateResult",
                            "{}",
                            "activated");
                }} else {{
                    callDBus("{}",
                            "/org/keydeck",
                            "org.keydeck.Callback",
                            "ActivateResult",
                            "{}",
                            "not_found");
                }}
            "#,
            class, title, self.dbus_addr, method_uuid, self.dbus_addr, method_uuid
        );

        // Write script to temp file
        let script_file_path = self.temp_dir.join(format!("{}.js", script_name));
        fs::write(&script_file_path, script)
            .map_err(|e| Error::IOError(format!("Failed to write script file: {}", e)))?;

        let script_path_str = script_file_path.to_str().ok_or_else(|| {
            Error::DBusError(format!(
                "Script path contains invalid UTF-8: {:?}",
                script_file_path
            ))
        })?;

        // Load the script
        let reply = self.kwin_call(
            "/Scripting",
            "org.kde.kwin.Scripting",
            "loadScript",
            &(script_path_str, script_name.as_str()),
        )?;
        let script_id: i32 = reply
            .body()
            .deserialize()
            .map_err(|e| Error::DBusError(format!("Failed to read script ID: {}", e)))?;

        let script_path = format!("/Scripting/Script{}", script_id);

        // Create channel to receive result
        let (sender, receiver) = mpsc::channel();
        {
            let mut channels = ACTIVATE_CHANNELS.write().unwrap();
            channels.insert(method_uuid.clone(), sender);
        }

        // Run the script
        if let Err(e) = self.kwin_call(&script_path, "org.kde.kwin.Script", "run", &()) {
            // Clean up channel
            {
                let mut channels = ACTIVATE_CHANNELS.write().unwrap();
                channels.remove(&method_uuid);
            }
            // Clean up script
            let _ = self.kwin_call(
                "/Scripting",
                "org.kde.kwin.Scripting",
                "unloadScript",
                &(script_name.as_str(),),
            );
            let _ = fs::remove_file(&script_file_path);
            return Err(Error::ScriptError(format!("Failed to run script: {}", e)));
        }

        // Wait for result with timeout
        let timeout = std::time::Duration::from_millis(500);
        let final_result = receiver.recv_timeout(timeout).ok();

        // Clean up channel
        {
            let mut channels = ACTIVATE_CHANNELS.write().unwrap();
            channels.remove(&method_uuid);
        }

        // Stop and unload script
        let _ = self.kwin_call(&script_path, "org.kde.kwin.Script", "stop", &());
        let _ = self.kwin_call(
            "/Scripting",
            "org.kde.kwin.Scripting",
            "unloadScript",
            &(script_name.as_str(),),
        );
        let _ = fs::remove_file(&script_file_path);

        match final_result.as_deref() {
            Some("activated") => Ok(()),
            Some("not_found") => Err(Error::ScriptError("Window not found".to_string())),
            _ => Err(Error::ScriptError(
                "No response from activation script".to_string(),
            )),
        }
    }

    /// Start listening for window activation events
    pub fn start_focus_listener(&mut self) -> Result<MpscReceiver<WindowInfo>, Error> {
        self.cleanup_stale_scripts();

        // Check if listener is already running
        {
            let listener_id = self.listener_script_id.read().unwrap();
            if listener_id.is_some() {
                drop(listener_id);
                self.stop_focus_listener()?;
            }
        }

        // Create channel for events
        let (sender, receiver) = mpsc::channel();
        {
            let mut channels = LISTENER_CHANNELS.write().unwrap();
            channels.insert(LISTENER_METHOD_NAME.to_string(), sender);
        }

        // Create persistent listener script
        let script = format!(
            r#"
                var currentClient = null;
                var captionConnection = null;
                var windowActivatedConnection = null;

                function sendWindowInfo(client) {{
                    if (client) {{
                        callDBus("{}",
                                "/org/keydeck",
                                "org.keydeck.Callback",
                                "WindowActivated",
                                client.caption + "|" + client.resourceClass);
                    }}
                }}

                function setupClient(client) {{
                    if (captionConnection) {{
                        captionConnection.disconnect();
                        captionConnection = null;
                    }}

                    currentClient = client;
                    sendWindowInfo(client);

                    if (client && client.captionChanged) {{
                        captionConnection = client.captionChanged.connect(function() {{
                            if (currentClient === workspace.activeWindow) {{
                                sendWindowInfo(currentClient);
                            }}
                        }});
                    }}
                }}

                function cleanup() {{
                    if (captionConnection) {{
                        captionConnection.disconnect();
                        captionConnection = null;
                    }}
                    if (windowActivatedConnection) {{
                        windowActivatedConnection.disconnect();
                        windowActivatedConnection = null;
                    }}
                }}

                setupClient(workspace.activeWindow);
                windowActivatedConnection = workspace.windowActivated.connect(setupClient);
            "#,
            self.dbus_addr
        );

        let script_file_path = self.temp_dir.join(format!("{}.js", LISTENER_SCRIPT_NAME));
        fs::write(&script_file_path, script)
            .map_err(|e| Error::IOError(format!("Failed to write listener script: {}", e)))?;

        let script_path_str = script_file_path.to_str().ok_or_else(|| {
            Error::DBusError(format!(
                "Listener script path contains invalid UTF-8: {:?}",
                script_file_path
            ))
        })?;

        // Load the script
        let reply = self.kwin_call(
            "/Scripting",
            "org.kde.kwin.Scripting",
            "loadScript",
            &(script_path_str, LISTENER_SCRIPT_NAME),
        )?;
        let script_id: i32 = reply
            .body()
            .deserialize()
            .map_err(|e| Error::DBusError(format!("Failed to read script ID: {}", e)))?;

        {
            let mut listener_id = self.listener_script_id.write().unwrap();
            *listener_id = Some(script_id);
        }

        let script_path = format!("/Scripting/Script{}", script_id);

        // Run the script
        if let Err(e) = self.kwin_call(&script_path, "org.kde.kwin.Script", "run", &()) {
            {
                let mut channels = LISTENER_CHANNELS.write().unwrap();
                channels.remove(LISTENER_METHOD_NAME);
            }
            let _ = self.kwin_call(
                "/Scripting",
                "org.kde.kwin.Scripting",
                "unloadScript",
                &(LISTENER_SCRIPT_NAME,),
            );
            let _ = fs::remove_file(&script_file_path);
            {
                let mut listener_id = self.listener_script_id.write().unwrap();
                *listener_id = None;
            }
            return Err(Error::ScriptError(format!(
                "Failed to run listener script: {}",
                e
            )));
        }

        let _ = fs::remove_file(&script_file_path);
        self.owns_listener = true;

        Ok(receiver)
    }

    /// Stop the focus listener
    pub fn stop_focus_listener(&self) -> Result<(), Error> {
        let script_id = {
            let mut listener_id = self.listener_script_id.write().unwrap();
            listener_id.take()
        };

        {
            let mut channels = LISTENER_CHANNELS.write().unwrap();
            channels.remove(LISTENER_METHOD_NAME);
        }

        if let Some(script_id) = script_id {
            let script_path = format!("/Scripting/Script{}", script_id);
            let _ = self.kwin_call(&script_path, "org.kde.kwin.Script", "stop", &());
            let _ = self.kwin_call(
                "/Scripting",
                "org.kde.kwin.Scripting",
                "unloadScript",
                &(LISTENER_SCRIPT_NAME,),
            );
        }

        Ok(())
    }
}

impl Drop for KWinScriptClient {
    fn drop(&mut self) {
        if self.owns_listener {
            let _ = self.stop_focus_listener();
        }

        // Remove callback handler from object server
        let _ = self
            .conn
            .object_server()
            .remove::<CallbackHandler, _>("/org/keydeck");

        // Clear remaining channel entries
        if self.owns_listener {
            let mut listener_channels = LISTENER_CHANNELS.write().unwrap();
            listener_channels.remove(LISTENER_METHOD_NAME);
        }
    }
}
