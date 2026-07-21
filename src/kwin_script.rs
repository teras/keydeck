// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! KWin Scripting API for Wayland
//!
//! This module provides a Rust interface to KWin's JavaScript scripting API via D-Bus.
//! It allows querying active window information and activating windows without
//! requiring external tools like kdotool.
//!
//! ## The `/Scripting/ScriptN` id-collision problem (and how this module avoids it)
//!
//! KWin's dynamic scripting D-Bus API is treacherous: `loadScript(file, pluginName)`
//! returns an i32 id and is *supposed* to register a matching `/Scripting/ScriptN`
//! object node that you then `run()`. But that id is derived from the size of KWin's
//! internal script list and is **reused** after any `unloadScript` in the session. When
//! a returned id collides with a node that already exists, the new registration fails
//! **silently**: no fresh node appears, `isScriptLoaded` still reports `true`, and
//! `run()`-ing that node executes a **foreign** script's body instead of ours (or
//! nothing). This is exactly why focus pages silently stopped updating.
//!
//! We defeat this deterministically without ever touching foreign scripts: we snapshot
//! the `/Scripting` child nodes immediately before `loadScript`, and only trust a
//! returned id whose `ScriptN` node was **not** already present (proof it is ours). On a
//! collision we unload our failed attempt, burn the colliding id with a throwaway "pad"
//! script, and retry. We never call the global `Scripting.start()` / `reconfigure`,
//! which would start unrelated foreign scripts.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver as MpscReceiver, Sender as MpscSender};
use std::sync::{Arc, LazyLock, RwLock};
use std::time::Duration;
use tokio::runtime::Runtime;
use zbus::fdo::{RequestNameFlags, RequestNameReply};

// Global channels for routing D-Bus callbacks to the correct client
static ACTIVATE_CHANNELS: LazyLock<Arc<RwLock<HashMap<String, MpscSender<String>>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));
static LISTENER_CHANNELS: LazyLock<Arc<RwLock<HashMap<String, MpscSender<WindowInfo>>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));
// Proof-of-life confirmations, keyed by per-start confirm tag.
static CONFIRM_CHANNELS: LazyLock<Arc<RwLock<HashMap<String, MpscSender<()>>>>> =
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

// Fixed script name for the single-instance focus listener.
const LISTENER_SCRIPT_NAME: &str = "keydeck-focus-listener";
const LISTENER_METHOD_NAME: &str = "keydeck_windowActivated";
// Well-known bus name the daemon tries to own so KWin scripts have a stable callback
// target that survives a daemon restart (a crashed daemon's orphaned listener script
// keeps delivering to whoever currently owns this name).
const WELL_KNOWN_NAME: &str = "onl.ycode.keydeck";
// Bounded retry budget for the fresh-node acquisition loop. Each iteration either
// succeeds or consumes exactly one colliding id via a pad script, so it converges in
// 1-2 iterations in practice; 64 is a generous ceiling that still guarantees
// termination under adversarial concurrent id churn.
const MAX_ACQUIRE_ATTEMPTS: usize = 64;

/// D-Bus callback handler for KWin script responses.
/// Registered on the ObjectServer at /onl/ycode/keydeck to receive callDBus callbacks.
struct CallbackHandler;

