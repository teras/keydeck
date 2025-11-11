// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::pages::{ButtonConfig, KeyDeckConf};
use crate::{error_log, info_log, verbose_log, warn_log};
use keydeck::get_icon_dir;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize)]
struct ValidationResult {
    success: bool,
    config_path: String,
    summary: Option<ConfigSummary>,
    errors: Vec<ValidationError>,
    warnings: Vec<ValidationWarning>,
    unreferenced_icons: Vec<String>,
    services_tested: Vec<ServiceTestResult>,
}

#[derive(Serialize)]
struct ConfigSummary {
    page_groups: usize,
    total_pages: usize,
    button_definitions: usize,
    macros: usize,
    services: usize,
    colors: usize,
    image_dir: String,
    tick_time: f64,
}

#[derive(Serialize)]
struct ValidationError {
    category: String,
    message: String,
}

#[derive(Serialize)]
struct ValidationWarning {
    category: String,
    message: String,
}

#[derive(Serialize)]
struct ServiceTestResult {
    name: String,
    success: bool,
    output: Option<String>,
    error: Option<String>,
}

/// Validates a keydeck configuration file.
/// This performs a full load of the YAML including:
/// - YAML syntax validation
/// - Schema validation (serde deserialization)
/// - Template inheritance resolution
/// - Macro syntax validation (parameter substitution patterns)
/// - Service validation (actually runs them to test)
/// - Action syntax validation (parses but doesn't execute)
///
/// Returns true if validation succeeds, false otherwise.
pub fn validate_config(config_path: &str, json_output: bool) -> bool {
    let mut result = ValidationResult {
        success: true,
        config_path: config_path.to_string(),
        summary: None,
        errors: Vec::new(),
        warnings: Vec::new(),
        unreferenced_icons: Vec::new(),
        services_tested: Vec::new(),
    };

    if !json_output {
        info_log!("Validating keydeck configuration: {}", config_path);
    }

    // Load the configuration file
    let path = PathBuf::from(config_path);
    let data = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(e) => {
            error_log!("Error: Failed to read config file at {}", path.display());
            error_log!("Reason: {}", e);
            return false;
        }
    };

    // Parse the YAML
    let deserializer = serde_yaml_ng::Deserializer::from_str(&data);
    let mut conf: KeyDeckConf = match serde_path_to_error::deserialize(deserializer) {
        Ok(conf) => conf,
        Err(e) => {
            eprintln!("Error parsing config file: {}", path.display());
            eprintln!();
            eprintln!("Path: {}", e.path());
            eprintln!("{}", e.into_inner());
            return false;
        }
    };

    // Validate tick_time is within range (1-60 seconds)
    if conf.tick_time < 1.0 || conf.tick_time > 60.0 {
        eprintln!("Error: tick_time must be between 1 and 60 seconds");
        eprintln!("Current value: {}", conf.tick_time);
        return false;
    }

    // Validate that templates don't have window_name (only valid for pages)
    if let Some(templates) = &conf.templates {
        for (template_name, template) in templates {
            if template.window_name.is_some() {
                eprintln!("Error: Template '{}' has 'window_name' field", template_name);
                eprintln!("The 'window_name' field is only valid in pages, not templates.");
                eprintln!("Templates are never directly displayed, so window matching doesn't apply.");
                eprintln!("\nPlease remove the 'window_name' field from template '{}'", template_name);
                return false;
            }
        }
    }

    // Resolve template inheritance for all pages
    let empty_templates = indexmap::IndexMap::new();
    for (_, pages) in &mut conf.page_groups {
        for (page_name, page) in &mut pages.pages {
            if let Some(template_names) = &page.inherits {
                let templates_map = conf.templates.as_ref().unwrap_or(&empty_templates);

                for template_name in template_names {
                    let mut visited = Vec::new();
                    match KeyDeckConf::resolve_template_recursive(template_name, templates_map, &mut visited) {
                        Ok((resolved_buttons, resolved_on_tick, resolved_lock)) => {
                            for (button_name, button_config) in resolved_buttons {
                                page.buttons
                                    .entry(button_name)
                                    .or_insert(button_config);
                            }
                            if page.on_tick.is_none() && resolved_on_tick.is_some() {
                                page.on_tick = resolved_on_tick;
                            }
                            if page.lock.is_none() && resolved_lock.is_some() {
                                page.lock = resolved_lock;
                            }
                        }
                        Err(e) => {
                            eprintln!("Error resolving templates for page '{}': {}", page_name, e);
                            return false;
                        }
                    }
                }
            }
        }
    }

    // Compute dynamic flags for all buttons after template resolution
    crate::dynamic_detection::compute_all_dynamic_flags(&mut conf);

    // Collect summary information
    let button_def_count = conf.buttons.as_ref().map(|b| b.len()).unwrap_or(0);
    let macro_count = conf.macros.as_ref().map(|m| m.len()).unwrap_or(0);
    let service_count = conf.services.as_ref().map(|s| s.len()).unwrap_or(0);
    let color_count = conf.colors.as_ref().map(|c| c.len()).unwrap_or(0);

    let mut total_pages = 0;
    for (_group_name, page_group) in &conf.page_groups {
        total_pages += page_group.pages.len();
    }

    if !json_output {
        info_log!("Configuration parsed successfully!");
        info_log!("  Page groups: {}", conf.page_groups.len());
        info_log!("  Total pages: {}", total_pages);
        info_log!("  Button definitions: {}", button_def_count);
        info_log!("  Macros: {}", macro_count);
        info_log!("  Services: {}", service_count);
        info_log!("  Colors: {}", color_count);
        info_log!("  Image directory: {}", get_icon_dir());
        info_log!("  Tick time: {}s", conf.tick_time);
    }

    // Validate page references (main_page, jump targets, etc.)
    validate_page_references(&conf, &mut result, json_output);

    // Validate macro syntax (parameter substitution patterns)
    validate_macro_syntax(&conf, &mut result);

    // Validate service references and test execution
    validate_services(&conf, &mut result);

    // Validate button definition references
    validate_button_def_references(&conf, &mut result);

    // Validate icon file existence
    validate_icon_files(&conf, &mut result, json_output);

    // Populate summary
    result.summary = Some(ConfigSummary {
        page_groups: conf.page_groups.len(),
        total_pages,
        button_definitions: button_def_count,
        macros: macro_count,
        services: service_count,
        colors: color_count,
        image_dir: get_icon_dir(),
        tick_time: conf.tick_time,
    });

    result.success = result.errors.is_empty();

    // Output results
    if json_output {
        // Output as JSON
        match serde_json::to_string_pretty(&result) {
            Ok(json) => { println!("{}", json); },  // JSON output - no prefix
            Err(e) => {
                eprintln!("Error serializing validation results to JSON: {}", e);
                return false;
            }
        }
    } else {
        // Regular output
        if result.success {
            info_log!("✓ All validations passed!");
        } else {
            error_log!("✗ Validation failed with errors");
        }
    }

    result.success
}

