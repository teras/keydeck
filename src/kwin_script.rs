//! KWin Scripting API for Wayland
//!
//! This module provides a Rust interface to KWin's JavaScript scripting API via D-Bus.
//! It allows querying active window information and activating windows without
//! requiring external tools like kdotool.

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender as MpscSender, Receiver as MpscReceiver};
use std::time::Duration;
use dbus::blocking::{Connection, SyncConnection};
use dbus::channel::{MatchingReceiver, Sender};
use dbus::Message;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

// Global channels for routing D-Bus callbacks to the correct client
// Each method name maps to a sender for that specific operation
lazy_static::lazy_static! {
    // For activate_window() results (String = "activated" or "not_found")
    static ref ACTIVATE_CHANNELS: Arc<RwLock<HashMap<String, MpscSender<String>>>> = Arc::new(RwLock::new(HashMap::new()));
    // For window event listeners (WindowInfo events)
    static ref LISTENER_CHANNELS: Arc<RwLock<HashMap<String, MpscSender<WindowInfo>>>> = Arc::new(RwLock::new(HashMap::new()));
}

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

/// KWin scripting client
///
/// Multiple instances can run concurrently - each manages its own scripts and channels.
pub struct KWinScriptClient {
    kwin_conn: Connection,
    temp_dir: PathBuf,
    dbus_addr: String,
    listener_script_id: Arc<RwLock<Option<i32>>>,
    listener_script_name: Arc<RwLock<Option<String>>>,
    listener_method_name: Arc<RwLock<Option<String>>>,
    stop_flag: Arc<AtomicBool>,
}

impl KWinScriptClient {
    /// Create a new KWin scripting client
    pub fn new() -> Result<Self, Error> {
        let kwin_conn = Connection::new_session()
            .map_err(|e| Error::DBusError(format!("Failed to connect to session bus: {}", e)))?;

        let self_conn = Arc::new(SyncConnection::new_session()
            .map_err(|e| Error::DBusError(format!("Failed to create sync connection: {}", e)))?);

        let dbus_addr = self_conn.unique_name().to_string();
        let temp_dir = std::env::temp_dir();

        // Set up message receiver
        let _receiver = self_conn.start_receive(
            dbus::message::MatchRule::new_method_call(),
            Box::new(|message: Message, connection: &SyncConnection| -> bool {
                if let Some(member) = message.member() {
                    if let Some(arg) = message.get1::<&str>() {
                        let member_str = member.to_string();

                        // Handle window activation events (from listeners)
                        if member_str.starts_with("windowActivated_") {
                            // Parse the window info and send through the channel
                            let parts: Vec<&str> = arg.split('|').collect();
                            if parts.len() >= 2 {
                                let window_info = WindowInfo {
                                    title: parts[0].to_string(),
                                    class: parts[1].to_string(),
                                };

                                // Send through the specific listener's channel
                                let channels = LISTENER_CHANNELS.read().unwrap();
                                if let Some(sender) = channels.get(&member_str) {
                                    let _ = sender.send(window_info);
                                }
                            }
                        }
                        // Handle activate_window() results
                        else if member_str.starts_with("activate_") {
                            // Send result through the channel and remove it
                            let mut channels = ACTIVATE_CHANNELS.write().unwrap();
                            if let Some(sender) = channels.remove(&member_str) {
                                let _ = sender.send(arg.to_string());
                            }
                        }

                        // Send empty reply to acknowledge
                        let reply = message.method_return();
                        let _ = connection.send(reply);
                    }
                }
                true
            }),
        );

        let stop_flag = Arc::new(AtomicBool::new(false));

        // Start background thread to process incoming messages
        let conn_clone = Arc::clone(&self_conn);
        let stop_flag_clone = Arc::clone(&stop_flag);
        std::thread::spawn(move || {
            while !stop_flag_clone.load(Ordering::Relaxed) {
                let _ = conn_clone.process(Duration::from_millis(100));
            }
        });

        Ok(Self {
            kwin_conn,
            temp_dir,
            dbus_addr,
            listener_script_id: Arc::new(RwLock::new(None)),
            listener_script_name: Arc::new(RwLock::new(None)),
            listener_method_name: Arc::new(RwLock::new(None)),
            stop_flag,
        })
    }