#[zbus::interface(name = "onl.ycode.keydeck.Callback")]
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

    /// Proof-of-life signal: the listener script calls this once, right after it
    /// finishes connecting `windowActivated`, tagged with this start's confirm id.
    /// Receiving it proves the fresh node really ran *our* body (not a same-numbered
    /// foreign node), independent of whether an active window exists yet.
    async fn listener_started(&self, tag: &str) {
        let mut channels = CONFIRM_CHANNELS.write().unwrap();
        if let Some(sender) = channels.remove(tag) {
            let _ = sender.send(());
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

        // Register callback handler for incoming KWin script responses
        conn.object_server()
            .at("/onl/ycode/keydeck", CallbackHandler)
            .map_err(|e| {
                Error::DBusError(format!("Failed to register callback handler: {}", e))
            })?;

        // Try to own the stable well-known name. The first client (the daemon's focus
        // listener) wins it; short-lived activation clients fall back to the unique
        // name, which works just as well for their one-shot scripts. Baking the
        // well-known name into the listener script means a restarted daemon re-acquires
        // it and immediately keeps receiving events from any still-loaded script.
        let dbus_addr = match conn
            .request_name_with_flags(WELL_KNOWN_NAME, RequestNameFlags::DoNotQueue.into())
        {
            Ok(RequestNameReply::PrimaryOwner) => WELL_KNOWN_NAME.to_string(),
            _ => conn
                .unique_name()
                .ok_or_else(|| Error::DBusError("No unique bus name".to_string()))?
                .to_string(),
        };

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

    /// Parse `<node name="ScriptN">` child-node entries out of an Introspect XML blob.
    fn parse_script_nodes(xml: &str) -> HashSet<String> {
        let mut set = HashSet::new();
        for part in xml.split("<node name=\"").skip(1) {
            if let Some(end) = part.find('"') {
                let name = &part[..end];
                if name.starts_with("Script") {
                    set.insert(name.to_string());
                }
            }
        }
        set
    }

    /// Snapshot the current child nodes of /Scripting (Script0, Script1, ...).
    fn snapshot_script_nodes(&self) -> Result<HashSet<String>, Error> {
        let reply = self.kwin_call(
            "/Scripting",
            "org.freedesktop.DBus.Introspectable",
            "Introspect",
            &(),
        )?;
        let xml: String = reply
            .body()
            .deserialize()
            .map_err(|e| Error::DBusError(format!("Failed to read introspection XML: {}", e)))?;
        Ok(Self::parse_script_nodes(&xml))
    }

    /// Load `script_path_str` under `plugin_name` and return `(id, object_path)` of a
    /// script node that is *provably* ours.
    ///
    /// See the module docs for the id-collision problem. We snapshot the node set
    /// immediately before `loadScript`; if `ScriptN` (N = the returned id) was NOT in
    /// that snapshot, KWin could only have produced it by registering a fresh node for
    /// our load, so it is provably ours. If it WAS already present, we hit a collision:
    /// unload our (non-functional) attempt, burn the colliding id with a disposable pad
    /// script, and retry. Pads are unloaded once we're done, win or lose.
    fn load_script_deterministic(
        &self,
        script_path_str: &str,
        plugin_name: &str,
    ) -> Result<(i32, String), Error> {
        let mut pads: Vec<String> = Vec::new();

        let result = (|| -> Result<(i32, String), Error> {
            for _ in 0..MAX_ACQUIRE_ATTEMPTS {
                let before = self.snapshot_script_nodes()?;

                let reply = self.kwin_call(
                    "/Scripting",
                    "org.kde.kwin.Scripting",
                    "loadScript",
                    &(script_path_str, plugin_name),
                )?;
                let script_id: i32 = reply
                    .body()
                    .deserialize()
                    .map_err(|e| Error::DBusError(format!("Failed to read script ID: {}", e)))?;

                // KWin returns a negative id (-1) when `plugin_name` is already loaded.
                // No node is created, so `Script{id}` would be absent from the snapshot
                // and wrongly look "fresh". Treat it as a name clash: unload the stale
                // registration by name and retry (no pad needed - this is not an id
                // collision, so nothing consumed a fresh id).
                if script_id < 0 {
                    let _ = self.kwin_call(
                        "/Scripting",
                        "org.kde.kwin.Scripting",
                        "unloadScript",
                        &(plugin_name,),
                    );
                    continue;
                }

                let candidate = format!("Script{}", script_id);

                if !before.contains(&candidate) {
                    return Ok((script_id, format!("/Scripting/{}", candidate)));
                }

                // Collision: our registration silently landed on a foreign, already-live
                // node. Unload our attempt and burn the colliding id with a throwaway pad
                // so the next loadScript call is forced onto a fresh id.
                let _ = self.kwin_call(
                    "/Scripting",
                    "org.kde.kwin.Scripting",
                    "unloadScript",
                    &(plugin_name,),
                );

                let pad_name = format!("keydeck-pad-{}", uuid::Uuid::new_v4());
                let pad_path = self.temp_dir.join(format!("{}.js", pad_name));
                if fs::write(&pad_path, "// keydeck deterministic-load padding script\n").is_ok() {
                    if let Some(pad_path_str) = pad_path.to_str() {
                        let _ = self.kwin_call(
                            "/Scripting",
                            "org.kde.kwin.Scripting",
                            "loadScript",
                            &(pad_path_str, pad_name.as_str()),
                        );
                        pads.push(pad_name);
                    }
                    let _ = fs::remove_file(&pad_path);
                }
            }
            Err(Error::ScriptError(format!(
                "Could not obtain a provably fresh script node for '{}' after {} attempts",
                plugin_name, MAX_ACQUIRE_ATTEMPTS
            )))
        })();

        for pad in pads {
            let _ = self.kwin_call(
                "/Scripting",
                "org.kde.kwin.Scripting",
                "unloadScript",
                &(pad.as_str(),),
            );
        }

        result
    }

    /// Escape a string for safe interpolation inside a JS double-quoted string literal.
    /// A window title/class containing `"`, `\`, or a newline would otherwise break the
    /// generated script — and a KWin JS syntax error makes `run()` report success while
    /// executing nothing (the script even self-unloads), i.e. a silent no-op activation.
    fn js_escape(s: &str) -> String {
        let mut out = String::with_capacity(s.len() + 2);
        for c in s.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                // JS forbids raw U+2028/U+2029 in string literals; escape them too.
                '\u{2028}' => out.push_str("\\u2028"),
                '\u{2029}' => out.push_str("\\u2029"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out
    }

    /// Activate a window matching the given class and title.
    ///
    /// Ownership of the *result* is proven by `method_uuid`: it is baked into the JS and
    /// echoed back in `ActivateResult`, and the dispatch matches on it (see
    /// `CallbackHandler::activate_result`). A foreign script that we might run() by
    /// mistake in the (rare) load race does not know the uuid, so it can never forge an
    /// "activated" reply — the worst case is a benign early start of a foreign
    /// loaded-not-started script plus our own timeout -> Error. Hence no separate
    /// proof-of-life is needed here (unlike the persistent listener).
    pub fn activate_window(&self, class: &str, title: &str) -> Result<(), Error> {
        let script_name = format!("keydeck-activate-{}", uuid::Uuid::new_v4());
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
                            "/onl/ycode/keydeck",
                            "onl.ycode.keydeck.Callback",
                            "ActivateResult",
                            "{}",
                            "activated");
                }} else {{
                    callDBus("{}",
                            "/onl/ycode/keydeck",
                            "onl.ycode.keydeck.Callback",
                            "ActivateResult",
                            "{}",
                            "not_found");
                }}
            "#,
            Self::js_escape(class),
            Self::js_escape(title),
            self.dbus_addr,
            method_uuid,
            self.dbus_addr,
            method_uuid
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

        // Load the script, retrying past any id collisions until we have a node
        // that's provably ours (see load_script_deterministic).
        let (_script_id, script_path) =
            match self.load_script_deterministic(script_path_str, &script_name) {
                Ok(v) => v,
                Err(e) => {
                    let _ = fs::remove_file(&script_file_path);
                    return Err(e);
                }
            };

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

        // Wait for result with timeout. 2s (matching the listener confirm) so a busy
        // compositor doesn't report "No response" for an activation that actually ran.
        let final_result = receiver.recv_timeout(Duration::from_secs(2)).ok();

        // Clean up channel
        {
            let mut channels = ACTIVATE_CHANNELS.write().unwrap();
            channels.remove(&method_uuid);
        }

        // Unload the script BY NAME only. We must NOT stop() by candidate node path:
        // on a timeout the load race means that path could belong to a FOREIGN script,
        // and stop() there would kill someone else's script. unloadScript(name) targets
        // only our own registration and stops it if running.
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

    /// The persistent listener script body, parameterized with the callback target and
    /// this start's proof-of-life confirm tag.
    fn listener_script_body(dbus_addr: &str, confirm_tag: &str) -> String {
        format!(
            r#"
                var currentClient = null;
                var captionConnection = null;
                var windowActivatedConnection = null;

                function sendWindowInfo(client) {{
                    if (client) {{
                        callDBus("{dbus_addr}",
                                "/onl/ycode/keydeck",
                                "onl.ycode.keydeck.Callback",
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

                callDBus("{dbus_addr}",
                        "/onl/ycode/keydeck",
                        "onl.ycode.keydeck.Callback",
                        "ListenerStarted",
                        "{confirm_tag}");
            "#,
            dbus_addr = dbus_addr,
            confirm_tag = confirm_tag,
        )
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

        // Create persistent listener script with a per-start confirm tag
        let confirm_tag = uuid::Uuid::new_v4().to_string();
        let script = Self::listener_script_body(&self.dbus_addr, &confirm_tag);

        let script_file_path = self.temp_dir.join(format!("{}.js", LISTENER_SCRIPT_NAME));
        if let Err(e) = fs::write(&script_file_path, script) {
            LISTENER_CHANNELS.write().unwrap().remove(LISTENER_METHOD_NAME);
            return Err(Error::IOError(format!(
                "Failed to write listener script: {}",
                e
            )));
        }

        let script_path_str = match script_file_path.to_str() {
            Some(s) => s,
            None => {
                LISTENER_CHANNELS.write().unwrap().remove(LISTENER_METHOD_NAME);
                let _ = fs::remove_file(&script_file_path);
                return Err(Error::DBusError(format!(
                    "Listener script path contains invalid UTF-8: {:?}",
                    script_file_path
                )));
            }
        };

        // Load the script, retrying past any id collisions until we have a node
        // that's provably ours (see load_script_deterministic).
        let (script_id, script_path) =
            match self.load_script_deterministic(script_path_str, LISTENER_SCRIPT_NAME) {
                Ok(v) => v,
                Err(e) => {
                    LISTENER_CHANNELS.write().unwrap().remove(LISTENER_METHOD_NAME);
                    let _ = fs::remove_file(&script_file_path);
                    return Err(e);
                }
            };

        {
            let mut listener_id = self.listener_script_id.write().unwrap();
            *listener_id = Some(script_id);
        }

        // Register the proof-of-life channel BEFORE run() so the script's
        // ListenerStarted callback can never race ahead of us.
        let (confirm_tx, confirm_rx) = mpsc::channel::<()>();
        {
            let mut channels = CONFIRM_CHANNELS.write().unwrap();
            channels.insert(confirm_tag.clone(), confirm_tx);
        }

        // Run the script. The temp file is intentionally kept until after run():
        // KWin reads the script file lazily at run() time, not at loadScript() time.
        if let Err(e) = self.kwin_call(&script_path, "org.kde.kwin.Script", "run", &()) {
            self.abort_listener_start(&confirm_tag, &script_file_path);
            return Err(Error::ScriptError(format!(
                "Failed to run listener script: {}",
                e
            )));
        }

        // Confirm the script body actually executed (proof it's really ours, not a
        // same-numbered foreign node we ran by mistake). Independent of active-window
        // presence, unlike waiting for a WindowActivated event. The 2s budget is
        // generous so a loaded/slow compositor at boot doesn't false-fail here and push
        // the orchestrator onto the X11 fallback path (which stalls under Wayland).
        let confirmed = confirm_rx.recv_timeout(Duration::from_secs(2)).is_ok();

        if !confirmed {
            self.abort_listener_start(&confirm_tag, &script_file_path);
            return Err(Error::ScriptError(
                "Listener script did not confirm startup (no proof-of-life signal)".to_string(),
            ));
        }

        CONFIRM_CHANNELS.write().unwrap().remove(&confirm_tag);
        let _ = fs::remove_file(&script_file_path);
        self.owns_listener = true;
        Ok(receiver)
    }

    /// Roll back a failed `start_focus_listener` attempt: drop channels, unload the
    /// script by name, remove the temp file, and clear the recorded id.
    fn abort_listener_start(&self, confirm_tag: &str, script_file_path: &std::path::Path) {
        CONFIRM_CHANNELS.write().unwrap().remove(confirm_tag);
        LISTENER_CHANNELS.write().unwrap().remove(LISTENER_METHOD_NAME);
        let _ = self.kwin_call(
            "/Scripting",
            "org.kde.kwin.Scripting",
            "unloadScript",
            &(LISTENER_SCRIPT_NAME,),
        );
        let _ = fs::remove_file(script_file_path);
        *self.listener_script_id.write().unwrap() = None;
    }

    /// Is our focus-listener script still loaded in KWin? Used by the daemon's watchdog
    /// to detect a KWin restart (which silently drops all dynamically-loaded scripts)
    /// and re-install promptly. On a transient D-Bus error we assume it is still loaded,
    /// so a momentary hiccup never triggers a needless restart.
    pub fn is_listener_loaded(&self) -> bool {
        match self.kwin_call(
            "/Scripting",
            "org.kde.kwin.Scripting",
            "isScriptLoaded",
            &(LISTENER_SCRIPT_NAME,),
        ) {
            Ok(reply) => reply.body().deserialize::<bool>().unwrap_or(true),
            Err(_) => true,
        }
    }

    /// Stop the focus listener
    pub fn stop_focus_listener(&self) -> Result<(), Error> {
        let had_id = {
            let mut listener_id = self.listener_script_id.write().unwrap();
            listener_id.take().is_some()
        };

        {
            let mut channels = LISTENER_CHANNELS.write().unwrap();
            channels.remove(LISTENER_METHOD_NAME);
        }

        if had_id {
            // unloadScript stops (if running) and unloads by pluginName - reliable
            // regardless of which /Scripting/ScriptN node it currently owns.
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
            .remove::<CallbackHandler, _>("/onl/ycode/keydeck");

        // Clear remaining channel entries
        if self.owns_listener {
            let mut listener_channels = LISTENER_CHANNELS.write().unwrap();
            listener_channels.remove(LISTENER_METHOD_NAME);
        }
    }
}
