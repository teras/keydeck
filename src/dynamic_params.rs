// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use crate::pages::ServiceConfig;
use crate::services::{ensure_service_started, get_service_value, ServicesState};
use chrono::Local;
use indexmap::IndexMap;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Error indicator displayed when dynamic parameter evaluation fails
pub const ERROR_INDICATOR: &str = "⚠";

/// Evaluates all dynamic parameters in a string and returns a map of parameter -> value.
/// Supports three provider types:
/// - ${time:FORMAT} - Current time using strftime format
/// - ${env:VAR} - Environment variable
/// - ${service:NAME} - Cached service value
///
/// On error, returns ERROR_INDICATOR for that parameter.
pub fn evaluate_dynamic_params(
    text: &str,
    services_config: &Option<IndexMap<String, ServiceConfig>>,
    services_state: &ServicesState,
    services_active: &Arc<AtomicBool>,
) -> HashMap<String, String> {
    let mut params = HashMap::new();

    // Regex to match ${...} patterns
    let re = Regex::new(r"\$\{([^}]+)\}").unwrap();

    for cap in re.captures_iter(text) {
        let content = &cap[1];     // e.g., "time:%H:%M"

        // Parse provider type and argument
        let value = if let Some((provider, arg)) = content.split_once(':') {
            match provider {
                "time" => evaluate_time_provider(arg),
                "env" => evaluate_env_provider(arg),
                "service" => evaluate_service_provider(arg, services_config, services_state, services_active),
                _ => {
                    // Unknown provider
                    ERROR_INDICATOR.to_string()
                }
            }
        } else {
            // No colon, invalid format
            ERROR_INDICATOR.to_string()
        };

        // Store mapping from full pattern to value
        // Use content (not full_match) as key for substitution
        params.insert(content.to_string(), value);
    }

    params
}

/// Evaluates ${time:FORMAT} provider using chrono
fn evaluate_time_provider(format: &str) -> String {
    let now = Local::now();
    match now.format(format).to_string().is_empty() {
        true => ERROR_INDICATOR.to_string(),
        false => now.format(format).to_string(),
    }
}

/// Evaluates ${env:VAR} provider
fn evaluate_env_provider(var_name: &str) -> String {
    env::var(var_name).unwrap_or_else(|_| ERROR_INDICATOR.to_string())
}

/// Evaluates ${service:NAME} provider
/// Lazily starts the service if not already running.
fn evaluate_service_provider(
    service_name: &str,
    services_config: &Option<IndexMap<String, ServiceConfig>>,
    services_state: &ServicesState,
    services_active: &Arc<AtomicBool>,
) -> String {
    // Check if services are configured
    let config_map = match services_config {
        Some(map) => map,
        None => return ERROR_INDICATOR.to_string(), // No services configured
    };

    // Ensure service is started (lazy spawn)
    if !ensure_service_started(service_name, config_map, services_state, services_active) {
        // Service not found in configuration
        return ERROR_INDICATOR.to_string();
    }

    // Get current value from cache
    get_service_value(service_name, services_state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::new_services_state;

    #[test]
    fn test_time_provider() {
        let result = evaluate_time_provider("%Y");
        assert!(result.len() == 4); // Year is 4 digits
        assert!(result.parse::<i32>().is_ok());
    }

    #[test]
    fn test_env_provider() {
        env::set_var("TEST_VAR", "test_value");
        let result = evaluate_env_provider("TEST_VAR");
        assert_eq!(result, "test_value");

        let result = evaluate_env_provider("NONEXISTENT_VAR_12345");
        assert_eq!(result, "⚠");
    }

    #[test]
    fn test_evaluate_dynamic_params() {
        env::set_var("USER_TEST", "testuser");

        let text = "Time: ${time:%H:%M} User: ${env:USER_TEST}";
        let services_state = new_services_state();
        let services_active = Arc::new(AtomicBool::new(true));
        let params = evaluate_dynamic_params(text, &None, &services_state, &services_active);

        assert!(params.contains_key("time:%H:%M"));
        assert!(params.contains_key("env:USER_TEST"));
        assert_eq!(params.get("env:USER_TEST").unwrap(), "testuser");
    }
}
