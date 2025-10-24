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

    // Parse output format: "0300:1010 355499441494 Akp153E"
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
#[tauri::command]
fn load_config(path: Option<String>) -> Result<KeyDeckConf, String> {
    let config_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        get_config_path()
    };

    if !config_path.exists() {
        return Err(format!("Config file not found at {}", config_path.display()));
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    serde_yaml_ng::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))
}

/// Save keydeck configuration to ~/.config/keydeck.yaml
#[tauri::command]
fn save_config(config: KeyDeckConf) -> Result<(), String> {
    let config_path = get_config_path();

    // Ensure the directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let yaml = serde_yaml_ng::to_string(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_path, yaml)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

/// Send SIGHUP signal to keydeck server to reload configuration
#[tauri::command]
fn reload_keydeck() -> Result<(), String> {
    use std::fs;

    // Read PID from lock file
    let lock_path = PathBuf::from("/tmp/keydeck.lock");

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
        // Default to ~/.config/keydeck_images if no image_dir specified
        let mut path = PathBuf::from(std::env::var("HOME").expect("HOME not set"));
        path.push(".config/keydeck_images");
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
        // Default to ~/.config/keydeck_images if no image_dir specified
        let mut path = PathBuf::from(std::env::var("HOME").map_err(|e| format!("HOME not set: {}", e))?);
        path.push(".config/keydeck_images");
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
    path.push(".config/keydeck.yaml");
    path
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
