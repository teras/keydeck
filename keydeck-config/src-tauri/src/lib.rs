// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use tauri::{Emitter, Manager};

#[derive(Debug, Serialize, Deserialize)]
struct IconInfo {
    filename: String,
    data_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DaemonStatus {
    running: bool,
    pid: Option<i32>,
    timestamp: i64,
}

mod backup_restore;
mod windows;

#[cfg(target_os = "linux")]
mod linux_icon_finder;

#[cfg(target_os = "windows")]
mod windows_icon_finder;

#[cfg(target_os = "macos")]
mod macos_icon_finder;

// Re-export keydeck types and functions for frontend
pub use keydeck_types::{
    get_config_dir, get_config_path, get_icon_dir, DeviceInfo, KeyDeckConf, DEFAULT_ICON_DIR_REL,
};

#[derive(Debug, Serialize, Deserialize)]
struct DeviceListItem {
    device_id: String,
    serial: String,
    model: String,
}

/// List all connected StreamDeck devices by executing keydeck --list
#[tauri::command]
fn list_devices() -> Result<Vec<DeviceListItem>, String> {
    // Find keydeck binary (assume it's in the parent target directory)
    let keydeck_bin = find_keydeck_binary()?;

    let output = Command::new(&keydeck_bin)
        .arg("--list")
        .output()
        .map_err(|e| format!("Failed to execute keydeck: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "keydeck --list failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    for line in stdout.lines() {
        if line.starts_with("Total devices:") || line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            devices.push(DeviceListItem {
                device_id: parts[0].to_string(),
                serial: parts[1].to_string(),
                model: parts[2].to_string(),
            });
        }
    }

    Ok(devices)
}

/// Get detailed device information by executing keydeck --info <device_id>
#[tauri::command]
fn get_device_info(device_id: String) -> Result<DeviceInfo, String> {
    let keydeck_bin = find_keydeck_binary()?;

    let output = Command::new(&keydeck_bin)
        .arg("--info")
        .arg(&device_id)
        .output()
        .map_err(|e| format!("Failed to execute keydeck: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "keydeck --info failed.\nStderr: {}\nStdout: {}",
            stderr, stdout
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    serde_yaml_ng::from_str(&stdout).map_err(|e| format!("Failed to parse device info: {}", e))
}

/// Load keydeck configuration from a file path (or default ~/.config/keydeck.yaml if path is None)
/// If the config file doesn't exist, returns a default empty configuration instead of an error.
/// This is expected behavior for first-time app launch.
#[tauri::command]
fn load_config(path: Option<String>) -> Result<KeyDeckConf, String> {
    let config_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        get_config_path()
    };

    if !config_path.exists() {
        // Return default empty config for first-time launch
        return Ok(KeyDeckConf::default());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    // If the file is empty, return default config
    if content.trim().is_empty() {
        return Ok(KeyDeckConf::default());
    }

    serde_yaml_ng::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))
}

/// List environment variable names available to the frontend for autocomplete
#[tauri::command]
fn list_env_vars() -> Vec<String> {
    std::env::vars().map(|(name, _)| name).collect()
}

#[tauri::command]
fn list_window_classes() -> Result<Vec<String>, String> {
    windows::list_window_classes()
}

/// Save keydeck configuration to ~/.config/keydeck/config.yaml atomically with timestamped backup
#[tauri::command]
fn save_config(config: KeyDeckConf) -> Result<(), String> {
    use std::fs;
    use std::time::SystemTime;

    let config_dir = get_config_dir();
    let config_path = get_config_path();

    // Ensure the directory exists
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    // Serialize config to YAML
    let yaml = serde_yaml_ng::to_string(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    // Step 1: Write to temporary file
    let temp_path = config_dir.join("config.tmp.yaml");
    fs::write(&temp_path, &yaml).map_err(|e| format!("Failed to write temp config file: {}", e))?;

    // Step 2: If current config exists, create timestamped backup
    if config_path.exists() {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let backup_name = format!(
            "config.{}.yaml",
            chrono::DateTime::from_timestamp(timestamp as i64, 0)
                .unwrap()
                .format("%Y%m%d_%H%M%S")
        );
        let backup_path = config_dir.join(&backup_name);

        fs::copy(&config_path, &backup_path)
            .map_err(|e| format!("Failed to create backup: {}", e))?;
    }

    // Step 3: Atomically replace config file (rename is atomic on Unix)
    fs::rename(&temp_path, &config_path)
        .map_err(|e| format!("Failed to save config file: {}", e))?;

    // Step 4: Cleanup old backups (keep only 10 most recent)
    cleanup_old_backups(&config_dir)?;

    Ok(())
}

/// Remove old backup files, keeping only the 10 most recent
fn cleanup_old_backups(config_dir: &PathBuf) -> Result<(), String> {
    use std::fs;

    // Read all backup files
    let entries =
        fs::read_dir(config_dir).map_err(|e| format!("Failed to read config directory: {}", e))?;

    let mut backups: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_name().to_string_lossy().starts_with("config.")
                && entry.file_name().to_string_lossy().ends_with(".yaml")
                && entry.file_name() != "config.yaml"
                && entry.file_name() != "config.tmp.yaml"
        })
        .collect();

    // Sort by modification time (newest first)
    backups.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    backups.reverse();

    // Remove all but the 10 most recent
    for backup in backups.iter().skip(10) {
        let _ = fs::remove_file(backup.path());
    }

    Ok(())
}

/// JSON shape emitted by `keydeck --daemon status`.
#[derive(Debug, Deserialize)]
struct DaemonStatusJson {
    running: bool,
    pid: Option<u32>,
    enabled: bool,
}

/// Query the daemon lifecycle status by invoking `keydeck --daemon status`.
///
/// The daemon prints `{"running":..,"pid":..,"enabled":..}` to stdout
/// regardless of exit code (0 = running, 1 = not running), so we parse
/// stdout unconditionally.
fn query_daemon_status() -> Result<DaemonStatusJson, String> {
    let keydeck_bin = find_keydeck_binary()?;

    let output = Command::new(&keydeck_bin)
        .args(["--daemon", "status"])
        .output()
        .map_err(|e| format!("Failed to execute keydeck: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).map_err(|e| {
        format!(
            "Failed to parse daemon status: {} (output: {})",
            e,
            stdout.trim()
        )
    })
}

/// Run `keydeck --daemon <verb>` for a lifecycle action that either succeeds
/// or fails. Returns the trimmed stderr/stdout as the error message on failure.
fn run_daemon(verb: &str) -> Result<(), String> {
    let keydeck_bin = find_keydeck_binary()?;

    let output = Command::new(&keydeck_bin)
        .arg("--daemon")
        .arg(verb)
        .output()
        .map_err(|e| format!("Failed to execute keydeck: {}", e))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let msg = if !stderr.trim().is_empty() {
        stderr.trim().to_string()
    } else if !stdout.trim().is_empty() {
        stdout.trim().to_string()
    } else {
        format!("keydeck --daemon {} failed", verb)
    };
    Err(msg)
}

/// Check if keydeck daemon is running (delegates to `keydeck --daemon status`)
#[tauri::command]
fn check_daemon_status() -> DaemonStatus {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    match query_daemon_status() {
        Ok(s) => DaemonStatus {
            running: s.running,
            pid: s.pid.map(|p| p as i32),
            timestamp,
        },
        Err(_) => DaemonStatus {
            running: false,
            pid: None,
            timestamp,
        },
    }
}

/// Check if the daemon is registered for autostart (delegates to `keydeck --daemon status`)
#[tauri::command]
fn check_service_enabled() -> bool {
    query_daemon_status().map(|s| s.enabled).unwrap_or(false)
}

/// Check if we should show the service prompt (shows for first 3 launches)
#[tauri::command]
fn should_show_service_prompt() -> bool {
    use std::fs;

    let counter_file = get_config_dir().join(".service_prompt_count");

    // Read current count, default to 0 if file doesn't exist
    let count: u32 = fs::read_to_string(&counter_file)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    // Show prompt for first 3 launches
    count < 3
}

/// Increment the service prompt counter
#[tauri::command]
fn increment_service_prompt_count() -> Result<(), String> {
    use std::fs;

    let config_dir = get_config_dir();
    let counter_file = config_dir.join(".service_prompt_count");

    // Ensure config directory exists
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    // Read current count, default to 0 if file doesn't exist
    let count: u32 = fs::read_to_string(&counter_file)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    // Increment and write back
    let new_count = count + 1;
    fs::write(&counter_file, new_count.to_string())
        .map_err(|e| format!("Failed to write counter file: {}", e))?;

    Ok(())
}

/// Set the service prompt counter to a specific value
#[tauri::command]
fn set_service_prompt_count(count: u32) -> Result<(), String> {
    use std::fs;

    let config_dir = get_config_dir();
    let counter_file = config_dir.join(".service_prompt_count");

    // Ensure config directory exists
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    // Write the specified count
    fs::write(&counter_file, count.to_string())
        .map_err(|e| format!("Failed to write counter file: {}", e))?;

    Ok(())
}

/// Reload the running daemon's configuration (delegates to `keydeck --daemon reload`)
#[tauri::command]
fn reload_keydeck() -> Result<(), String> {
    run_daemon("reload")
}

/// Register the daemon for autostart and start it now.
///
/// Autostart (install) and runtime (start) are orthogonal in the daemon's
/// lifecycle model, so "Start as Service" performs both: the daemon runs
/// immediately and comes back on every login.
#[tauri::command]
async fn start_daemon_service() -> Result<(), String> {
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        run_daemon("install")?;
        run_daemon("start")
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Stop the running daemon and remove its autostart registration.
#[tauri::command]
async fn stop_daemon_service() -> Result<(), String> {
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        run_daemon("stop")?;
        run_daemon("uninstall")
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Reinstall the daemon autostart entry (rewrites it against the current binary
/// path) and restart. Fixes a stale autostart entry after the binary moves.
#[tauri::command]
async fn reinstall_daemon_service() -> Result<(), String> {
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        // Remove any stale entry first; ignore errors if nothing is registered.
        let _ = run_daemon("uninstall");
        run_daemon("install")?;
        run_daemon("restart")
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Backup entire config directory to a zip file
#[tauri::command]
fn backup_config_directory(path: String) -> Result<(), String> {
    backup_restore::backup_config_directory(&path)
}

/// Restore entire config directory from a zip file
#[tauri::command]
fn restore_config_directory(path: String) -> Result<(), String> {
    backup_restore::restore_config_directory(&path)
}

/// Get the full path to an image file from the hard-coded icon directory
#[tauri::command]
fn get_image_path(filename: String) -> Result<String, String> {
    let base_dir = PathBuf::from(get_icon_dir());
    let image_path = base_dir.join(&filename);

    if !image_path.exists() {
        return Err(format!("Image not found: {}", image_path.display()));
    }

    image_path
        .to_str()
        .ok_or_else(|| "Invalid path encoding".to_string())
        .map(|s| s.to_string())
}

/// Check if a directory exists
#[tauri::command]
fn check_directory_exists(path: String) -> Result<bool, String> {
    let dir_path = PathBuf::from(&path);
    Ok(dir_path.exists() && dir_path.is_dir())
}

#[tauri::command]
fn list_icons() -> Result<Vec<IconInfo>, String> {
    let base_dir = PathBuf::from(get_icon_dir());

    if !base_dir.exists() {
        return Ok(Vec::new()); // Return empty list if directory doesn't exist
    }

    let entries = std::fs::read_dir(&base_dir)
        .map_err(|e| format!("Failed to read directory {}: {}", base_dir.display(), e))?;

    let mut icons = Vec::new();

    // Common image extensions
    let valid_extensions = ["png", "jpg", "jpeg", "gif", "bmp", "svg", "webp"];

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if let Some(ext_str) = ext.to_str() {
                    if valid_extensions.contains(&ext_str.to_lowercase().as_str()) {
                        if let Some(filename) = path.file_name() {
                            if let Some(filename_str) = filename.to_str() {
                                // Convert to data URL
                                if let Ok(data_url) =
                                    get_icon_data_url(path.to_string_lossy().to_string())
                                {
                                    icons.push(IconInfo {
                                        filename: filename_str.to_string(),
                                        data_url,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by filename
    icons.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(icons)
}

// Helper functions

fn find_keydeck_binary() -> Result<PathBuf, String> {
    // Platform-correct executable name (`keydeck` on Unix, `keydeck.exe` on Windows).
    let exe_name = format!("keydeck{}", std::env::consts::EXE_SUFFIX);

    // 1. Check in the same directory as the current executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let keydeck_path = exe_dir.join(&exe_name);
            if keydeck_path.exists() {
                return Ok(keydeck_path);
            }
        }
    }

    // 2. Search in PATH environment variable (uses the OS-specific separator)
    if let Some(path_env) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_env) {
            let keydeck_path = dir.join(&exe_name);
            if keydeck_path.exists() {
                return Ok(keydeck_path);
            }
        }
    }

    // 3. Try relative development paths (for running from source during development)
    let dev_paths = vec![
        PathBuf::from("../target/release").join(&exe_name),
        PathBuf::from("../target/debug").join(&exe_name),
        PathBuf::from("../../target/release").join(&exe_name),
        PathBuf::from("../../target/debug").join(&exe_name),
    ];

    for path in dev_paths {
        if path.exists() {
            return Ok(path);
        }
    }

    Err("keydeck binary not found. Please ensure keydeck is installed in the same directory or in PATH.".to_string())
}

/// Create the default icon directory if it doesn't exist
#[tauri::command]
fn ensure_default_icon_dir() -> Result<String, String> {
    let path = PathBuf::from(get_icon_dir());

    if !path.exists() {
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create icon directory: {}", e))?;
    }

    path.to_str()
        .ok_or_else(|| "Invalid path encoding".to_string())
        .map(|s| s.to_string())
}

/// List all installed applications (Linux only)
#[cfg(target_os = "linux")]
#[tauri::command]
async fn list_applications() -> Result<Vec<linux_icon_finder::AppInfo>, String> {
    // Run the blocking operation on a background thread
    let apps = tokio::task::spawn_blocking(|| linux_icon_finder::find_applications())
        .await
        .map_err(|e| format!("Task join error: {}", e))??;

    // Convert icon paths to base64 data URLs
    let apps_with_data_urls: Vec<linux_icon_finder::AppInfo> = apps
        .into_iter()
        .filter_map(|mut app| {
            // Convert the icon path to a data URL
            match get_icon_data_url(app.icon_path.clone()) {
                Ok(data_url) => {
                    app.icon_data_url = Some(data_url);
                    Some(app)
                }
                Err(_) => None, // Skip apps with failed icon conversion
            }
        })
        .collect();

    Ok(apps_with_data_urls)
}

/// Select and copy an application icon to the keydeck icons directory (Linux only)
#[cfg(target_os = "linux")]
#[tauri::command]
fn select_app_icon(app_name: String, icon_path: String) -> Result<String, String> {
    let icon_dir = get_icon_dir();
    linux_icon_finder::copy_app_icon(app_name, icon_path, icon_dir)
}

/// List all installed applications (Windows only)
#[cfg(target_os = "windows")]
#[tauri::command]
async fn list_applications() -> Result<Vec<windows_icon_finder::AppInfo>, String> {
    // Run the blocking operation on a background thread
    tokio::task::spawn_blocking(|| windows_icon_finder::find_applications())
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

/// Select and copy an application icon to the keydeck icons directory (Windows only)
#[cfg(target_os = "windows")]
#[tauri::command]
fn select_app_icon(app_name: String, icon_path: String) -> Result<String, String> {
    let icon_dir = get_icon_dir();
    windows_icon_finder::copy_app_icon(app_name, icon_path, icon_dir)
}

/// List all installed applications (macOS only)
#[cfg(target_os = "macos")]
#[tauri::command]
async fn list_applications() -> Result<Vec<macos_icon_finder::AppInfo>, String> {
    // Run the blocking filesystem scan + icon decoding on a background thread
    let apps = tokio::task::spawn_blocking(|| macos_icon_finder::find_applications())
        .await
        .map_err(|e| format!("Task join error: {}", e))??;

    // Convert the cached PNG icon paths to base64 data URLs for the frontend
    let apps_with_data_urls: Vec<macos_icon_finder::AppInfo> = apps
        .into_iter()
        .filter_map(|mut app| match get_icon_data_url(app.icon_path.clone()) {
            Ok(data_url) => {
                app.icon_data_url = Some(data_url);
                Some(app)
            }
            Err(_) => None, // Skip apps with failed icon conversion
        })
        .collect();

    Ok(apps_with_data_urls)
}

/// Select and copy an application icon to the keydeck icons directory (macOS only)
#[cfg(target_os = "macos")]
#[tauri::command]
fn select_app_icon(app_name: String, icon_path: String) -> Result<String, String> {
    let icon_dir = get_icon_dir();
    macos_icon_finder::copy_app_icon(app_name, icon_path, icon_dir)
}

/// Result of icon cleanup preview, categorizing icons by usage
#[derive(Debug, Serialize, Deserialize)]
struct IconCleanupPreview {
    /// Icons currently in use by the configuration
    in_use: Vec<String>,
    /// Icons protected by glob patterns in protected_icons config
    protected: Vec<String>,
    /// Icons not in use and not protected (will be deleted)
    unused: Vec<String>,
}

/// Preview which icons will be deleted by the cleanup process
#[tauri::command]
fn preview_icon_cleanup() -> Result<IconCleanupPreview, String> {
    let icon_dir = PathBuf::from(get_icon_dir());

    if !icon_dir.exists() {
        return Ok(IconCleanupPreview {
            in_use: Vec::new(),
            protected: Vec::new(),
            unused: Vec::new(),
        });
    }

    // Collect all icons from the icon directory
    let all_icons: Vec<String> = std::fs::read_dir(&icon_dir)
        .map_err(|e| format!("Failed to read icon directory: {}", e))?
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.is_file() {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            })
        })
        .collect();

    // Load the actual config from disk to find which icons are in use and protected patterns
    let config_path = get_config_path();
    let mut used_icons = std::collections::HashSet::new();
    let mut config_protected_patterns = Vec::new();

    if config_path.exists() {
        let config_content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        let config: KeyDeckConf = serde_yaml_ng::from_str(&config_content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;

        collect_used_icons(&config, &mut used_icons);

        // Get protected patterns from config (prefer config over parameter)
        if let Some(protected) = config.protected_icons {
            config_protected_patterns = protected;
        }
    }

    // Use protected patterns from config
    let final_protected_patterns = config_protected_patterns;

    // Categorize icons
    let mut in_use = Vec::new();
    let mut protected = Vec::new();
    let mut unused = Vec::new();

    for icon in all_icons {
        if used_icons.contains(&icon) {
            in_use.push(icon);
        } else if is_protected(&icon, &final_protected_patterns) {
            protected.push(icon);
        } else {
            unused.push(icon);
        }
    }

    // Sort for consistent display
    in_use.sort();
    protected.sort();
    unused.sort();

    Ok(IconCleanupPreview {
        in_use,
        protected,
        unused,
    })
}

/// Execute icon cleanup, deleting unused icons
#[tauri::command]
fn execute_icon_cleanup() -> Result<usize, String> {
    let icon_dir = PathBuf::from(get_icon_dir());

    if !icon_dir.exists() {
        return Ok(0);
    }

    // Get the preview to know which icons to delete (reads config from disk)
    let preview = preview_icon_cleanup()?;

    let mut deleted_count = 0;

    // Delete unused icons
    for icon in preview.unused {
        let icon_path = icon_dir.join(&icon);
        if let Err(e) = std::fs::remove_file(&icon_path) {
            eprintln!("Failed to delete icon {}: {}", icon, e);
        } else {
            deleted_count += 1;
        }
    }

    Ok(deleted_count)
}

/// Recursively collect all icon filenames referenced in the configuration
fn collect_used_icons(config: &KeyDeckConf, used_icons: &mut std::collections::HashSet<String>) {
    // Collect from button definitions
    if let Some(buttons) = &config.buttons {
        for button in buttons.values() {
            if let Some(icon) = &button.icon {
                used_icons.insert(icon.clone());
            }
        }
    }

    // Collect from templates
    if let Some(templates) = &config.templates {
        for page in templates.values() {
            collect_icons_from_page(page, used_icons);
        }
    }

    // Collect from page groups
    for pages in config.page_groups.values() {
        // Collect from pages in the group
        for page in pages.pages.values() {
            collect_icons_from_page(page, used_icons);
        }
    }
}

fn collect_icons_from_page(
    page: &keydeck_types::Page,
    used_icons: &mut std::collections::HashSet<String>,
) {
    for button_config in page.buttons.values() {
        collect_icons_from_button_config(button_config, used_icons);
    }
}

fn collect_icons_from_button_config(
    button_config: &keydeck_types::ButtonConfig,
    used_icons: &mut std::collections::HashSet<String>,
) {
    match button_config {
        keydeck_types::ButtonConfig::Template(_) => {
            // Template references are resolved at runtime, can't determine icons here
        }
        keydeck_types::ButtonConfig::Detailed(button) => {
            if let Some(icon) = &button.icon {
                used_icons.insert(icon.clone());
            }
        }
    }
}

/// Check if an icon matches any of the protected patterns
fn is_protected(icon: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        glob::Pattern::new(pattern)
            .map(|p| p.matches(icon))
            .unwrap_or(false)
    })
}

/// Read an icon file from any system path and return it as a base64 data URL
/// This bypasses Tauri's asset protocol restrictions for system icons
#[tauri::command]
fn get_icon_data_url(file_path: String) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    use std::fs;

    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(format!("Icon file not found: {}", file_path));
    }

    // Read the file
    let data = fs::read(&path).map_err(|e| format!("Failed to read icon file: {}", e))?;

    // Determine MIME type from extension
    let mime_type = match path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("bmp") => "image/bmp",
        Some("webp") => "image/webp",
        _ => "image/png", // Default to PNG
    };

    // Encode as base64
    let base64_data = general_purpose::STANDARD.encode(&data);

    // Return data URL
    Ok(format!("data:{};base64,{}", mime_type, base64_data))
}

/// Stream daemon logs to the frontend.
///
/// On Linux the daemon runs as a systemd user service, so logs are read from
/// the journal (`journalctl --user`). On Windows and macOS the daemon logs to
/// stdout/stderr of a detached process which is not captured to a queryable
/// store, so we emit a single informational entry instead.
#[cfg(target_os = "linux")]
#[tauri::command]
async fn stream_journal_logs(window: tauri::Window) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    // Spawn thread to handle log streaming after a small delay
    // This ensures the frontend listener is ready
    std::thread::spawn(move || {
        // Small delay to ensure frontend is listening
        std::thread::sleep(std::time::Duration::from_millis(100));

        // First, get historical logs (last 200 lines)
        let history_output = Command::new("journalctl")
            .args([
                "--user",
                "-u",
                "keydeck.service",
                "-n",
                "200",
                "--output=json",
            ])
            .output();

        match history_output {
            Ok(output) if output.status.success() => {
                let history = String::from_utf8_lossy(&output.stdout);
                for line in history.lines() {
                    if !line.trim().is_empty() {
                        if let Err(e) = window.emit("log-entry", line) {
                            eprintln!("Failed to emit log entry: {}", e);
                        }
                    }
                }
            }
            Ok(output) => {
                eprintln!(
                    "Failed to fetch history: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                eprintln!("Failed to execute journalctl: {}", e);
            }
        }

        // Then start streaming new logs
        let child = Command::new("journalctl")
            .args(["--user", "-u", "keydeck.service", "-f", "--output=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        match child {
            Ok(mut process) => {
                if let Some(stdout) = process.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            let _ = window.emit("log-entry", line);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to start journalctl streaming: {}", e);
            }
        }
    });

    Ok(())
}

/// Non-Linux fallback: no queryable log store, emit an informational entry.
#[cfg(not(target_os = "linux"))]
#[tauri::command]
async fn stream_journal_logs(window: tauri::Window) -> Result<(), String> {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        // Journal-JSON shaped entry so the frontend LogViewer can parse it.
        let msg = serde_json::json!({
            "MESSAGE": "Log streaming is not available on this platform. \
                        The daemon logs to its own stdout/stderr.",
            "PRIORITY": "6",
        })
        .to_string();
        let _ = window.emit("log-entry", msg);
    });

    Ok(())
}

/// Upload custom icon file to keydeck icons directory
/// Returns the filename of the saved icon
#[tauri::command]
fn upload_custom_icon(file_path: String, suggested_name: Option<String>) -> Result<String, String> {
    use std::fs;

    let source = PathBuf::from(&file_path);

    if !source.exists() {
        return Err(format!("Icon file not found: {}", file_path));
    }

    // Get icon directory
    let icon_dir = PathBuf::from(get_icon_dir());

    // Ensure icon directory exists
    fs::create_dir_all(&icon_dir).map_err(|e| format!("Failed to create icon directory: {}", e))?;

    // Get extension from source file
    let ext = source.extension().and_then(|s| s.to_str()).unwrap_or("png");

    // Determine base filename
    let base_name = if let Some(name) = suggested_name {
        // Use suggested name, sanitized
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
    } else {
        // Use original filename without extension
        source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("custom_icon")
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
    };

    // Find an available filename
    let mut filename = format!("{}.{}", base_name, ext);
    let mut dest = icon_dir.join(&filename);
    let mut counter = 2;

    // If file exists, try _2, _3, _4, etc.
    while dest.exists() {
        filename = format!("{}_{}.{}", base_name, counter, ext);
        dest = icon_dir.join(&filename);
        counter += 1;
    }

    // Copy the file
    fs::copy(&source, &dest).map_err(|e| format!("Failed to copy icon: {}", e))?;

    Ok(filename)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Set the window icon
            if let Some(window) = app.get_webview_window("main") {
                let icon_bytes = include_bytes!("../icons/icon.png");
                if let Ok(icon) = image::load_from_memory(icon_bytes) {
                    let rgba = icon.to_rgba8();
                    let (width, height) = rgba.dimensions();
                    let tauri_icon = tauri::image::Image::new_owned(rgba.into_raw(), width, height);
                    let _ = window.set_icon(tauri_icon);
                }
            }

            // Handle splashscreen: close it when main window is ready
            let splashscreen_window = app.get_webview_window("splashscreen");
            let main_window = app.get_webview_window("main");

            if let (Some(splashscreen), Some(main)) = (splashscreen_window, main_window) {
                // Listen for the main window to finish loading
                let main_clone = main.clone();
                let splashscreen_clone = splashscreen.clone();

                std::thread::spawn(move || {
                    // Wait a bit for the main window to be ready
                    std::thread::sleep(std::time::Duration::from_millis(500));

                    // Show main window
                    let _ = main_clone.show();

                    // Close splashscreen
                    let _ = splashscreen_clone.close();
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_devices,
            get_device_info,
            load_config,
            save_config,
            check_daemon_status,
            check_service_enabled,
            should_show_service_prompt,
            increment_service_prompt_count,
            set_service_prompt_count,
            start_daemon_service,
            stop_daemon_service,
            reinstall_daemon_service,
            list_env_vars,
            list_window_classes,
            reload_keydeck,
            backup_config_directory,
            restore_config_directory,
            get_image_path,
            check_directory_exists,
            list_icons,
            ensure_default_icon_dir,
            list_applications,
            select_app_icon,
            preview_icon_cleanup,
            execute_icon_cleanup,
            get_icon_data_url,
            upload_custom_icon,
            stream_journal_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
