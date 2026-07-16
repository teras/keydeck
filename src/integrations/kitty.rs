// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! kitty terminal integration: installs a watcher that reports the focused
//! window's foreground program to keydeck as the `context` variable.
//!
//! Install writes two files and one line:
//!   * `<keydeck-config>/kitty_context_watcher.py` — the watcher (embedded here,
//!     refreshed on every daemon start so it never drifts behind the binary).
//!   * `<kitty-config>/keydeck.conf` — a fragment holding the `watcher` directive.
//!   * one `include keydeck.conf` line in `kitty.conf`, so the user's own config
//!     body is never rewritten — only that single, clearly-marked include.
//!
//! Uninstall reverses exactly those. Everything is idempotent.

use crate::{error_log, info_log};
use std::fs;
use std::path::PathBuf;

/// The watcher script, embedded so a `keydeck` upgrade always ships the current one.
const WATCHER: &str = include_str!("kitty_context_watcher.py");

const FRAGMENT_NAME: &str = "keydeck.conf";
const INCLUDE_LINE: &str = "include keydeck.conf";
const WATCHER_NAME: &str = "kitty_context_watcher.py";

/// Comment block written above the `include` line in kitty.conf, so anyone reading
/// the config knows what it is and how to manage it. Removed again on uninstall.
const MARKER_LINES: &[&str] = &[
    "# keydeck: terminal-context integration.",
    "# Loads a watcher that reports the program running in the focused tab",
    "# (claude, mc, …) to keydeck, so Stream Deck pages can auto-switch per app.",
    "# Manage with:  keydeck --integration kitty  install | uninstall | status",
];

/// Path where the watcher script is installed (in keydeck's own config dir, so it
/// is versioned with the binary rather than copied into kitty's dir).
fn watcher_path() -> PathBuf {
    keydeck::get_config_dir().join(WATCHER_NAME)
}

/// kitty's config directory: `$KITTY_CONFIG_DIRECTORY`, else `<XDG_CONFIG>/kitty`.
fn kitty_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("KITTY_CONFIG_DIRECTORY") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    // get_config_dir() is `<XDG_CONFIG>/keydeck`; its parent is `<XDG_CONFIG>`.
    let base = keydeck::get_config_dir()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("kitty")
}

fn kitty_conf() -> PathBuf {
    kitty_config_dir().join("kitty.conf")
}

fn fragment_path() -> PathBuf {
    kitty_config_dir().join(FRAGMENT_NAME)
}

/// Dispatches `--integration kitty <action>`.
pub fn run(action: &str) -> i32 {
    match action {
        "install" => install(),
        "uninstall" => uninstall(),
        "status" => status(),
        other => {
            error_log!(
                "Unknown action '{}'. Use one of: {}",
                other,
                crate::integrations::ACTIONS
            );
            1
        }
    }
}

fn write_watcher() -> std::io::Result<()> {
    let path = watcher_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, WATCHER)
}

fn install() -> i32 {
    // 1. The watcher script (keydeck's dir).
    if let Err(e) = write_watcher() {
        error_log!("Failed to write watcher script {:?}: {}", watcher_path(), e);
        return 1;
    }

    // 2. The kitty fragment holding the watcher directive (absolute path).
    let kitty_dir = kitty_config_dir();
    if let Err(e) = fs::create_dir_all(&kitty_dir) {
        error_log!("Failed to create kitty config dir {:?}: {}", kitty_dir, e);
        return 1;
    }
    let fragment = format!(
        "# Managed by `keydeck --integration kitty`. Do not edit; overwritten on upgrade.\n\
         watcher {}\n",
        watcher_path().display()
    );
    if let Err(e) = fs::write(fragment_path(), fragment) {
        error_log!("Failed to write kitty fragment {:?}: {}", fragment_path(), e);
        return 1;
    }

    // 3. One `include` line in kitty.conf (idempotent).
    if let Err(e) = ensure_include() {
        error_log!("Failed to update {:?}: {}", kitty_conf(), e);
        return 1;
    }

    info_log!("kitty integration installed.");
    info_log!("Restart kitty (or open a new instance) for the watcher to load.");
    0
}

