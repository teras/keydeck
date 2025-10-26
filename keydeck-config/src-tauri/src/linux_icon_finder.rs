use once_cell::sync::Lazy;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::process::Command;

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
    // Desktop file locations
    let desktop_dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share/applications"),
    ];

    // Collect all .desktop files first
    let mut desktop_files = Vec::new();
    for dir in desktop_dirs {
        if !dir.exists() {
            continue;
        }

        let entries = fs::read_dir(&dir)
            .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    desktop_files.push(path);
                }
            }
        }
    }

    // Parse desktop files in parallel (this is where icon conversion happens)
    let apps: Vec<AppInfo> = desktop_files
        .par_iter()
        .filter_map(|path| parse_desktop_file(path))
        .map(|(name, icon_path)| AppInfo { name, icon_path })
        .collect();

    // Sort by name
    let mut apps = apps;
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

/// Get or create temporary directory for converted icons
fn get_temp_icons_dir() -> Result<PathBuf, String> {
    let temp_dir = std::env::temp_dir().join("keydeck_converted_icons");

    if !temp_dir.exists() {
        fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp icons directory: {}", e))?;
    }

    Ok(temp_dir)
}

/// Convert unsupported icon formats to PNG using Rust libraries
/// Only PNG and JPG are natively supported, all others need conversion
fn convert_icon_to_png(source_path: &Path) -> Result<PathBuf, String> {
    let ext = source_path.extension()
        .and_then(|s| s.to_str())
        .ok_or("No file extension")?
        .to_lowercase();

    // PNG and JPG/JPEG are already supported, no conversion needed
    if ext == "png" || ext == "jpg" || ext == "jpeg" {
        return Err("Already a supported format".to_string());
    }

    // Create output path in temp directory with deterministic name
    let temp_dir = get_temp_icons_dir()?;

    // Convert source path to safe filename: /usr/share/icons/audacity.xpm -> _usr_share_icons_audacity.xpm.png
    let safe_name = source_path.to_string_lossy()
        .replace('/', "_")
        .replace('\\', "_");
    let output_path = temp_dir.join(format!("{}.png", safe_name));

    // Skip if already converted (cache hit)
    if output_path.exists() {
        return Ok(output_path);
    }

    // Try format-specific conversion
    match ext.as_str() {
        "svg" => convert_svg_to_png(source_path, &output_path),
        "gif" | "bmp" | "webp" => convert_with_image_crate(source_path, &output_path),
        "xpm" => convert_with_imagemagick(source_path, &output_path),
        _ => Err(format!("Unsupported format: {}", ext)),
    }
}

/// Convert SVG to PNG using resvg
fn convert_svg_to_png(source_path: &Path, output_path: &Path) -> Result<PathBuf, String> {
    use resvg::usvg;

    // Read SVG file
    let svg_data = fs::read(source_path)
        .map_err(|e| format!("Failed to read SVG: {}", e))?;

    // Parse SVG
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &options)
        .map_err(|e| format!("Failed to parse SVG: {}", e))?;

    // Calculate size (limit to 512x512)
    let size = tree.size();
    let scale = (512.0 / size.width().max(size.height())).min(1.0);
    let width = (size.width() * scale) as u32;
    let height = (size.height() * scale) as u32;

    // Render to pixmap
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or("Failed to create pixmap")?;

    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());

    // Save as PNG
    pixmap.save_png(output_path)
        .map_err(|e| format!("Failed to save PNG: {}", e))?;

    Ok(output_path.to_path_buf())
}

/// Convert GIF/BMP/WebP to PNG using the image crate
fn convert_with_image_crate(source_path: &Path, output_path: &Path) -> Result<PathBuf, String> {
    // Load image with the image crate
    let img = image::open(source_path)
        .map_err(|e| format!("Failed to load image: {}", e))?;

    // Convert to RGBA
    let rgba = img.to_rgba8();

    // Save as PNG
    rgba.save(output_path)
        .map_err(|e| format!("Failed to save PNG: {}", e))?;

    Ok(output_path.to_path_buf())
}

/// Check if ImageMagick convert command is available
fn is_imagemagick_available() -> bool {
    Command::new("convert")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Convert XPM to PNG using ImageMagick as fallback
fn convert_with_imagemagick(source_path: &Path, output_path: &Path) -> Result<PathBuf, String> {
    // Check if convert is available first
    if !is_imagemagick_available() {
        return Err("ImageMagick 'convert' command not found. Please install ImageMagick to use XPM icons.".to_string());
    }

    let status = Command::new("convert")
        .arg(source_path)
        .arg(output_path)
        .status();

    match status {
        Ok(status) if status.success() => Ok(output_path.to_path_buf()),
        Ok(_) => Err("ImageMagick conversion failed".to_string()),
        Err(e) => Err(format!("Failed to run convert command: {}", e)),
    }
}

/// Resolve icon name/path to actual file path, converting XPM/SVG to PNG if needed
fn resolve_icon_path(icon_field: &str) -> Option<String> {
    // If it's already an absolute path, use it directly
    if icon_field.starts_with('/') {
        let path = PathBuf::from(icon_field);
        if path.exists() {
            return convert_if_needed(&path);
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
            return convert_if_needed(&pixmap_path);
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
                    return convert_if_needed(&icon_path);
                }
            }
        }
    }

    None
}

/// Convert icon to PNG if it's not PNG or JPG, otherwise return original path
fn convert_if_needed(path: &Path) -> Option<String> {
    let ext = path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase());

    match ext.as_deref() {
        Some("png") | Some("jpg") | Some("jpeg") => {
            // Already supported format, return as-is
            Some(path.to_string_lossy().to_string())
        }
        Some(_) => {
            // Unsupported format (XPM, SVG, GIF, BMP, etc.) - try to convert
            match convert_icon_to_png(path) {
                Ok(converted_path) => Some(converted_path.to_string_lossy().to_string()),
                Err(_) => {
                    // Conversion failed (e.g., XPM without ImageMagick)
                    // Return None to skip this icon rather than showing broken icon
                    None
                }
            }
        }
        None => {
            // No extension, return as-is
            Some(path.to_string_lossy().to_string())
        }
    }
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