    /// Activate a window matching the given class and title
    ///
    /// This is a one-shot operation that doesn't require the event listener.
    /// Matching is case-insensitive and uses substring search.
    pub fn activate_window(&self, class: &str, title: &str) -> Result<(), Error> {
        // Generate unique script name
        let script_name = format!("focus2-activate-{}", uuid::Uuid::new_v4());
        let method_name = format!("activate_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));

        let script = format!(
            r#"
                var targetClass = "{}";
                var targetTitle = "{}";
                var clients = workspace.windowList();
                var found = false;

                for (var i = 0; i < clients.length; i++) {{
                    var client = clients[i];
                    var classMatch = targetClass === "" ||
                                   client.resourceClass.toLowerCase().indexOf(targetClass.toLowerCase()) >= 0;
                    var titleMatch = targetTitle === "" ||
                                   client.caption.toLowerCase().indexOf(targetTitle.toLowerCase()) >= 0;

                    if (classMatch && titleMatch) {{
                        workspace.activeWindow = client;
                        found = true;
                        break;
                    }}
                }}

                if (found) {{
                    callDBus("{}",
                            "/",
                            "",
                            "{}",
                            "activated");
                }} else {{
                    callDBus("{}",
                            "/",
                            "",
                            "{}",
                            "not_found");
                }}
            "#,
            class, title, self.dbus_addr, method_name, self.dbus_addr, method_name
        );

        // Write to file
        let script_file_path = self.temp_dir.join(format!("{}.js", script_name));
        fs::write(&script_file_path, script)
            .map_err(|e| Error::IOError(format!("Failed to write script file: {}", e)))?;

        // Load the script via D-Bus
        let kwin_proxy = self.kwin_conn.with_proxy(
            "org.kde.KWin",
            "/Scripting",
            Duration::from_millis(5000)
        );

        let (script_id,): (i32,) = kwin_proxy.method_call(
            "org.kde.kwin.Scripting",
            "loadScript",
            (script_file_path.to_str().unwrap(), script_name.as_str()),
        ).map_err(|e| Error::DBusError(format!("Failed to load script: {}", e)))?;

        let script_path = format!("/Scripting/Script{}", script_id);

        // Run the script
        let script_proxy = self.kwin_conn.with_proxy(
            "org.kde.KWin",
            &script_path,
            Duration::from_millis(5000)
        );

        // Create a channel to receive the result (event-driven, not polling!)
        let (sender, receiver) = mpsc::channel();

        // Store the sender so D-Bus handler can send us the result
        {
            let mut channels = ACTIVATE_CHANNELS.write().unwrap();
            channels.insert(method_name.clone(), sender);
        }

        // Run the script - if this fails, we need to unload it and clean up the channel
        let run_result: Result<(), dbus::Error> = script_proxy.method_call("org.kde.kwin.Script", "run", ());
        if let Err(e) = run_result {
            // Remove our channel
            {
                let mut channels = ACTIVATE_CHANNELS.write().unwrap();
                channels.remove(&method_name);
            }

            // Clean up the script before returning error
            let _: Result<(), _> = kwin_proxy.method_call(
                "org.kde.kwin.Scripting",
                "unloadScript",
                (script_name.as_str(),),
            );
            let _ = fs::remove_file(&script_file_path);
            return Err(Error::ScriptError(format!("Failed to run script: {}", e)));
        }

        // Wait for result with timeout (event-driven - blocks until message arrives!)
        let timeout = Duration::from_millis(500);
        let final_result = receiver.recv_timeout(timeout).ok();

        // Clean up our channel from the map (in case it's still there)
        {
            let mut channels = ACTIVATE_CHANNELS.write().unwrap();
            channels.remove(&method_name);
        }

        // Stop the script
        let _: Result<(), _> = script_proxy.method_call("org.kde.kwin.Script", "stop", ());

        // Unload the script from KWin to prevent memory leak
        let _: Result<(), _> = kwin_proxy.method_call(
            "org.kde.kwin.Scripting",
            "unloadScript",
            (script_name.as_str(),),
        );

        // Clean up temp file
        let _ = fs::remove_file(&script_file_path);

        match final_result.as_deref() {
            Some("activated") => Ok(()),
            Some("not_found") => Err(Error::ScriptError("Window not found".to_string())),
            _ => Err(Error::ScriptError("No response from activation script".to_string())),
        }
    }

    /// Start listening for window activation events
    ///
    /// Returns a Receiver that will receive WindowInfo whenever:
    /// - The active window changes (window switch)
    /// - The active window's title changes (e.g., browser tab switch, IDE file switch)
    ///
    /// This is event-driven (no polling required). Zero CPU usage when idle.
    ///
    /// The listener script runs in the background until `stop_focus_listener()` is called.
    ///
    /// Note: Only one listener can be active at a time. Calling this multiple times
    /// will stop the previous listener and start a new one.
    pub fn start_focus_listener(&self) -> Result<MpscReceiver<WindowInfo>, Error> {
        // Check if listener is already running
        {
            let listener_id = self.listener_script_id.read().unwrap();
            if listener_id.is_some() {
                // Stop the existing listener first
                drop(listener_id); // Release lock before calling stop
                self.stop_focus_listener()?;
            }
        }

        // Generate unique method name for this listener
        let method_name = format!("windowActivated_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));

        // Create channel for events
        let (sender, receiver) = mpsc::channel();

        // Store sender in global channel map
        {
            let mut channels = LISTENER_CHANNELS.write().unwrap();
            channels.insert(method_name.clone(), sender);
        }

        // Create persistent listener script
        let script = format!(
            r#"
                var currentClient = null;
                var captionConnection = null;

                function sendWindowInfo(client) {{
                    if (client) {{
                        callDBus("{}",
                                "/",
                                "",
                                "{}",
                                client.caption + "|" + client.resourceClass);
                    }}
                }}

                function setupClient(client) {{
                    // Disconnect from previous client's caption changes
                    if (captionConnection) {{
                        captionConnection.disconnect();
                        captionConnection = null;
                    }}

                    currentClient = client;
                    sendWindowInfo(client);

                    // Listen to title changes on the active window
                    if (client && client.captionChanged) {{
                        captionConnection = client.captionChanged.connect(function() {{
                            // Only send if this is still the active window
                            if (currentClient === workspace.activeWindow) {{
                                sendWindowInfo(currentClient);
                            }}
                        }});
                    }}
                }}

                // Send initial window immediately
                setupClient(workspace.activeWindow);

                // Listen for future activations
                workspace.windowActivated.connect(setupClient);
            "#,
            self.dbus_addr, method_name
        );

        // Generate unique script name and write to file
        let script_name = format!("focus2-listener-{}", uuid::Uuid::new_v4());
        let script_file_path = self.temp_dir.join(format!("{}.js", script_name));
        fs::write(&script_file_path, script)
            .map_err(|e| Error::IOError(format!("Failed to write listener script: {}", e)))?;

        // Load the script via D-Bus
        let kwin_proxy = self.kwin_conn.with_proxy(
            "org.kde.KWin",
            "/Scripting",
            Duration::from_millis(5000)
        );

        let (script_id,): (i32,) = kwin_proxy.method_call(
            "org.kde.kwin.Scripting",
            "loadScript",
            (script_file_path.to_str().unwrap(), script_name.as_str()),
        ).map_err(|e| Error::DBusError(format!("Failed to load listener script: {}", e)))?;

        // Store script ID, name, and method name for later unloading
        {
            let mut listener_id = self.listener_script_id.write().unwrap();
            *listener_id = Some(script_id);
        }
        {
            let mut listener_name = self.listener_script_name.write().unwrap();
            *listener_name = Some(script_name.clone());
        }
        {
            let mut listener_method = self.listener_method_name.write().unwrap();
            *listener_method = Some(method_name.clone());
        }

        let script_path = format!("/Scripting/Script{}", script_id);

        // Run the script (it will stay running)
        let script_proxy = self.kwin_conn.with_proxy(
            "org.kde.KWin",
            &script_path,
            Duration::from_millis(5000)
        );

        // Run the script (it will stay running) - if this fails, we need to unload it
        let run_result: Result<(), dbus::Error> = script_proxy.method_call("org.kde.kwin.Script", "run", ());
        if let Err(e) = run_result {
            // Remove our channel from global map
            {
                let mut channels = LISTENER_CHANNELS.write().unwrap();
                channels.remove(&method_name);
            }

            // Clean up the script before returning error
            let _: Result<(), _> = kwin_proxy.method_call(
                "org.kde.kwin.Scripting",
                "unloadScript",
                (script_name.as_str(),),
            );
            let _ = fs::remove_file(&script_file_path);

            // Clear stored IDs since we failed
            {
                let mut listener_id = self.listener_script_id.write().unwrap();
                *listener_id = None;
            }
            {
                let mut listener_name = self.listener_script_name.write().unwrap();
                *listener_name = None;
            }
            {
                let mut listener_method = self.listener_method_name.write().unwrap();
                *listener_method = None;
            }

            return Err(Error::ScriptError(format!("Failed to run listener script: {}", e)));
        }

        // Clean up temp file
        let _ = fs::remove_file(&script_file_path);

        Ok(receiver)
    }

    /// Stop the focus listener
    pub fn stop_focus_listener(&self) -> Result<(), Error> {
        let (script_id, script_name, method_name) = {
            let mut listener_id = self.listener_script_id.write().unwrap();
            let mut listener_name = self.listener_script_name.write().unwrap();
            let mut listener_method = self.listener_method_name.write().unwrap();
            (listener_id.take(), listener_name.take(), listener_method.take())
        };

        // Remove our channel from the global map
        if let Some(method) = method_name {
            let mut channels = LISTENER_CHANNELS.write().unwrap();
            channels.remove(&method);
        }

        if let Some(script_id) = script_id {
            let script_path = format!("/Scripting/Script{}", script_id);
            let script_proxy = self.kwin_conn.with_proxy(
                "org.kde.KWin",
                &script_path,
                Duration::from_millis(5000)
            );

            // Stop the script
            let _: Result<(), _> = script_proxy.method_call("org.kde.kwin.Script", "stop", ());

            // Unload the script from KWin to prevent memory leak
            if let Some(name) = script_name {
                let kwin_proxy = self.kwin_conn.with_proxy(
                    "org.kde.KWin",
                    "/Scripting",
                    Duration::from_millis(5000)
                );
                let _: Result<(), _> = kwin_proxy.method_call(
                    "org.kde.kwin.Scripting",
                    "unloadScript",
                    (name.as_str(),),
                );
            }
        }

        Ok(())
    }
}

impl Drop for KWinScriptClient {
    fn drop(&mut self) {
        // Stop the background thread
        self.stop_flag.store(true, Ordering::Relaxed);

        // Stop the listener if it's running
        let _ = self.stop_focus_listener();

        // Give the background thread time to exit
        std::thread::sleep(Duration::from_millis(150));
    }
}
