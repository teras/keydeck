// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::error_log;
use std::fs::{remove_file, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::LazyLock;

/// Lock file path: `<runtime-dir>/.keydeck.lock`. Prefers the per-user runtime dir
/// (`$XDG_RUNTIME_DIR`, e.g. `/run/user/1000`) so the lock is private to the user
/// and auto-removed on logout. Falls back to the temp dir — already per-user on
/// macOS/Windows; only Linux's global `/tmp` is the case this avoids.
static KEYDECK_LOCK: LazyLock<PathBuf> = LazyLock::new(|| {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir)
        .join(".keydeck.lock")
});

/// The start-time (seconds since epoch) of `pid`, or `None` if no such process
/// exists. Refreshes ONLY this one pid — no full process scan (and `without_tasks`
/// skips its thread list, which a presence/identity check never needs). Uses
/// `sysinfo` for cross-platform lookup.
fn process_start_time(pid: u32) -> Option<u64> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
    let pid = Pid::from_u32(pid);
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        false,
        ProcessRefreshKind::nothing().without_tasks(),
    );
    sys.process(pid).map(|p| p.start_time())
}

/// Parse the lock file into `(pid, recorded_start_time)`. New files store
/// `"<pid>:<start>"`; a legacy bare `"<pid>"` yields `start = None`.
fn read_lock() -> Option<(u32, Option<u64>)> {
    let mut file = File::open(&*KEYDECK_LOCK).ok()?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;
    let line = contents.trim();
    match line.split_once(':') {
        Some((pid, start)) => Some((pid.trim().parse().ok()?, start.trim().parse().ok())),
        None => Some((line.parse().ok()?, None)),
    }
}

/// True only if the lock's pid is genuinely still *our* daemon: the process exists
/// AND its start-time matches the one recorded. This defeats PID reuse — after an
/// unclean exit (SIGKILL/crash/power loss) the OS can hand the stale pid to an
/// unrelated process, which a bare "does pid exist?" check would wrongly accept.
/// Legacy bare-pid locks have no recorded start-time, so they fall back to a
/// best-effort existence check.
fn lock_is_live(pid: u32, recorded_start: Option<u64>) -> bool {
    match (process_start_time(pid), recorded_start) {
        (Some(actual), Some(recorded)) => actual == recorded,
        (Some(_), None) => true,
        (None, _) => false,
    }
}

/// Returns the PID recorded in the lock file, but only if that process is still our
/// running daemon (see [`lock_is_live`]). Used by the `--daemon status/stop/reload`
/// controller to find the running server. Returns `None` if no daemon is running.
pub fn running_pid() -> Option<u32> {
    let (pid, start) = read_lock()?;
    lock_is_live(pid, start).then_some(pid)
}

fn check_stale_lock() -> bool {
    if let Some((pid, start)) = read_lock() {
        if !lock_is_live(pid, start) {
            // Recorded process is gone (or the pid was recycled) — clear the lock.
            let _ = remove_file(&*KEYDECK_LOCK);
            return true;
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
    // Attempt to create a new lock file recording our PID and start-time, so a
    // future instance can tell a live daemon from a recycled PID.
    let pid = process::id();
    let start = process_start_time(pid).unwrap_or(0);
    if let Err(e) = OpenOptions::new()
        .write(true)
        .create_new(true) // Only create if it doesn't already exist
        .open(&*KEYDECK_LOCK)
        .and_then(|mut file| writeln!(file, "{}:{}", pid, start))
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