/// Validates macro syntax - checks parameter substitution patterns
fn validate_macro_syntax(conf: &KeyDeckConf, result: &mut ValidationResult) {
    verbose_log!("Validating macro syntax...");

    let Some(macros) = &conf.macros else {
        verbose_log!("  No macros defined");
        return;
    };

    let param_pattern = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();

    for (macro_name, macro_def) in macros {
        verbose_log!("  Checking macro '{}'", macro_name);

        // Check if macro has default parameters defined
        let default_params: HashSet<String> = macro_def.params
            .as_ref()
            .map(|p| p.keys().cloned().collect())
            .unwrap_or_default();

        // Scan the raw YAML value for parameter references
        let mut used_params = HashSet::new();
        scan_yaml_for_params(&macro_def.actions, &param_pattern, &mut used_params);

        // Check for undefined parameters (used but not in defaults)
        for param in &used_params {
            if !default_params.contains(param) {
                let msg = format!("Macro '{}' uses parameter '{}' but doesn't define a default value",
                         macro_name, param);
                warn_log!("{}", msg);
                result.warnings.push(ValidationWarning {
                    category: "macro".to_string(),
                    message: msg,
                });
            }
        }
    }
}

/// Helper to scan YAML value recursively for parameter references like ${param}
fn scan_yaml_for_params(value: &serde_yaml_ng::Value, pattern: &regex::Regex, params: &mut HashSet<String>) {
    match value {
        serde_yaml_ng::Value::String(s) => {
            for cap in pattern.captures_iter(s) {
                params.insert(cap[1].to_string());
            }
        }
        serde_yaml_ng::Value::Sequence(seq) => {
            for item in seq {
                scan_yaml_for_params(item, pattern, params);
            }
        }
        serde_yaml_ng::Value::Mapping(map) => {
            for (_key, val) in map {
                scan_yaml_for_params(val, pattern, params);
            }
        }
        _ => {}
    }
}

