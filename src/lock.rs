use crate::error_log;
use std::fs::{remove_file, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::Path;
use std::process;

const KEYDECK_LOCK: &str = "/tmp/.keydeck.lock";

fn check_stale_lock() -> bool {
    if let Ok(mut file) = File::open(KEYDECK_LOCK) {
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_ok() {
            if let Ok(pid) = contents.trim().parse::<u32>() {
                // Check if the process with this PID is still running
                let running = Path::new(&format!("/proc/{}", pid)).exists();
                if !running {
                    // If not running, remove the stale lock file
                    let _ = remove_file(KEYDECK_LOCK);
                    return true;
                }
            }
        }
    }
    false
}

pub fn ensure_lock() {
    // Check for stale lock file
    if Path::new(KEYDECK_LOCK).exists() && !check_stale_lock() {
        error_log!("Error: Another instance of the program is already running.");
        process::exit(1);
    }
    // Attempt to create a new lock file with PID
    if let Err(e) = OpenOptions::new()
        .write(true)
        .create_new(true) // Only create if it doesn't already exist
        .open(KEYDECK_LOCK)
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
    remove_file(KEYDECK_LOCK).expect("Failed to remove lock file /tmp/.keydeck.lock");
}