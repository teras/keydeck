use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use tauri::Manager;

// Re-export keydeck types for frontend
pub use keydeck::{DeviceInfo, KeyDeckConf};

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
        return Err(format!("keydeck --list failed: {}", String::from_utf8_lossy(&output.stderr)));
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
        return Err(format!("keydeck --info failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_yaml_ng::from_str(&stdout)
        .map_err(|e| format!("Failed to parse device info: {}", e))
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
        return Ok(KeyDeckConf::from_file_or_default());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    serde_yaml_ng::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))
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
    fs::write(&temp_path, &yaml)
        .map_err(|e| format!("Failed to write temp config file: {}", e))?;

    // Step 2: If current config exists, create timestamped backup
    if config_path.exists() {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let backup_name = format!("config.{}.yaml",
            chrono::DateTime::from_timestamp(timestamp as i64, 0)
                .unwrap()
                .format("%Y%m%d_%H%M%S"));
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
    let entries = fs::read_dir(config_dir)
        .map_err(|e| format!("Failed to read config directory: {}", e))?;

    let mut backups: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_name()
                .to_string_lossy()
                .starts_with("config.") &&
            entry.file_name()
                .to_string_lossy()
                .ends_with(".yaml") &&
            entry.file_name() != "config.yaml" &&
            entry.file_name() != "config.tmp.yaml"
        })
        .collect();

    // Sort by modification time (newest first)
    backups.sort_by_key(|entry| {
        entry.metadata()
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

/// Send SIGHUP signal to keydeck server to reload configuration
#[tauri::command]
fn reload_keydeck() -> Result<(), String> {
    use std::fs;

    // Read PID from lock file
    let lock_path = PathBuf::from("/tmp/.keydeck.lock");

    if !lock_path.exists() {
        return Err("keydeck server is not running (no lock file found)".to_string());
    }

    let pid_str = fs::read_to_string(&lock_path)
        .map_err(|e| format!("Failed to read lock file: {}", e))?;

    let pid: i32 = pid_str.trim().parse()
        .map_err(|e| format!("Invalid PID in lock file: {}", e))?;

    // Send SIGHUP signal
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    kill(Pid::from_raw(pid), Signal::SIGHUP)
        .map_err(|e| format!("Failed to send SIGHUP: {}", e))?;

    Ok(())
}

/// Export configuration to a specified file path
#[tauri::command]
fn export_config(config: KeyDeckConf, path: String) -> Result<(), String> {
    let yaml = serde_yaml_ng::to_string(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&path, yaml)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

/// Get the full path to an image file from the image directory
#[tauri::command]
fn get_image_path(image_dir: Option<String>, filename: String) -> Result<String, String> {
    let base_dir = if let Some(dir) = image_dir {
        PathBuf::from(dir)
    } else {
        // Default to ~/.config/keydeck/icons if no image_dir specified
        let mut path = PathBuf::from(std::env::var("HOME").expect("HOME not set"));
        path.push(".config/keydeck/icons");
        path
    };

    let image_path = base_dir.join(&filename);

    if !image_path.exists() {
        return Err(format!("Image not found: {}", image_path.display()));
    }

    image_path.to_str()
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
fn list_icons(image_dir: Option<String>) -> Result<Vec<String>, String> {
    let base_dir = if let Some(dir) = image_dir {
        PathBuf::from(dir)
    } else {
        // Default to ~/.config/keydeck/icons if no image_dir specified
        let mut path = PathBuf::from(std::env::var("HOME").map_err(|e| format!("HOME not set: {}", e))?);
        path.push(".config/keydeck/icons");
        path
    };

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
                                icons.push(filename_str.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    icons.sort();
    Ok(icons)
}

// Helper functions

fn find_keydeck_binary() -> Result<PathBuf, String> {
    // Try release build first, then debug build
    let possible_paths = vec![
        PathBuf::from("../target/release/keydeck"),
        PathBuf::from("../target/debug/keydeck"),
        PathBuf::from("../../target/release/keydeck"),
        PathBuf::from("../../target/debug/keydeck"),
    ];

    for path in possible_paths {
        if path.exists() {
            return Ok(path);
        }
    }

    Err("keydeck binary not found. Please build keydeck first.".to_string())
}

fn get_config_path() -> PathBuf {
    let mut path = PathBuf::from(std::env::var("HOME").expect("HOME not set"));
    path.push(".config/keydeck/config.yaml");
    path
}

fn get_config_dir() -> PathBuf {
    let mut path = PathBuf::from(std::env::var("HOME").expect("HOME not set"));
    path.push(".config/keydeck");
    path
}

/// Create the default icon directory if it doesn't exist
#[tauri::command]
fn ensure_default_icon_dir() -> Result<String, String> {
    let mut path = PathBuf::from(std::env::var("HOME").map_err(|e| format!("HOME not set: {}", e))?);
    path.push(".config/keydeck/icons");

    if !path.exists() {
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create icon directory: {}", e))?;
    }

    path.to_str()
        .ok_or_else(|| "Invalid path encoding".to_string())
        .map(|s| s.to_string())
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
            reload_keydeck,
            export_config,
            get_image_path,
            check_directory_exists,
            list_icons,
            ensure_default_icon_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