/// Validates services by actually running them
fn validate_services(conf: &KeyDeckConf, result: &mut ValidationResult) {
    verbose_log!("Validating services...");

    let Some(services) = &conf.services else {
        verbose_log!("  No services defined");
        return;
    };

    // Test each service by running it once
    for (service_name, service_config) in services {
        verbose_log!("  Testing service '{}'...", service_name);

        let cmd = &service_config.exec;
        let timeout = service_config.timeout;

        // Run the command with timeout
        let output = std::process::Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        match output {
            Ok(mut child) => {
                // Wait for the command with timeout
                let start = std::time::Instant::now();
                loop {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            if status.success() {
                                let output = child.wait_with_output().unwrap();
                                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                                verbose_log!("    ✓ Success: {}", stdout);
                                result.services_tested.push(ServiceTestResult {
                                    name: service_name.clone(),
                                    success: true,
                                    output: Some(stdout),
                                    error: None,
                                });
                            } else {
                                let msg = format!("Service '{}' exited with status: {}", service_name, status);
                                eprintln!("Error: {}", msg);
                                result.errors.push(ValidationError {
                                    category: "service".to_string(),
                                    message: msg.clone(),
                                });
                                result.services_tested.push(ServiceTestResult {
                                    name: service_name.clone(),
                                    success: false,
                                    output: None,
                                    error: Some(msg),
                                });
                            }
                            break;
                        }
                        Ok(None) => {
                            // Still running - check timeout (if specified)
                            if let Some(timeout_val) = timeout {
                                if start.elapsed().as_secs_f64() > timeout_val {
                                    let _ = child.kill();
                                    let msg = format!("Service '{}' timed out after {}s", service_name, timeout_val);
                                    eprintln!("Error: {}", msg);
                                result.errors.push(ValidationError {
                                    category: "service".to_string(),
                                    message: msg.clone(),
                                });
                                result.services_tested.push(ServiceTestResult {
                                    name: service_name.clone(),
                                    success: false,
                                    output: None,
                                    error: Some(msg),
                                });
                                    break;
                                }
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        Err(e) => {
                            let msg = format!("Failed to wait for service '{}': {}", service_name, e);
                            eprintln!("Error: {}", msg);
                            result.errors.push(ValidationError {
                                category: "service".to_string(),
                                message: msg.clone(),
                            });
                            result.services_tested.push(ServiceTestResult {
                                name: service_name.clone(),
                                success: false,
                                output: None,
                                error: Some(msg),
                            });
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                let msg = format!("Failed to execute service '{}': {}", service_name, e);
                eprintln!("Error: {}", msg);
                result.errors.push(ValidationError {
                    category: "service".to_string(),
                    message: msg.clone(),
                });
                result.services_tested.push(ServiceTestResult {
                    name: service_name.clone(),
                    success: false,
                    output: None,
                    error: Some(msg),
                });
            }
        }
    }
}

/// Validates that all button definition references exist
fn validate_button_def_references(conf: &KeyDeckConf, result: &mut ValidationResult) {
    verbose_log!("Validating button definition references...");

    let button_defs = conf.buttons.as_ref();
    let mut referenced_button_defs = HashSet::new();

    // Collect all button definition references
    for (_group_name, page_group) in &conf.page_groups {
        for (_page_name, page) in &page_group.pages {
            for (_button_key, button_config) in &page.buttons {
                // Check if this is a button definition reference
                if let ButtonConfig::Template(ref_name) = button_config {
                    referenced_button_defs.insert(ref_name.clone());
                }
            }
        }
    }

    // Check if referenced button definitions exist
    for button_def_name in referenced_button_defs {
        if let Some(button_defs_map) = button_defs {
            if !button_defs_map.contains_key(&button_def_name) {
                let msg = format!("Button definition '{}' is referenced but not defined", button_def_name);
                eprintln!("Error: {}", msg);
                result.errors.push(ValidationError {
                    category: "button_definition".to_string(),
                    message: msg,
                });
            }
        } else {
            let msg = format!("Button definition '{}' is referenced but no button definitions exist", button_def_name);
            eprintln!("Error: {}", msg);
            result.errors.push(ValidationError {
                category: "button_definition".to_string(),
                message: msg,
            });
        }
    }
}

/// Validates that all icon files referenced in buttons exist
fn validate_icon_files(conf: &KeyDeckConf, result: &mut ValidationResult, json_output: bool) {
    verbose_log!("Validating icon files...");

    let image_dir = get_icon_dir();

    let mut referenced_icons = HashSet::new();

    // Collect all icon references from button definitions
    if let Some(button_defs) = &conf.buttons {
        for (_name, button) in button_defs {
            if let Some(icon) = &button.icon {
                referenced_icons.insert(icon.clone());
            }
        }
    }

    // Collect all icon references from pages
    for (_group_name, page_group) in &conf.page_groups {
        for (_page_name, page) in &page_group.pages {
            for (_button_key, button_config) in &page.buttons {
                if let ButtonConfig::Detailed(button) = button_config {
                    if let Some(icon) = &button.icon {
                        referenced_icons.insert(icon.clone());
                    }
                }
            }
        }
    }

    // Check if icon files exist
    for icon_file in &referenced_icons {
        let icon_path = PathBuf::from(&image_dir).join(icon_file);
        if !icon_path.exists() {
            let msg = format!("Icon file '{}' not found at path: {}", icon_file, icon_path.display());
            eprintln!("Error: {}", msg);
            result.errors.push(ValidationError {
                category: "icon".to_string(),
                message: msg,
            });
        } else {
            verbose_log!("  ✓ Icon file exists: {}", icon_file);
        }
    }

    // Collect unreferenced icon files for reporting
    let image_dir_path = PathBuf::from(&image_dir);
    if let Ok(entries) = fs::read_dir(&image_dir_path) {
        let mut unreferenced_icons = Vec::new();

        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    if let Some(filename) = entry.file_name().to_str() {
                        // Check if this icon is referenced in the config
                        if !referenced_icons.contains(filename) {
                            unreferenced_icons.push(filename.to_string());
                        }
                    }
                }
            }
        }

        // Sort and report unreferenced icons
        if !unreferenced_icons.is_empty() {
            unreferenced_icons.sort();

            // Always populate the result for JSON output
            result.unreferenced_icons = unreferenced_icons.clone();

            // In verbose mode (non-JSON), also print to console
            if !json_output && crate::DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
                verbose_log!("\n  Unreferenced icon files:");
                for icon in unreferenced_icons {
                    verbose_log!("  {} # icon not referenced in configuration", icon);
                }
            }
        }
    }
}

