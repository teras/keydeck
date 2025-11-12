// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;

/// Creates a backup of the entire ~/.config/keydeck/ directory as a ZIP file.
///
/// # Behavior
/// - Includes: config.yaml, devices/ folder, symlinks (with absolute/relative paths preserved), and all other files
/// - Excludes: config.TIMESTAMP.yaml (timestamped backups), .service_prompt_count (internal state)
/// - Symlinks are stored as symlinks in the ZIP with proper Unix file type bits
///
/// # Errors
/// Returns an error if the configuration directory doesn't exist or if ZIP creation fails
pub fn backup_config_directory(zip_path: &str) -> Result<(), String> {
    let config_dir = get_config_dir_path()?;

    // Check if config directory exists
    if !config_dir.exists() {
        return Err("Configuration directory does not exist yet. Nothing to backup.".to_string());
    }

    // Create zip file
    let file = File::create(zip_path)
        .map_err(|e| format!("Failed to create zip file: {}", e))?;
    let mut zip = ZipWriter::new(file);

    let options: FileOptions<()> = FileOptions::default()
        .compression_method(CompressionMethod::Deflated);

    // Walk through the config directory
    for entry in WalkDir::new(&config_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let relative_path = path.strip_prefix(&config_dir)
            .map_err(|e| format!("Failed to get relative path: {}", e))?;

        // Skip the root directory itself
        if relative_path.as_os_str().is_empty() {
            continue;
        }

        // Skip backup config files and internal state files
        if let Some(filename) = path.file_name() {
            let filename_str = filename.to_string_lossy();

            // Skip timestamped config backup files (config.TIMESTAMP.yaml)
            if filename_str.starts_with("config.")
                && filename_str.ends_with(".yaml")
                && filename_str != "config.yaml" {
                continue;
            }

            // Skip internal state files
            if filename_str == ".service_prompt_count" {
                continue;
            }
        }

        let name = relative_path.to_string_lossy().to_string();

        // Check if it's a symlink
        let metadata = fs::symlink_metadata(path)
            .map_err(|e| format!("Failed to read metadata: {}", e))?;

        if metadata.is_symlink() {
            // Store symlink as symlink in ZIP
            let target = fs::read_link(path)
                .map_err(|e| format!("Failed to read symlink target: {}", e))?;

            // Convert target to string to preserve it exactly as-is
            let target_str = target.to_str()
                .ok_or_else(|| format!("Invalid symlink target path encoding"))?;

            // Use add_symlink to manually write the symlink with exact target
            zip.add_symlink(&name, target_str, options)
                .map_err(|e| format!("Failed to add symlink to zip: {}", e))?;
        } else if path.is_file() {
            // Add file to zip
            zip.start_file(&name, options)
                .map_err(|e| format!("Failed to start file in zip: {}", e))?;

            let mut f = File::open(path)
                .map_err(|e| format!("Failed to open file: {}", e))?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)
                .map_err(|e| format!("Failed to read file: {}", e))?;

            zip.write_all(&buffer)
                .map_err(|e| format!("Failed to write file to zip: {}", e))?;
        } else if path.is_dir() {
            // Add directory to zip (with trailing slash)
            zip.add_directory(&name, options)
                .map_err(|e| format!("Failed to add directory to zip: {}", e))?;
        }
    }

    zip.finish()
        .map_err(|e| format!("Failed to finalize zip: {}", e))?;

    Ok(())
}

/// Restores the ~/.config/keydeck/ directory from a ZIP backup.
///
/// # Behavior
/// - Extracts all files from the ZIP archive to ~/.config/keydeck/
/// - Overwrites existing files and directories
/// - Creates the config directory if it doesn't exist
/// - Properly restores symlinks with their original targets (absolute or relative)
/// - Removes conflicting files/directories before extraction
///
/// # Errors
/// Returns an error if the ZIP file cannot be read or extraction fails
pub fn restore_config_directory(zip_path: &str) -> Result<(), String> {
    let config_dir = get_config_dir_path()?;

    // Ensure the config directory exists (create it if needed)
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    // Open zip file
    let file = File::open(zip_path)
        .map_err(|e| format!("Failed to open zip file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip archive: {}", e))?;

    // Extract all files
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Failed to access file in zip: {}", e))?;

        let outpath = config_dir.join(file.name());

        // Check if this is a symlink (Unix permissions start with 0o120xxx)
        let is_symlink = if let Some(mode) = file.unix_mode() {
            (mode & 0o170000) == 0o120000
        } else {
            false
        };

        if file.is_dir() {
            // Create directory
            fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        } else if is_symlink {
            // Restore symlink
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| format!("Failed to read symlink target from zip: {}", e))?;
            let target = String::from_utf8(buffer)
                .map_err(|e| format!("Invalid symlink target encoding: {}", e))?;

            // Create parent directory if needed
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directory: {}", e))?;
            }

            // Remove existing file/symlink/directory if it exists
            if outpath.exists() {
                let existing_metadata = fs::symlink_metadata(&outpath)
                    .map_err(|e| format!("Failed to read existing path metadata: {}", e))?;

                if existing_metadata.is_dir() {
                    fs::remove_dir_all(&outpath)
                        .map_err(|e| format!("Failed to remove existing directory: {}", e))?;
                } else {
                    fs::remove_file(&outpath)
                        .map_err(|e| format!("Failed to remove existing file: {}", e))?;
                }
            } else if outpath.is_symlink() {
                // Handle broken symlinks (exists() returns false but is_symlink() returns true)
                fs::remove_file(&outpath)
                    .map_err(|e| format!("Failed to remove existing symlink: {}", e))?;
            }

            // Create symlink
            std::os::unix::fs::symlink(&target, &outpath)
                .map_err(|e| format!("Failed to create symlink: {}", e))?;
        } else {
            // Extract regular file
            // Create parent directory if needed
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directory: {}", e))?;
            }

            // Remove existing directory if it exists (replace with file)
            if outpath.exists() {
                let existing_metadata = fs::symlink_metadata(&outpath)
                    .map_err(|e| format!("Failed to read existing path metadata: {}", e))?;

                if existing_metadata.is_dir() {
                    fs::remove_dir_all(&outpath)
                        .map_err(|e| format!("Failed to remove existing directory: {}", e))?;
                }
            }

            // Extract file
            let mut outfile = File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| format!("Failed to read from zip: {}", e))?;
            outfile.write_all(&buffer)
                .map_err(|e| format!("Failed to write file: {}", e))?;
        }
    }

    Ok(())
}

/// Gets the keydeck config directory path (~/.config/keydeck/)
/// Returns the path without requiring it to exist (will be created if needed)
fn get_config_dir_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME")
        .map_err(|_| "HOME environment variable not set".to_string())?;
    let config_dir = Path::new(&home).join(".config").join("keydeck");

    Ok(config_dir)
}
