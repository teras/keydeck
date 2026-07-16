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
    keydeck_types::get_config_path()
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
    ) -> Result<
        (
            HashMap<String, ButtonConfig>,
            Option<Vec<Action>>,
            Option<bool>,
            Option<IndexMap<String, crate::pages::Encoder>>,
        ),
        String,
    > {
        // Check for circular inheritance
        if visited.contains(&template_name.to_string()) {
            visited.push(template_name.to_string());
            let cycle_path = visited.join(" → ");
            return Err(format!(
                "Circular template inheritance detected: {}",
                cycle_path
            ));
        }

        // Get the template
        let template = templates
            .get(template_name)
            .ok_or_else(|| format!("Template '{}' not found", template_name))?;

        // Mark as visited for cycle detection
        visited.push(template_name.to_string());

        let mut merged_buttons = HashMap::new();
        let mut merged_on_tick: Option<Vec<Action>> = None;
        let mut merged_lock: Option<bool> = None;
        let mut merged_encoders: Option<IndexMap<String, crate::pages::Encoder>> = None;

        // First, recursively resolve parent templates
        if let Some(parent_templates) = &template.inherits {
            for parent_name in parent_templates {
                let (parent_buttons, parent_on_tick, parent_lock, parent_encoders) =
                    Self::resolve_template_recursive(parent_name, templates, visited)?;
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
                // Merge parent encoders (later parents override earlier ones)
                if let Some(parent_enc) = parent_encoders {
                    merged_encoders
                        .get_or_insert_with(IndexMap::new)
                        .extend(parent_enc);
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

        // Merge this template's encoders (overriding parent encoders)
        if let Some(template_enc) = &template.encoders {
            merged_encoders
                .get_or_insert_with(IndexMap::new)
                .extend(template_enc.clone());
        }

        // Remove from visited (backtrack for DFS)
        visited.pop();

        Ok((merged_buttons, merged_on_tick, merged_lock, merged_encoders))
    }

    /// Load and fully resolve the configuration, exiting the process on any error.
    ///
    /// Use this only for the initial startup load, where an invalid config means
    /// the daemon has nothing to run. For live reloads (SIGHUP) use [`try_load`],
    /// which reports the error instead of killing a running daemon.
    ///
    /// [`try_load`]: Self::try_load
    pub fn load() -> KeyDeckConf {
        Self::try_load().unwrap_or_else(|e| {
            eprintln!("{}", e);
            std::process::exit(1);
        })
    }

    /// Load and fully resolve the configuration, returning a descriptive error
    /// string instead of terminating the process. The error is a ready-to-print,
    /// possibly multi-line message.
    pub fn try_load() -> Result<KeyDeckConf, String> {
        let path = get_default_config_path();

        // Check if file exists, create empty file if not
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "Error: Failed to create config directory at {}\nReason: {}",
                        parent.display(),
                        e
                    )
                })?;
            }
            fs::write(&path, "").map_err(|e| {
                format!(
                    "Error: Failed to create empty config file at {}\nReason: {}",
                    path.display(),
                    e
                )
            })?;
        }

        let data = fs::read_to_string(&path).map_err(|e| {
            format!(
                "Error: Failed to read config file at {}\nReason: {}\n\nPlease create a config file at ~/.config/keydeck/config.yaml\nSee the documentation for configuration format.",
                path.display(),
                e
            )
        })?;

        // If the file is empty, use default config
        let mut conf: KeyDeckConf = if data.trim().is_empty() {
            KeyDeckConf::default()
        } else {
            let deserializer = serde_yaml_ng::Deserializer::from_str(&data);
            serde_path_to_error::deserialize(deserializer).map_err(|e| {
                let err_path = e.path().to_string();
                format!(
                    "Error parsing config file: {}\n\nPath: {}\n{}",
                    path.display(),
                    err_path,
                    e.into_inner()
                )
            })?
        };

        // Validate tick_time is within range (1-60 seconds)
        if conf.tick_time < 1.0 || conf.tick_time > 60.0 {
            return Err(format!(
                "Error: tick_time must be between 1 and 60 seconds\nCurrent value: {}\n\nPlease update your config file at {}",
                conf.tick_time,
                path.display()
            ));
        }

        // Upgrade legacy `window_name` into the unified `when` structure.
        conf.migrate_legacy_window_name();

        // Validate that templates don't have auto-switch conditions (only valid for pages)
        if let Some(templates) = &conf.templates {
            for (template_name, template) in templates {
                if template.when.is_some() {
                    return Err(format!(
                        "Error: Template '{}' has a 'when' (or legacy 'window_name') field\nAuto-switch conditions are only valid in pages, not templates.\nTemplates are never directly displayed, so window/context matching doesn't apply.\n\nPlease remove the 'when'/'window_name' field from template '{}'\nConfig file: {}",
                        template_name,
                        template_name,
                        path.display()
                    ));
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
                        match Self::resolve_template_recursive(
                            template_name,
                            templates_map,
                            &mut visited,
                        ) {
                            Ok((resolved_buttons, resolved_on_tick, resolved_lock, resolved_encoders)) => {
                                // Merge resolved buttons into page (page buttons take priority)
                                for (button_name, button_config) in resolved_buttons {
                                    page.buttons.entry(button_name).or_insert(button_config);
                                }
                                // Merge on_tick (page's on_tick takes priority over template's)
                                if page.on_tick.is_none() && resolved_on_tick.is_some() {
                                    page.on_tick = resolved_on_tick;
                                }
                                // Merge lock (page's lock takes priority over template's)
                                if page.lock.is_none() && resolved_lock.is_some() {
                                    page.lock = resolved_lock;
                                }
                                // Merge encoders (page's encoders take priority over template's)
                                if let Some(resolved_enc) = resolved_encoders {
                                    let page_encoders = page.encoders.get_or_insert_with(IndexMap::new);
                                    for (enc_name, enc_config) in resolved_enc {
                                        page_encoders.entry(enc_name).or_insert(enc_config);
                                    }
                                }
                            }
                            Err(e) => {
                                return Err(format!(
                                    "Error resolving templates for page '{}': {}\n\nPlease check your template inheritance configuration.",
                                    page_name, e
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Compute dynamic flags for all buttons after template resolution
        crate::dynamic_detection::compute_all_dynamic_flags(&mut conf);

        Ok(conf)
    }
}