/// Validates page references (main_page, restore_mode, jump targets)
fn validate_page_references(conf: &KeyDeckConf, result: &mut ValidationResult, json_output: bool) {
    verbose_log!("Validating page references...");

    for (group_name, page_group) in &conf.page_groups {
        // Validate main_page reference
        if let Some(main_page_name) = &page_group.main_page {
            if !page_group.pages.contains_key(main_page_name) {
                let msg = format!(
                    "Page group '{}' has main_page '{}' but this page does not exist. Available pages: {:?}",
                    group_name,
                    main_page_name,
                    page_group.pages.keys().collect::<Vec<_>>()
                );
                if !json_output {
                    eprintln!("Error: {}", msg);
                }
                result.errors.push(ValidationError {
                    category: "page_reference".to_string(),
                    message: msg,
                });
            }
        }

        // Validate jump action targets in each page
        for (page_name, page) in &page_group.pages {
            // Check button actions for jump targets
            for (button_key, button_config) in &page.buttons {
                if let crate::pages::ButtonConfig::Detailed(button) = button_config {
                    if let Some(actions) = &button.actions {
                        validate_actions_page_refs(
                            actions,
                            group_name,
                            page_name,
                            button_key,
                            &page_group.pages,
                            result,
                            json_output
                        );
                    }
                }
            }

            // Check on_tick actions for jump targets
            if let Some(on_tick_actions) = &page.on_tick {
                validate_actions_page_refs(
                    on_tick_actions,
                    group_name,
                    page_name,
                    "on_tick",
                    &page_group.pages,
                    result,
                    json_output
                );
            }
        }
    }

    // Note: Macro validation for page references is complex because:
    // 1. Macros use raw serde_yaml_ng::Value for actions (not parsed Action enum)
    // 2. Macros can be called from any page group context
    // 3. Jump targets in macros are validated at runtime when the macro is executed
    // So we skip detailed macro jump validation here.
}

/// Helper to validate action references to pages
fn validate_actions_page_refs(
    actions: &[crate::pages::Action],
    group_name: &str,
    page_name: &str,
    location: &str,
    available_pages: &indexmap::IndexMap<String, crate::pages::Page>,
    result: &mut ValidationResult,
    json_output: bool
) {
    for action in actions {
        match action {
            crate::pages::Action::Jump { jump: target_page } => {
                if !available_pages.contains_key(target_page) {
                    let msg = format!(
                        "Page group '{}', page '{}', {}: jump action references non-existent page '{}'. Available pages: {:?}",
                        group_name,
                        page_name,
                        location,
                        target_page,
                        available_pages.keys().collect::<Vec<_>>()
                    );
                    if !json_output {
                        eprintln!("Error: {}", msg);
                    }
                    result.errors.push(ValidationError {
                        category: "page_reference".to_string(),
                        message: msg,
                    });
                }
            }
            crate::pages::Action::Try { try_actions, else_actions } => {
                // Recursively validate try and else blocks
                validate_actions_page_refs(try_actions, group_name, page_name, location, available_pages, result, json_output);
                if let Some(else_acts) = else_actions {
                    validate_actions_page_refs(else_acts, group_name, page_name, location, available_pages, result, json_output);
                }
            }
            _ => {} // Other actions don't reference pages
        }
    }
}
