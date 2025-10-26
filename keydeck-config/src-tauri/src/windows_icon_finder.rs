use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Represents a Windows application with its name and icon path
#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
    pub name: String,
    pub icon_path: String, // Path to extracted .ico or .png file
}

/// Cache for application list to avoid repeated filesystem scanning
static APP_CACHE: Lazy<Mutex<Option<Vec<AppInfo>>>> = Lazy::new(|| Mutex::new(None));

/// Find all applications from Windows Start Menu shortcuts
pub fn find_applications() -> Result<Vec<AppInfo>, String> {
    // Check cache first
    {
        let cache = APP_CACHE.lock().unwrap();
        if let Some(apps) = cache.as_ref() {
            return Ok(apps.clone());
        }
    }

    let mut apps = Vec::new();
    let mut seen_names = HashMap::new();

    // Scan Start Menu locations
    let start_menu_paths = get_start_menu_paths()?;

    for base_path in start_menu_paths {
        if let Err(e) = scan_directory_for_shortcuts(&base_path, &mut apps, &mut seen_names) {
            eprintln!("Warning: Failed to scan {}: {}", base_path.display(), e);
        }
    }

    // Sort by name
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Cache the result
    {
        let mut cache = APP_CACHE.lock().unwrap();
        *cache = Some(apps.clone());
    }

    Ok(apps)
}

/// Get Start Menu paths for all users and current user
fn get_start_menu_paths() -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();

    // All Users Start Menu
    if let Ok(program_data) = std::env::var("ProgramData") {
        let all_users_menu = PathBuf::from(program_data)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs");
        if all_users_menu.exists() {
            paths.push(all_users_menu);
        }
    }

    // Current User Start Menu
    if let Ok(app_data) = std::env::var("APPDATA") {
        let user_menu = PathBuf::from(app_data)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs");
        if user_menu.exists() {
            paths.push(user_menu);
        }
    }

    if paths.is_empty() {
        return Err("Could not find Start Menu directories".to_string());
    }

    Ok(paths)
}

/// Recursively scan a directory for .lnk files
fn scan_directory_for_shortcuts(
    dir: &Path,
    apps: &mut Vec<AppInfo>,
    seen_names: &mut HashMap<String, usize>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            // Recursively scan subdirectories
            let _ = scan_directory_for_shortcuts(&path, apps, seen_names);
        } else if path.extension().and_then(|s| s.to_str()) == Some("lnk") {
            if let Ok(app_info) = parse_shortcut(&path) {
                // Deduplicate by name
                let name_lower = app_info.name.to_lowercase();
                if !seen_names.contains_key(&name_lower) {
                    seen_names.insert(name_lower, apps.len());
                    apps.push(app_info);
                }
            }
        }
    }

    Ok(())
}

/// Parse a .lnk shortcut file and extract application info
fn parse_shortcut(lnk_path: &Path) -> Result<AppInfo, String> {
    // Parse the .lnk file using the lnk crate
    let shortcut = lnk::ShellLink::open(lnk_path)
        .map_err(|e| format!("Failed to parse shortcut: {}", e))?;

    // Get the target path
    let target_path = shortcut
        .link_info()
        .and_then(|info| info.local_base_path())
        .or_else(|| shortcut.relative_path())
        .ok_or_else(|| "No target path found in shortcut".to_string())?;

    // Get the application name from the shortcut filename
    let app_name = lnk_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "Invalid shortcut filename".to_string())?
        .to_string();

    // Get icon location (prefer explicit icon, fallback to target exe)
    let icon_path = shortcut
        .icon_location()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| target_path.to_string());

    // Clean up icon path (remove icon index if present, e.g., "app.exe,0")
    let icon_path = icon_path.split(',').next().unwrap_or(&icon_path).to_string();

    Ok(AppInfo {
        name: app_name,
        icon_path,
    })
}

/// Copy and extract icon from an application to the icon directory
pub fn copy_app_icon(
    app_name: String,
    icon_path: String,
    icon_dir: String,
) -> Result<String, String> {
    // Sanitize the app name to create a valid filename
    let sanitized_name = sanitize_filename(&app_name);

    // Ensure icon directory exists
    fs::create_dir_all(&icon_dir)
        .map_err(|e| format!("Failed to create icon directory: {}", e))?;

    // Check if source icon file exists
    let source_path = PathBuf::from(&icon_path);
    if !source_path.exists() {
        return Err(format!("Icon file not found: {}", icon_path));
    }

    // Determine output filename
    let mut output_filename = format!("{}.png", sanitized_name);
    let mut counter = 2;
    let icon_dir_path = PathBuf::from(&icon_dir);

    // Handle filename collisions
    while icon_dir_path.join(&output_filename).exists() {
        output_filename = format!("{}_{}.png", sanitized_name, counter);
        counter += 1;
    }

    let output_path = icon_dir_path.join(&output_filename);

    // Extract icon based on file type
    if source_path.extension().and_then(|s| s.to_str()) == Some("ico") {
        // Direct .ico file - just copy it
        fs::copy(&source_path, &output_path)
            .map_err(|e| format!("Failed to copy icon file: {}", e))?;
    } else if is_pe_file(&source_path) {
        // Extract icon from .exe or .dll
        extract_icon_from_pe(&source_path, &output_path)?;
    } else {
        // Try to copy as-is (might be .png, .jpg, etc.)
        fs::copy(&source_path, &output_path)
            .map_err(|e| format!("Failed to copy icon file: {}", e))?;
    }

    Ok(output_filename)
}

/// Check if a file is a PE executable (.exe or .dll)
fn is_pe_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        matches!(ext.to_lowercase().as_str(), "exe" | "dll")
    } else {
        false
    }
}

/// Extract icon from a PE file (.exe or .dll) and save as PNG
fn extract_icon_from_pe(source_path: &Path, output_path: &Path) -> Result<(), String> {
    // Read the PE file
    let exe_data = fs::read(source_path)
        .map_err(|e| format!("Failed to read executable: {}", e))?;

    // Extract icon using exeico crate
    let icons = exeico::get_icos(&exe_data)
        .map_err(|e| format!("Failed to extract icon from PE file: {}", e))?;

    if icons.is_empty() {
        return Err("No icons found in executable".to_string());
    }

    // Get the first (usually largest/best quality) icon
    let icon_data = &icons[0];

    // Save as PNG (exeico returns PNG data)
    fs::write(output_path, icon_data)
        .map_err(|e| format!("Failed to write icon file: {}", e))?;

    Ok(())
}

/// Sanitize an application name to create a valid filename
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else if c.is_whitespace() {
                '_'
            } else {
                '_'
            }
        })
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Google Chrome"), "google_chrome");
        assert_eq!(sanitize_filename("VLC media player"), "vlc_media_player");
        assert_eq!(sanitize_filename("7-Zip"), "7-zip");
        assert_eq!(sanitize_filename("App (x64)"), "app__x64_");
    }
}