fn uninstall() -> i32 {
    let mut ok = true;
    if let Err(e) = remove_include() {
        error_log!("Failed to update {:?}: {}", kitty_conf(), e);
        ok = false;
    }
    if let Err(e) = remove_file_if_present(&fragment_path()) {
        error_log!("Failed to remove {:?}: {}", fragment_path(), e);
        ok = false;
    }
    if let Err(e) = remove_file_if_present(&watcher_path()) {
        error_log!("Failed to remove {:?}: {}", watcher_path(), e);
        ok = false;
    }
    if ok {
        info_log!("kitty integration removed. Restart kitty to drop the watcher.");
        0
    } else {
        1
    }
}

/// Prints `{"script":b,"registered":b,"installed":b}` and returns 0 if fully
/// installed, 1 otherwise — mirroring `--daemon status`, so the config UI can
/// render not-installed / installed / partial states.
fn status() -> i32 {
    let script = watcher_path().exists();
    let registered = fragment_path().exists() && include_present().unwrap_or(false);
    let installed = script && registered;
    println!(
        "{{\"script\":{},\"registered\":{},\"installed\":{}}}",
        script, registered, installed
    );
    if installed {
        0
    } else {
        1
    }
}

/// Refreshes the watcher script from the embedded copy if the integration is
/// installed and the running binary is newer than the on-disk script. Keeps the
/// script in lockstep with the daemon across upgrades. No-op if not installed.
pub fn refresh_if_installed() {
    let path = watcher_path();
    if !path.exists() {
        return; // not installed → nothing to refresh
    }
    let exe_newer = match (exe_modified(), file_modified(&path)) {
        (Some(exe), Some(file)) => exe > file,
        _ => false,
    };
    if exe_newer {
        if let Err(e) = write_watcher() {
            error_log!("Failed to refresh kitty watcher {:?}: {}", path, e);
        } else {
            info_log!("Refreshed kitty watcher script to match this build");
        }
    }
}

// --- kitty.conf include line management -------------------------------------

fn include_present() -> std::io::Result<bool> {
    let conf = kitty_conf();
    if !conf.exists() {
        return Ok(false);
    }
    let body = fs::read_to_string(&conf)?;
    Ok(body.lines().any(|l| l.trim() == INCLUDE_LINE))
}

fn ensure_include() -> std::io::Result<()> {
    let conf = kitty_conf();
    let mut body = if conf.exists() {
        fs::read_to_string(&conf)?
    } else {
        String::new()
    };
    if body.lines().any(|l| l.trim() == INCLUDE_LINE) {
        return Ok(()); // already registered
    }
    if !body.is_empty() && !body.ends_with('\n') {
        body.push('\n');
    }
    for line in MARKER_LINES {
        body.push_str(line);
        body.push('\n');
    }
    body.push_str(INCLUDE_LINE);
    body.push('\n');
    fs::write(&conf, body)
}

fn remove_include() -> std::io::Result<()> {
    let conf = kitty_conf();
    if !conf.exists() {
        return Ok(());
    }
    let body = fs::read_to_string(&conf)?;
    let kept: Vec<&str> = body
        .lines()
        .filter(|l| {
            let t = l.trim();
            t != INCLUDE_LINE && !MARKER_LINES.contains(&t)
        })
        .collect();
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    fs::write(&conf, out)
}

// --- small helpers ----------------------------------------------------------

fn remove_file_if_present(path: &PathBuf) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

fn exe_modified() -> Option<std::time::SystemTime> {
    std::env::current_exe()
        .ok()
        .and_then(|p| fs::metadata(p).ok())
        .and_then(|m| m.modified().ok())
}

fn file_modified(path: &PathBuf) -> Option<std::time::SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}
