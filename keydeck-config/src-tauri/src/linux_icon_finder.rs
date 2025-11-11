// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use once_cell::sync::Lazy;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::process::Command;
use image::GenericImageView;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub icon_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_data_url: Option<String>,
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
        .map(|(name, icon_path)| AppInfo { name, icon_path, icon_data_url: None })
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

/// Get or create cache directory for normalized icons
fn get_cache_icons_dir() -> Result<PathBuf, String> {
    let home = std::env::var("HOME")
        .map_err(|_| "HOME environment variable not set".to_string())?;
    let cache_dir = PathBuf::from(home).join(".cache/keydeck/icons");

    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to create cache icons directory: {}", e))?;
    }

    Ok(cache_dir)
}

/// Cache and normalize icon to 256x256 PNG with timestamp-based invalidation
/// Processes ALL image formats, scales to max 256x256, and caches to ~/.cache/keydeck/icons/
fn cache_and_normalize_icon(source_path: &Path) -> Result<PathBuf, String> {
    // Create cache directory
    let cache_dir = get_cache_icons_dir()?;

    // Convert source path to safe filename: /usr/share/icons/audacity.xpm -> _usr_share_icons_audacity.xpm.png
    let safe_name = source_path.to_string_lossy()
        .replace('/', "_")
        .replace('\\', "_");
    let cache_path = cache_dir.join(format!("{}.png", safe_name));

    // Check if cache is valid (exists and is newer than source)
    if cache_path.exists() {
        if let (Ok(source_meta), Ok(cache_meta)) = (fs::metadata(source_path), fs::metadata(&cache_path)) {
            if let (Ok(source_mtime), Ok(cache_mtime)) = (source_meta.modified(), cache_meta.modified()) {
                if cache_mtime >= source_mtime {
                    // Cache is valid, return cached path
                    return Ok(cache_path);
                }
            }
        }
    }

    // Cache miss or invalid - need to regenerate
    let ext = source_path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase());

    // Load and normalize based on format
    match ext.as_deref() {
        Some("svg") => normalize_svg_to_cache(source_path, &cache_path),
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("bmp") | Some("webp") => {
            normalize_raster_to_cache(source_path, &cache_path)
        }
        Some("xpm") => normalize_xpm_to_cache(source_path, &cache_path),
        _ => Err(format!("Unsupported format: {:?}", ext)),
    }
}

/// Normalize SVG to 256x256 PNG
fn normalize_svg_to_cache(source_path: &Path, cache_path: &Path) -> Result<PathBuf, String> {
    use resvg::usvg;

    // Read SVG file
    let svg_data = fs::read(source_path)
        .map_err(|e| format!("Failed to read SVG: {}", e))?;

    // Parse SVG
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &options)
        .map_err(|e| format!("Failed to parse SVG: {}", e))?;

    // Calculate size (limit to 256x256, maintain aspect ratio)
    let size = tree.size();
    let scale = (256.0 / size.width().max(size.height())).min(1.0);
    let width = (size.width() * scale) as u32;
    let height = (size.height() * scale) as u32;

    // Render to pixmap
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or("Failed to create pixmap")?;

    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());

    // Save as PNG
    pixmap.save_png(cache_path)
        .map_err(|e| format!("Failed to save PNG: {}", e))?;

    Ok(cache_path.to_path_buf())
}

/// Normalize raster images (PNG/JPG/GIF/BMP/WebP) to 256x256 PNG
fn normalize_raster_to_cache(source_path: &Path, cache_path: &Path) -> Result<PathBuf, String> {
    // Load image with the image crate
    let img = image::open(source_path)
        .map_err(|e| format!("Failed to load image: {}", e))?;

    // Get current dimensions
    let (width, height) = img.dimensions();

    // Scale down if larger than 256x256 (maintain aspect ratio)
    let scaled_img = if width > 256 || height > 256 {
        let scale = 256.0 / width.max(height) as f32;
        let new_width = (width as f32 * scale) as u32;
        let new_height = (height as f32 * scale) as u32;
        img.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    // Convert to RGBA and save as PNG
    let rgba = scaled_img.to_rgba8();
    rgba.save(cache_path)
        .map_err(|e| format!("Failed to save PNG: {}", e))?;

    Ok(cache_path.to_path_buf())
}

/// Check if ImageMagick convert command is available
fn is_imagemagick_available() -> bool {
    Command::new("convert")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Normalize XPM to 256x256 PNG using ImageMagick
fn normalize_xpm_to_cache(source_path: &Path, cache_path: &Path) -> Result<PathBuf, String> {
    // Check if convert is available first
    if !is_imagemagick_available() {
        return Err("ImageMagick 'convert' command not found. Please install ImageMagick to use XPM icons.".to_string());
    }

    // Use ImageMagick to convert and resize in one step
    let status = Command::new("convert")
        .arg(source_path)
        .arg("-resize")
        .arg("256x256>") // Only shrink if larger, maintain aspect ratio
        .arg(cache_path)
        .status();

    match status {
        Ok(status) if status.success() => Ok(cache_path.to_path_buf()),
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

/// Cache and normalize all icons to 256x256 PNG
fn convert_if_needed(path: &Path) -> Option<String> {
    // Always cache and normalize ALL formats (including PNG/JPG)
    // This ensures consistent size and location
    match cache_and_normalize_icon(path) {
        Ok(cached_path) => Some(cached_path.to_string_lossy().to_string()),
        Err(_) => {
            // Caching/normalization failed - skip this icon
            None
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
