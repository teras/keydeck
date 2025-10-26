use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub icon_path: String,
}

// Cache for application list
static APP_CACHE: Lazy<Mutex<Option<Vec<AppInfo>>>> = Lazy::new(|| Mutex::new(None));

/// Main public function to find all applications
pub fn find_applications() -> Result<Vec<AppInfo>, String> {
    // Check cache first
    {
        let cache = APP_CACHE.lock().unwrap();
        if let Some(cached_apps) = cache.as_ref() {
            return Ok(cached_apps.clone());
        }
    }

    // Scan desktop files
    let apps = scan_desktop_files()?;

    // Cache the results
    {
        let mut cache = APP_CACHE.lock().unwrap();
        *cache = Some(apps.clone());
    }

    Ok(apps)
}

/// Scan .desktop file directories and parse them
fn scan_desktop_files() -> Result<Vec<AppInfo>, String> {
    let mut apps = Vec::new();

    // Desktop file locations
    let desktop_dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share/applications"),
    ];

    for dir in desktop_dirs {
        if !dir.exists() {
            continue;
        }

        let entries = fs::read_dir(&dir)
            .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                if let Some((name, icon_path)) = parse_desktop_file(&path) {
                    apps.push(AppInfo { name, icon_path });
                }
            }
        }
    }

    // Sort by name
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(apps)
}

/// Parse a .desktop file and extract name and icon path
fn parse_desktop_file(path: &Path) -> Option<(String, String)> {
    let content = fs::read_to_string(path).ok()?;

    let mut in_desktop_entry = false;
    let mut app_type = None;
    let mut name = None;
    let mut icon = None;
    let mut no_display = false;
    let mut hidden = false;

    for line in content.lines() {
        let line = line.trim();

        // Check section headers
        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        // Parse key=value pairs
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "Type" => app_type = Some(value.trim().to_string()),
                "Name" => name = Some(value.trim().to_string()),
                "Icon" => icon = Some(value.trim().to_string()),
                "NoDisplay" => no_display = value.trim() == "true",
                "Hidden" => hidden = value.trim() == "true",
                _ => {}
            }
        }
    }

    // Only process applications
    if app_type.as_deref() != Some("Application") {
        return None;
    }

    // Skip hidden or NoDisplay entries
    if no_display || hidden {
        return None;
    }

    let name = name?;
    let icon_field = icon?;

    // Resolve icon path
    let icon_path = resolve_icon_path(&icon_field)?;

    Some((name, icon_path))
}

/// Resolve icon name/path to actual file path
fn resolve_icon_path(icon_field: &str) -> Option<String> {
    // If it's already an absolute path, use it directly
    if icon_field.starts_with('/') {
        let path = PathBuf::from(icon_field);
        if path.exists() {
            return Some(icon_field.to_string());
        }
        return None;
    }

    // It's an icon name, search common locations
    let icon_name = icon_field;
    let extensions = ["png", "svg", "xpm", "jpg"];

    // 1. Check /usr/share/pixmaps
    for ext in &extensions {
        let pixmap_path = PathBuf::from(format!("/usr/share/pixmaps/{}.{}", icon_name, ext));
        if pixmap_path.exists() {
            return Some(pixmap_path.to_string_lossy().to_string());
        }
    }

    // 2. Check hicolor theme (common sizes)
    let sizes = ["48x48", "64x64", "128x128", "256x256", "scalable"];
    let icon_theme_dirs = vec![
        PathBuf::from("/usr/share/icons/hicolor"),
        PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share/icons/hicolor"),
    ];

    for theme_dir in icon_theme_dirs {
        for size in &sizes {
            for ext in &extensions {
                let icon_path = theme_dir.join(size).join("apps").join(format!("{}.{}", icon_name, ext));
                if icon_path.exists() {
                    return Some(icon_path.to_string_lossy().to_string());
                }
            }
        }
    }

    None
}

/// Copy app icon to keydeck icons directory
pub fn copy_app_icon(app_name: String, icon_path: String, icon_dir: String) -> Result<String, String> {
    let source = PathBuf::from(&icon_path);

    if !source.exists() {
        return Err(format!("Icon file not found: {}", icon_path));
    }

    // Get extension
    let ext = source.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png");

    // Sanitize app name for filename (replace spaces and special chars with underscores)
    let sanitized_name = app_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .collect::<String>()
        .to_lowercase();

    // Find an available filename
    let mut filename = format!("{}.{}", sanitized_name, ext);
    let mut dest = PathBuf::from(&icon_dir).join(&filename);
    let mut counter = 2;

    // If file exists, try _2, _3, _4, etc.
    while dest.exists() {
        filename = format!("{}_{}.{}", sanitized_name, counter, ext);
        dest = PathBuf::from(&icon_dir).join(&filename);
        counter += 1;
    }

    // Copy the file
    fs::copy(&source, &dest)
        .map_err(|e| format!("Failed to copy icon: {}", e))?;

    Ok(filename)
}
