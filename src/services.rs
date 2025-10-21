use crate::dynamic_params::ERROR_INDICATOR;
use crate::pages::ServiceConfig;
use crate::{error_log, verbose_log};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

/// Shared state for all services. Maps service name to current cached value.
pub type ServicesState = Arc<RwLock<HashMap<String, String>>>;

/// Creates a new empty services state
pub fn new_services_state() -> ServicesState {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Spawns a background thread for a service.
/// The service will execute its command on an interval and update the shared state.
///
/// # Arguments
/// * `name` - Service name (used as key in state HashMap)
/// * `config` - Service configuration (command, interval, timeout)
/// * `state` - Shared state HashMap for storing results
/// * `still_active` - Flag to stop the service thread gracefully
pub fn spawn_service(name: String, config: ServiceConfig, state: ServicesState, still_active: Arc<AtomicBool>) {
    verbose_log!("Spawning service thread for '{}'", name);

    thread::spawn(move || {
        let interval = config.interval();
        let timeout = config.timeout();
        let command = config.exec().to_string();

        while still_active.load(std::sync::atomic::Ordering::Relaxed) {
            // Execute command with timeout
            let result = execute_with_timeout(&command, timeout);

            // Update shared state
            {
                let mut state_lock = state.write().unwrap();
                match result {
                    Ok(output) => {
                        // Trim left and right whitespace, preserve internal spaces
                        let trimmed = output.trim().to_string();
                        verbose_log!("Service '{}' updated: {}", name, trimmed);
                        state_lock.insert(name.clone(), trimmed);
                    }
                    Err(e) => {
                        error_log!("Service '{}' failed: {}", name, e);
                        state_lock.insert(name.clone(), ERROR_INDICATOR.to_string());
                    }
                }
            }

            // Sleep until next interval
            thread::sleep(Duration::from_secs_f64(interval));
        }
        verbose_log!("Service thread '{}' stopping", name);
    });
}

/// Executes a bash command with a timeout.
/// Returns stdout on success, or error message on failure/timeout.
fn execute_with_timeout(command: &str, timeout_secs: f64) -> Result<String, String> {
    // Spawn command via bash
    let mut child = Command::new("bash")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    // Wait for command with timeout
    let start = std::time::Instant::now();
    let timeout_duration = Duration::from_secs_f64(timeout_secs);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process has finished
                if status.success() {
                    // Read stdout
                    let output = child.wait_with_output()
                        .map_err(|e| format!("Failed to read output: {}", e))?;
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    return Ok(stdout);
                } else {
                    return Err(format!("Command failed with exit code: {}", status.code().unwrap_or(-1)));
                }
            }
            Ok(None) => {
                // Process still running, check timeout
                if start.elapsed() > timeout_duration {
                    // Timeout exceeded, kill process
                    let _ = child.kill();
                    let _ = child.wait(); // Clean up zombie
                    return Err(format!("Command timed out after {:.1}s", timeout_secs));
                }

                // Sleep briefly before checking again
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                return Err(format!("Error waiting for process: {}", e));
            }
        }
    }
}

/// Lazily starts a service if it hasn't been started yet.
/// Called when ${service:name} is first encountered.
///
/// # Arguments
/// * `name` - Service name
/// * `services_config` - IndexMap of all service configurations
/// * `state` - Shared services state
/// * `still_active` - Flag to control service thread lifecycle
///
/// # Returns
/// true if service was started (or already running), false if service not found in config
pub fn ensure_service_started(
    name: &str,
    services_config: &IndexMap<String, ServiceConfig>,
    state: &ServicesState,
    still_active: &Arc<AtomicBool>,
) -> bool {
    // Check if service already started (exists in state)
    {
        let state_lock = state.read().unwrap();
        if state_lock.contains_key(name) {
            // Already running
            return true;
        }
    }

    // Not started yet, check if it exists in config
    if let Some(config) = services_config.get(name) {
        // Initialize with loading indicator
        {
            let mut state_lock = state.write().unwrap();
            state_lock.insert(name.to_string(), "...".to_string());
        }

        // Spawn service thread
        spawn_service(name.to_string(), config.clone(), state.clone(), still_active.clone());
        true
    } else {
        // Service not defined in config
        false
    }
}

/// Gets the current value of a service from the shared state.
/// Returns ERROR_INDICATOR if service not found or not started.
pub fn get_service_value(name: &str, state: &ServicesState) -> String {
    let state_lock = state.read().unwrap();
    state_lock.get(name).cloned().unwrap_or_else(|| ERROR_INDICATOR.to_string())
}
