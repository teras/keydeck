// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::error_log;
use std::fs::{remove_file, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::LazyLock;

/// Lock file path: `<temp>/.keydeck.lock`. On Linux this resolves to
/// `/tmp/.keydeck.lock` (unchanged); on Windows/macOS it uses the platform
/// temp directory.
static KEYDECK_LOCK: LazyLock<PathBuf> =
    LazyLock::new(|| std::env::temp_dir().join(".keydeck.lock"));

/// Returns true if a process with the given PID is currently running.
/// Uses `sysinfo` for cross-platform process lookup.
fn is_process_running(pid: u32) -> bool {
    use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};
    let sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
    );
    sys.process(Pid::from_u32(pid)).is_some()
}

/// Returns the PID recorded in the lock file, but only if that process is
/// still alive. Used by the `--daemon status/stop/reload` controller to find
/// the running server. Returns `None` if no daemon is running.
pub fn running_pid() -> Option<u32> {
    let mut file = File::open(&*KEYDECK_LOCK).ok()?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;
    let pid = contents.trim().parse::<u32>().ok()?;
    is_process_running(pid).then_some(pid)
}

fn check_stale_lock() -> bool {
    if let Ok(mut file) = File::open(&*KEYDECK_LOCK) {
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_ok() {
            if let Ok(pid) = contents.trim().parse::<u32>() {
                // Check if the process with this PID is still running
                if !is_process_running(pid) {
                    // If not running, remove the stale lock file
                    let _ = remove_file(&*KEYDECK_LOCK);
                    return true;
                }
            }
        }
    }
    false
}

pub fn ensure_lock() {
    // Check for stale lock file
    if Path::new(&*KEYDECK_LOCK).exists() && !check_stale_lock() {
        error_log!("Error: Another instance of the program is already running.");
        process::exit(1);
    }
    // Attempt to create a new lock file with PID
    if let Err(e) = OpenOptions::new()
        .write(true)
        .create_new(true) // Only create if it doesn't already exist
        .open(&*KEYDECK_LOCK)
        .and_then(|mut file| writeln!(file, "{}", process::id()))
    {
        if e.kind() == ErrorKind::AlreadyExists {
            eprintln!("Error: Another instance of the program is already running.");
        } else {
            eprintln!("Error creating lock file: {}", e);
        }
        process::exit(1);
    }
}

pub fn cleanup_lock() {
    // Silently ignore errors during cleanup - the program is exiting anyway
    // and we don't want to panic during cleanup (which could cause double-panic)
    let _ = remove_file(&*KEYDECK_LOCK);
}
