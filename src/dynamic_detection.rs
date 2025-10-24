use crate::pages::{Action, Button, ButtonConfig, DrawConfig, KeyDeckConf, Macro, TextConfig};
use indexmap::IndexMap;
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// Detects if a string contains dynamic parameter patterns.
/// Dynamic patterns have the format ${provider:argument} (colon required).
/// Macro parameter placeholders ${identifier} (no colon) are NOT dynamic.
///
/// Examples:
///   "${time:%H:%M}"      -> true  (dynamic)
///   "${env:USER}"        -> true  (dynamic)
///   "${service:cpu}"     -> true  (dynamic)
///   "${param}"           -> false (macro parameter)
///   "${service:${var}}"  -> true  (nested, but outer has colon)
pub fn has_dynamic_pattern(text: &str) -> bool {
    if !text.contains("${") {
        return false;
    }

    // Match ${provider:argument} - requires colon to be dynamic
    // Pattern: ${ followed by non-colon/non-brace chars, then :, then non-brace chars, then }
    lazy_static::lazy_static! {
        static ref DYNAMIC_PATTERN: Regex = Regex::new(r"\$\{[^:}]+:[^}]+\}").unwrap();
    }

    DYNAMIC_PATTERN.is_match(text)
}

/// Scans a TextConfig for dynamic patterns
fn has_dynamic_in_text(text_config: &Option<TextConfig>) -> bool {
    match text_config {
        Some(TextConfig::Simple(s)) => has_dynamic_pattern(s),
        Some(TextConfig::Detailed { value, .. }) => has_dynamic_pattern(value),
        None => false,
    }
}

/// Scans a DrawConfig for dynamic patterns
fn has_dynamic_in_draw(draw_config: &Option<DrawConfig>) -> bool {
    match draw_config {
        Some(draw) => has_dynamic_pattern(&draw.value),
        None => false,
    }
}

/// Scans actions for dynamic patterns (recursively)
fn has_dynamic_in_actions(
    actions: &[Action],
    macros: &Option<IndexMap<String, Macro>>,
    visited_macros: &mut HashSet<String>,
) -> bool {
    for action in actions {
        match action {
            Action::Exec { exec, .. } => {
                if has_dynamic_pattern(exec) {
                    return true;
                }
            }
            Action::Text { text } => {
                if has_dynamic_pattern(text) {
                    return true;
                }
            }
            Action::Key { key } => {
                if has_dynamic_pattern(key) {
                    return true;
                }
            }
            Action::Focus { focus } => {
                if has_dynamic_pattern(focus) {
                    return true;
                }
            }
            Action::Macro(macro_call) => {
                // Check call-site parameters for dynamic content
                for (_param_name, param_value) in &macro_call.params {
                    if has_dynamic_pattern(param_value) {
                        return true;
                    }
                }

                // Check if macro defaults (for non-overridden params) are dynamic
                // and if macro body is dynamic
                if is_macro_dynamic(&macro_call.name, &macro_call.params, macros, visited_macros) {
                    return true;
                }
            }
            Action::Try { try_actions, else_actions } => {
                if has_dynamic_in_actions(try_actions, macros, visited_macros) {
                    return true;
                }
                if let Some(else_acts) = else_actions {
                    if has_dynamic_in_actions(else_acts, macros, visited_macros) {
                        return true;
                    }
                }
            }
            Action::And { and_actions } => {
                if has_dynamic_in_actions(and_actions, macros, visited_macros) {
                    return true;
                }
            }
            Action::Or { or_actions } => {
                if has_dynamic_in_actions(or_actions, macros, visited_macros) {
                    return true;
                }
            }
            Action::Not { not_action } => {
                if has_dynamic_in_actions(&[*not_action.clone()], macros, visited_macros) {
                    return true;
                }
            }
            // Other actions (Jump, AutoJump, Wait, WaitFor, Return, Fail, Refresh) don't have dynamic content
            _ => {}
        }
    }
    false
}

/// Checks if a macro is dynamic, considering call-site parameter overrides
fn is_macro_dynamic(
    macro_name: &str,
    call_site_params: &HashMap<String, String>,
    macros: &Option<IndexMap<String, Macro>>,
    visited_macros: &mut HashSet<String>,
) -> bool {
    // Cycle detection
    if visited_macros.contains(macro_name) {
        return false; // Already visiting, avoid infinite loop
    }

    let Some(macros_map) = macros else {
        return false; // No macros defined
    };

    let Some(macro_def) = macros_map.get(macro_name) else {
        return false; // Macro not found
    };

    visited_macros.insert(macro_name.to_string());

    // Check default parameters (only if not overridden at call site)
    if let Some(default_params) = &macro_def.params {
        for (param_name, default_value) in default_params {
            if !call_site_params.contains_key(param_name) {
                // This param uses the default value
                if has_dynamic_pattern(default_value) {
                    visited_macros.remove(macro_name);
                    return true;
                }
            }
        }
    }

    // Check macro body for dynamic patterns
    let result = scan_yaml_value(&macro_def.actions, macros, visited_macros);

    visited_macros.remove(macro_name);
    result
}

