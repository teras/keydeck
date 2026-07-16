// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! Control socket for external context variables (Unix only).
//!
//! Opens a line-oriented `UnixListener` at `$XDG_RUNTIME_DIR/keydeck.sock` (falling
//! back to the system temp dir). Each line is a command:
//!
//! ```text
//! setvar <key> <value>
//! clearvar <key>
//! ```
//!
//! and is turned into a [`DeviceEvent::SetContextVar`], exactly as `listener_focus`
//! injects `FocusChanges`. The `keydeck --set key=value` CLI is the thin client that
//! writes these lines, so external watchers never need to know the protocol.

use crate::event::{send, DeviceEvent};
use crate::{error_log, verbose_log};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

/// Path of the control socket: `$XDG_RUNTIME_DIR/keydeck.sock`, or a per-user name
/// in the system temp dir when `XDG_RUNTIME_DIR` is unset.
pub fn control_socket_path() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir).join("keydeck.sock");
        }
    }
    let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    std::env::temp_dir().join(format!("keydeck-{}.sock", user))
}

/// Spawns the control-socket listener thread. Binds the socket (replacing any stale
/// file) and injects a `SetContextVar` event for every valid command line received.
pub fn spawn_context_listener(tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>) {
    let tx = tx.clone();
    let active = active.clone();
    let path = control_socket_path();

    // Replace a stale socket left behind by a previous run.
    let _ = std::fs::remove_file(&path);

    let listener = match UnixListener::bind(&path) {
        Ok(listener) => listener,
        Err(e) => {
            error_log!("Failed to bind control socket {:?}: {}", path, e);
            return;
        }
    };
    verbose_log!("Listening for context commands on {:?}", path);

    thread::spawn(move || {
        for stream in listener.incoming() {
            if !active.load(Ordering::Relaxed) {
                break;
            }
            match stream {
                Ok(stream) => {
                    let reader = BufReader::new(stream);
                    for line in reader.lines() {
                        match line {
                            Ok(line) => handle_line(&tx, &line),
                            Err(_) => break,
                        }
                    }
                }
                Err(e) => error_log!("Control socket accept error: {}", e),
            }
        }
    });
}

/// Parses one command line and injects the matching event.
fn handle_line(tx: &Sender<DeviceEvent>, line: &str) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    let mut parts = line.splitn(3, char::is_whitespace);
    match parts.next().unwrap_or("") {
        "setvar" => {
            let key = parts.next().unwrap_or("").trim().to_string();
            if key.is_empty() {
                error_log!("Control command 'setvar' missing key: {:?}", line);
                return;
            }
            let value = parts.next().map(|s| s.to_string());
            send(tx, DeviceEvent::SetContextVar { key, value });
        }
        "clearvar" => {
            let key = parts.next().unwrap_or("").trim().to_string();
            if key.is_empty() {
                error_log!("Control command 'clearvar' missing key: {:?}", line);
                return;
            }
            send(tx, DeviceEvent::SetContextVar { key, value: None });
        }
        other => error_log!("Unknown control command: {:?}", other),
    }
}

/// CLI client for `keydeck --set key=value`. Connects to the control socket and writes
/// a `setvar`/`clearvar` line (empty value clears). Silently succeeds if the daemon is
/// not running, so external watchers never break when keydeck is stopped.
pub fn send_context_var(arg: &str) {
    let (key, value) = match arg.split_once('=') {
        Some((key, value)) => (key.trim(), value),
        None => (arg.trim(), ""),
    };
    if key.is_empty() {
        eprintln!("Error: --set requires key=value");
        std::process::exit(1);
    }
    let line = if value.is_empty() {
        format!("clearvar {}\n", key)
    } else {
        format!("setvar {} {}\n", key, value)
    };

    let path = control_socket_path();
    match UnixStream::connect(&path) {
        Ok(mut stream) => {
            if let Err(e) = stream.write_all(line.as_bytes()) {
                eprintln!("Error: failed to write to control socket: {}", e);
                std::process::exit(1);
            }
        }
        Err(_) => {
            // Daemon not running — nothing to update. Not an error for watchers.
        }
    }
}
