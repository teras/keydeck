// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

// Backend-specific implementation for KeyDeckConf
// Type definitions are in keydeck-types crate

use indexmap::IndexMap;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Re-export types from keydeck-types
pub use keydeck_types::*;

fn get_default_config_path() -> PathBuf {
    let mut path = PathBuf::from(std::env::var("HOME").expect("Could not find home directory"));
    path.push(".config/keydeck/config.yaml");
    path
}

/// Backend-specific configuration loader
pub struct KeyDeckConfLoader;

impl KeyDeckConfLoader {
    /// Recursively resolves a template and all its parent templates, with cycle detection.
    /// Returns merged buttons, on_tick actions, and lock value in parent-first order (grandparent -> parent -> child).
    pub fn resolve_template_recursive(
        template_name: &str,
        templates: &IndexMap<String, Page>,
        visited: &mut Vec<String>,
    ) -> Result<(HashMap<String, ButtonConfig>, Option<Vec<Action>>, Option<bool>), String> {
        // Check for circular inheritance
        if visited.contains(&template_name.to_string()) {
            visited.push(template_name.to_string());
            let cycle_path = visited.join(" → ");
            return Err(format!("Circular template inheritance detected: {}", cycle_path));
        }

        // Get the template
        let template = templates.get(template_name).ok_or_else(|| {
            format!("Template '{}' not found", template_name)
        })?;

        // Mark as visited for cycle detection
        visited.push(template_name.to_string());

        let mut merged_buttons = HashMap::new();
        let mut merged_on_tick: Option<Vec<Action>> = None;
        let mut merged_lock: Option<bool> = None;

        // First, recursively resolve parent templates
        if let Some(parent_templates) = &template.inherits {
            for parent_name in parent_templates {
                let (parent_buttons, parent_on_tick, parent_lock) = Self::resolve_template_recursive(
                    parent_name,
                    templates,
                    visited,
                )?;
                // Merge parent buttons (later parents override earlier ones)
                merged_buttons.extend(parent_buttons);
                // on_tick is overridden by later parents (not merged)
                if parent_on_tick.is_some() {
                    merged_on_tick = parent_on_tick;
                }
                // lock is overridden by later parents (not merged)
                if parent_lock.is_some() {
                    merged_lock = parent_lock;
                }
            }
        }

        // Then merge this template's buttons (overriding parent buttons)
        merged_buttons.extend(template.buttons.clone());

        // on_tick is overridden by child (not merged)
        if template.on_tick.is_some() {
            merged_on_tick = template.on_tick.clone();
        }

        // lock is overridden by child (not merged)
        if template.lock.is_some() {
            merged_lock = template.lock;
        }

        // Remove from visited (backtrack for DFS)
        visited.pop();

        Ok((merged_buttons, merged_on_tick, merged_lock))
    }

    pub fn load() -> KeyDeckConf {
        let path = get_default_config_path();

        // Check if file exists, create empty file if not
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap_or_else(|e| {
                    eprintln!("Error: Failed to create config directory at {}", parent.display());
                    eprintln!("Reason: {}", e);
                    std::process::exit(1);
                });
            }
            fs::write(&path, "").unwrap_or_else(|e| {
                eprintln!("Error: Failed to create empty config file at {}", path.display());
                eprintln!("Reason: {}", e);
                std::process::exit(1);
            });
        }

        let data = fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("Error: Failed to read config file at {}", path.display());
            eprintln!("Reason: {}", e);
            eprintln!("\nPlease create a config file at ~/.config/keydeck/config.yaml");
            eprintln!("See the documentation for configuration format.");
            std::process::exit(1);
        });

        let deserializer = serde_yaml_ng::Deserializer::from_str(&data);
        let mut conf: KeyDeckConf = serde_path_to_error::deserialize(deserializer).unwrap_or_else(|e| {
            eprintln!("Error parsing config file: {}", path.display());
            eprintln!();
            eprintln!("Path: {}", e.path());
            eprintln!("{}", e.into_inner());
            std::process::exit(1);
        });

        // Validate tick_time is within range (1-60 seconds)
        if conf.tick_time < 1.0 || conf.tick_time > 60.0 {
            eprintln!("Error: tick_time must be between 1 and 60 seconds");
            eprintln!("Current value: {}", conf.tick_time);
            eprintln!("\nPlease update your config file at {}", path.display());
            std::process::exit(1);
        }

        // Validate that templates don't have window_name (only valid for pages)
        if let Some(templates) = &conf.templates {
            for (template_name, template) in templates {
                if template.window_name.is_some() {
                    eprintln!("Error: Template '{}' has 'window_name' field", template_name);
                    eprintln!("The 'window_name' field is only valid in pages, not templates.");
                    eprintln!("Templates are never directly displayed, so window matching doesn't apply.");
                    eprintln!("\nPlease remove the 'window_name' field from template '{}'", template_name);
                    eprintln!("Config file: {}", path.display());
                    std::process::exit(1);
                }
            }
        }

        // Resolve template inheritance for all pages
        let empty_templates = IndexMap::new();
        for (_, pages) in &mut conf.page_groups {
            for (page_name, page) in &mut pages.pages {
                // Recursively resolve all inherited templates
                if let Some(template_names) = &page.inherits {
                    let templates_map = conf.templates.as_ref().unwrap_or(&empty_templates);

                    for template_name in template_names {
                        let mut visited = Vec::new();
                        match Self::resolve_template_recursive(template_name, templates_map, &mut visited) {
                            Ok((resolved_buttons, resolved_on_tick, resolved_lock)) => {
                                // Merge resolved buttons into page (page buttons take priority)
                                for (button_name, button_config) in resolved_buttons {
                                    page.buttons
                                        .entry(button_name)
                                        .or_insert(button_config);
                                }
                                // Merge on_tick (page's on_tick takes priority over template's)
                                if page.on_tick.is_none() && resolved_on_tick.is_some() {
                                    page.on_tick = resolved_on_tick;
                                }
                                // Merge lock (page's lock takes priority over template's)
                                if page.lock.is_none() && resolved_lock.is_some() {
                                    page.lock = resolved_lock;
                                }
                            }
                            Err(e) => {
                                eprintln!("Error resolving templates for page '{}': {}", page_name, e);
                                eprintln!("\nPlease check your template inheritance configuration.");
                                std::process::exit(1);
                            }
                        }
                    }
                }
            }
        }

        // Compute dynamic flags for all buttons after template resolution
        crate::dynamic_detection::compute_all_dynamic_flags(&mut conf);

        conf
    }
}
