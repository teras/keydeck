// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! Discovery of installed macOS applications and extraction of their icons.
//!
//! Applications are `.app` bundles found under the standard install roots.
//! Each bundle's display name comes from its `Info.plist`, and its icon is the
//! `.icns` file referenced by `CFBundleIconFile` (or the first `.icns` in
//! `Contents/Resources`). Because browsers can't render `.icns` directly, the
//! icon is decoded to a PNG in a per-user cache and that PNG path is what the
//! rest of the app consumes (matching the Linux/Windows finders).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

/// Represents a macOS application with its name and (decoded PNG) icon path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub icon_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_data_url: Option<String>,
}

/// Cache for the application list to avoid repeated filesystem scanning.
static APP_CACHE: LazyLock<Mutex<Option<Vec<AppInfo>>>> = LazyLock::new(|| Mutex::new(None));

/// Directories scanned for `.app` bundles.
fn app_search_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/Applications/Utilities"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join("Applications"));
    }
    dirs
}

/// Directory where decoded PNG icons are cached.
fn icon_cache_dir() -> PathBuf {
    keydeck_types::get_config_dir().join(".app_icon_cache")
}

/// Find all installed applications, decoding each icon to a cached PNG.
pub fn find_applications() -> Result<Vec<AppInfo>, String> {
    // Check cache first
    {
        let cache = APP_CACHE.lock().unwrap();
        if let Some(apps) = cache.as_ref() {
            return Ok(apps.clone());
        }
    }

    let cache_dir = icon_cache_dir();
    let _ = fs::create_dir_all(&cache_dir);

    let mut apps: Vec<AppInfo> = Vec::new();
    let mut seen = HashSet::new();

    for dir in app_search_dirs() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue, // directory may not exist; that's fine
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("app") {
                continue;
            }

            let name = app_display_name(&path);
            if !seen.insert(name.clone()) {
                continue; // skip duplicate app names
            }

            // Skip apps whose icon can't be located/decoded rather than failing.
            if let Ok(png_path) = extract_app_icon_png(&path, &name, &cache_dir) {
                apps.push(AppInfo {
                    name,
                    icon_path: png_path.to_string_lossy().into_owned(),
                    icon_data_url: None,
                });
            }
        }
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    *APP_CACHE.lock().unwrap() = Some(apps.clone());
    Ok(apps)
}

/// Copy a previously-decoded PNG icon into the keydeck icon directory,
/// returning the stored filename.
pub fn copy_app_icon(
    app_name: String,
    icon_path: String,
    icon_dir: String,
) -> Result<String, String> {
    let sanitized_name = sanitize_filename(&app_name);

    fs::create_dir_all(&icon_dir).map_err(|e| format!("Failed to create icon directory: {}", e))?;

    let source_path = PathBuf::from(&icon_path);
    if !source_path.exists() {
        return Err(format!("Icon file not found: {}", icon_path));
    }

    let icon_dir_path = PathBuf::from(&icon_dir);
    let mut output_filename = format!("{}.png", sanitized_name);
    let mut counter = 2;
    while icon_dir_path.join(&output_filename).exists() {
        output_filename = format!("{}_{}.png", sanitized_name, counter);
        counter += 1;
    }

    fs::copy(&source_path, icon_dir_path.join(&output_filename))
        .map_err(|e| format!("Failed to copy icon file: {}", e))?;

    Ok(output_filename)
}

/// Read the human-readable app name from `Info.plist`, falling back to the
/// bundle filename without the `.app` extension.
fn app_display_name(app: &Path) -> String {
    let plist_path = app.join("Contents/Info.plist");
    if let Ok(value) = plist::Value::from_file(&plist_path) {
        if let Some(dict) = value.as_dictionary() {
            for key in ["CFBundleDisplayName", "CFBundleName"] {
                if let Some(s) = dict.get(key).and_then(|v| v.as_string()) {
                    if !s.trim().is_empty() {
                        return s.to_string();
                    }
                }
            }
        }
    }

    app.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string()
}

/// Locate the `.icns` icon file inside a `.app` bundle.
fn locate_icns(app: &Path) -> Result<PathBuf, String> {
    let resources = app.join("Contents/Resources");

    // Preferred: the file named by CFBundleIconFile.
    let plist_path = app.join("Contents/Info.plist");
    if let Ok(value) = plist::Value::from_file(&plist_path) {
        if let Some(icon) = value
            .as_dictionary()
            .and_then(|d| d.get("CFBundleIconFile"))
            .and_then(|v| v.as_string())
        {
            let mut p = resources.join(icon);
            if p.extension().is_none() {
                p.set_extension("icns");
            }
            if p.exists() {
                return Ok(p);
            }
        }
    }

    // Fallback: the first `.icns` in Resources.
    if let Ok(entries) = fs::read_dir(&resources) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("icns") {
                return Ok(p);
            }
        }
    }

    Err(format!("No .icns icon found in {}", app.display()))
}

/// Decode the app's `.icns` icon to a PNG in the cache directory and return
/// the PNG path. Reuses an already-decoded cache entry when present.
fn extract_app_icon_png(app: &Path, name: &str, cache_dir: &Path) -> Result<PathBuf, String> {
    let out = cache_dir.join(format!("{}.png", sanitize_filename(name)));
    if out.exists() {
        return Ok(out);
    }

    let icns_path = locate_icns(app)?;

    let file = fs::File::open(&icns_path).map_err(|e| format!("Failed to open icns: {}", e))?;
    let family = icns::IconFamily::read(std::io::BufReader::new(file))
        .map_err(|e| format!("Failed to read icns: {}", e))?;

    // Pick the largest available icon type for the best quality.
    let icon_type = family
        .available_icons()
        .into_iter()
        .max_by_key(|t| t.pixel_width() * t.pixel_height())
        .ok_or_else(|| "icns contains no icons".to_string())?;

    let image = family
        .get_icon_with_type(icon_type)
        .map_err(|e| format!("Failed to decode icns icon: {}", e))?;

    let outfile = fs::File::create(&out).map_err(|e| format!("Failed to create PNG: {}", e))?;
    image
        .write_png(std::io::BufWriter::new(outfile))
        .map_err(|e| format!("Failed to write PNG: {}", e))?;

    Ok(out)
}

/// Sanitize an application name to create a valid filename.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
}