/// Recursively scans a YAML value for dynamic patterns
fn scan_yaml_value(
    value: &serde_yaml_ng::Value,
    macros: &Option<IndexMap<String, Macro>>,
    visited_macros: &mut HashSet<String>,
) -> bool {
    match value {
        serde_yaml_ng::Value::String(s) => {
            // Only flag as dynamic if it has ${provider:arg} pattern
            has_dynamic_pattern(s)
        }

        serde_yaml_ng::Value::Mapping(map) => {
            // Check if this is a macro call
            if let Some(serde_yaml_ng::Value::String(macro_name)) =
                map.get(&serde_yaml_ng::Value::String("macro".to_string()))
            {
                // Extract call-site parameters
                let mut call_params = HashMap::new();
                for (key, val) in map {
                    if let serde_yaml_ng::Value::String(key_str) = key {
                        if key_str != "macro" {
                            if let serde_yaml_ng::Value::String(val_str) = val {
                                call_params.insert(key_str.clone(), val_str.clone());
                            }
                        }
                    }
                }

                // Check call-site parameters
                for (_, param_value) in &call_params {
                    if has_dynamic_pattern(param_value) {
                        return true;
                    }
                }

                // Check if macro is dynamic
                return is_macro_dynamic(macro_name, &call_params, macros, visited_macros);
            }

            // Not a macro call, scan all values in the mapping
            for (_, v) in map {
                if scan_yaml_value(v, macros, visited_macros) {
                    return true;
                }
            }
            false
        }

        serde_yaml_ng::Value::Sequence(seq) => {
            // Scan all items in sequence
            for item in seq {
                if scan_yaml_value(item, macros, visited_macros) {
                    return true;
                }
            }
            false
        }

        _ => false, // Numbers, booleans, null - not dynamic
    }
}

/// Main entry point: Determines if a button is dynamic
///
/// Returns true if:
/// 1. Button has explicit `dynamic: true` override
/// 2. Button contains dynamic patterns in text, draw, or actions
///
/// Returns false if:
/// 1. Button has explicit `dynamic: false` override
/// 2. No dynamic patterns detected (or detection failed)
pub fn is_button_dynamic(
    button: &Button,
    macros: &Option<IndexMap<String, Macro>>,
) -> bool {
    // Explicit override takes absolute precedence
    if let Some(explicit) = button.dynamic {
        return explicit;
    }

    // Otherwise, scan for dynamic patterns (default to false on any failure)

    // Check text field
    if has_dynamic_in_text(&button.text) {
        return true;
    }

    // Check draw config
    if has_dynamic_in_draw(&button.draw) {
        return true;
    }

    // Check actions
    if let Some(actions) = &button.actions {
        let mut visited_macros = HashSet::new();
        if has_dynamic_in_actions(actions, macros, &mut visited_macros) {
            return true;
        }
    }

    // No dynamic patterns found
    false
}

/// Computes and sets the is_dynamic_computed field for all buttons in the configuration.
/// Called once during configuration load, after template inheritance is resolved.
pub fn compute_all_dynamic_flags(conf: &mut KeyDeckConf) {
    let macros = conf.macros.clone();

    // First pass: Scan all global button definitions
    if let Some(button_defs) = &mut conf.buttons {
        for (_, button) in button_defs.iter_mut() {
            button.is_dynamic_computed = is_button_dynamic(button, &macros);
        }
    }

    // Second pass: Scan all page buttons (after template resolution)
    for (_, pages) in &mut conf.page_groups {
        for (_, page) in &mut pages.pages {
            for (_, button_config) in &mut page.buttons {
                if let ButtonConfig::Detailed(button) = button_config {
                    button.is_dynamic_computed = is_button_dynamic(button, &macros);
                }
                // ButtonConfig::Template references will use the computed value
                // from the button definition (computed in first pass above)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_dynamic_pattern() {
        // Dynamic patterns (with colon)
        assert!(has_dynamic_pattern("${time:%H:%M}"));
        assert!(has_dynamic_pattern("${env:USER}"));
        assert!(has_dynamic_pattern("${service:cpu}"));
        assert!(has_dynamic_pattern("CPU: ${service:cpu}%"));
        assert!(has_dynamic_pattern("${service:${var}}"));  // Nested

        // Not dynamic (no colon = macro parameter)
        assert!(!has_dynamic_pattern("${param}"));
        assert!(!has_dynamic_pattern("${value}"));
        assert!(!has_dynamic_pattern("Text ${var} here"));

        // No patterns at all
        assert!(!has_dynamic_pattern("static text"));
        assert!(!has_dynamic_pattern(""));
    }

    #[test]
    fn test_has_dynamic_in_text() {
        // Simple variant
        let text_simple = Some(TextConfig::Simple("CPU: ${service:cpu}%".to_string()));
        assert!(has_dynamic_in_text(&text_simple));

        let text_static = Some(TextConfig::Simple("Static".to_string()));
        assert!(!has_dynamic_in_text(&text_static));

        // Detailed variant
        let text_detailed = Some(TextConfig::Detailed {
            value: "${time:%H:%M}".to_string(),
            font_size: Some(20.0),
        });
        assert!(has_dynamic_in_text(&text_detailed));

        // None
        assert!(!has_dynamic_in_text(&None));
    }
}
