// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025
//
// Provides cached access to local system metrics (CPU, RAM, temperatures)

use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use sysinfo::{
    Components, CpuRefreshKind, MemoryRefreshKind, RefreshKind, System, MINIMUM_CPU_UPDATE_INTERVAL,
};

/// Cache refresh interval for expensive sysinfo sampling
const CACHE_TTL: Duration = Duration::from_millis(750);

lazy_static! {
    static ref SYSTEM_STATE: Mutex<SystemMetricsState> = Mutex::new(SystemMetricsState::new());
}

/// Public API: returns formatted value for `${system:...}` metric requests.
/// Returns error string if the metric cannot be resolved.
pub fn get_system_value(metric_name: &str) -> Result<String, String> {
    let metric = SystemMetric::parse(metric_name)?;
    let mut guard = SYSTEM_STATE
        .lock()
        .map_err(|_| "Internal system metrics lock poisoned".to_string())?;
    let snapshot = guard.snapshot();

    let value = match metric {
        SystemMetric::CpuMax => snapshot
            .cpu_max
            .map(format_percent)
            .ok_or_else(|| "CPU usage unavailable".to_string())?,
        SystemMetric::CpuAverage => snapshot
            .cpu_avg
            .map(format_percent)
            .ok_or_else(|| "CPU usage unavailable".to_string())?,
        SystemMetric::RamPercent => snapshot
            .ram_percent
            .map(format_percent)
            .ok_or_else(|| "RAM usage unavailable".to_string())?,
        SystemMetric::Temperature(alias) => snapshot
            .temperature_value(alias.as_str())
            .map(format_temperature)
            .ok_or_else(|| format!("No temperature sensor matches '{}'", alias))?,
    };

    Ok(value)
}

/// Formats CPU/RAM percentages without decimals
fn format_percent(value: f32) -> String {
    let rounded = value.round();
    if rounded.is_finite() {
        format!("{:.0}", rounded)
    } else {
        "0".to_string()
    }
}

/// Formats temperatures with one decimal
fn format_temperature(value: f32) -> String {
    if value.is_finite() {
        format!("{:.1}", value)
    } else {
        "0.0".to_string()
    }
}

/// Supported metric identifiers
#[derive(Debug)]
enum SystemMetric {
    CpuMax,
    CpuAverage,
    RamPercent,
    Temperature(String),
}

impl SystemMetric {
    fn parse(raw: &str) -> Result<Self, String> {
        let normalized = raw.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Err("Missing system metric name".to_string());
        }

        match normalized.as_str() {
            "cpumax" => Ok(Self::CpuMax),
            "cpuavg" => Ok(Self::CpuAverage),
            "ram" | "rampercent" | "ramusage" => Ok(Self::RamPercent),
            other if other.starts_with("temp") => {
                let alias = other.trim_start_matches("temp").to_string();
                if alias.is_empty() {
                    Err("Temperature metric requires a suffix (e.g., tempcpu)".to_string())
                } else {
                    Ok(Self::Temperature(alias))
                }
            }
            _ => Err(format!("Unknown system metric '{}'", raw)),
        }
    }
}

/// Cached metrics snapshot
struct MetricsSnapshot {
    timestamp: Instant,
    cpu_max: Option<f32>,
    cpu_avg: Option<f32>,
    ram_percent: Option<f32>,
    temperatures: Vec<TemperatureReading>,
}

impl MetricsSnapshot {
    fn temperature_value(&self, alias_raw: &str) -> Option<f32> {
        let alias = alias_raw
            .trim_matches(|c| c == '_' || c == '-' || c == ' ')
            .to_ascii_lowercase();
        if alias.is_empty() {
            return None;
        }

        if alias == "cpu" {
            if let Some(temp) = find_temperature_by_keywords(&self.temperatures, CPU_TEMP_KEYWORDS)
            {
                return Some(temp);
            }
        } else if alias == "gpu" {
            if let Some(temp) = find_temperature_by_keywords(&self.temperatures, GPU_TEMP_KEYWORDS)
            {
                return Some(temp);
            }
        } else if alias == "nvme" {
            if let Some(temp) = find_temperature_by_keywords(&self.temperatures, NVME_TEMP_KEYWORDS)
            {
                return Some(temp);
            }
        }

        self.temperatures
            .iter()
            .find(|reading| reading.normalized.contains(&alias))
            .map(|reading| reading.value)
    }
}

/// Stores normalized component labels for quick substring matching
struct TemperatureReading {
    normalized: String,
    value: f32,
}

const CPU_TEMP_KEYWORDS: &[&str] = &["cpu", "package id", "tctl", "tdie", "core", "soc"];
const GPU_TEMP_KEYWORDS: &[&str] = &["gpu"];
const NVME_TEMP_KEYWORDS: &[&str] = &["nvme"];

fn find_temperature_by_keywords(readings: &[TemperatureReading], keywords: &[&str]) -> Option<f32> {
    for keyword in keywords {
        let needle = keyword.to_ascii_lowercase();
        if let Some(reading) = readings.iter().find(|r| r.normalized.contains(&needle)) {
            return Some(reading.value);
        }
    }
    None
}

/// Stores sysinfo state and cached snapshot
struct SystemMetricsState {
    system: System,
    components: Components,
    cached: Option<MetricsSnapshot>,
    cpu_initialized: bool,
}

impl SystemMetricsState {
    fn new() -> Self {
        let refresh = RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything());
        let system = System::new_with_specifics(refresh);
        let mut components = Components::new();
        components.refresh_list();
        Self {
            system,
            components,
            cached: None,
            cpu_initialized: false,
        }
    }

    fn snapshot(&mut self) -> &MetricsSnapshot {
        let now = Instant::now();
        let needs_refresh = self
            .cached
            .as_ref()
            .map(|snap| now.duration_since(snap.timestamp) > CACHE_TTL)
            .unwrap_or(true);

        if needs_refresh {
            self.refresh_snapshot(now);
        }

        self.cached.as_ref().expect("metrics snapshot must exist")
    }

    fn refresh_snapshot(&mut self, now: Instant) {
        if !self.cpu_initialized {
            self.system.refresh_cpu();
            std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
            self.cpu_initialized = true;
        }
        self.system.refresh_cpu();
        self.system.refresh_memory();
        if self.components.list().is_empty() {
            self.components.refresh_list();
        } else {
            self.components.refresh();
        }

        let cpus = self.system.cpus();
        let (cpu_max, cpu_avg) = if cpus.is_empty() {
            (None, None)
        } else {
            let mut total = 0.0f32;
            let mut max = 0.0f32;
            for cpu in cpus {
                let usage = cpu.cpu_usage();
                if usage.is_finite() {
                    total += usage;
                    if usage > max {
                        max = usage;
                    }
                }
            }
            let avg = total / cpus.len() as f32;
            (Some(max), Some(avg))
        };

        let total_mem = self.system.total_memory() as f64;
        let used_mem = self.system.used_memory() as f64;
        let ram_percent = if total_mem > 0.0 {
            Some(((used_mem / total_mem) * 100.0) as f32)
        } else {
            None
        };

        let mut temperatures = Vec::new();
        for component in self.components.list() {
            let normalized = component.label().to_ascii_lowercase();
            let value = component.temperature();
            if value.is_finite() {
                temperatures.push(TemperatureReading { normalized, value });
            }
        }

        self.cached = Some(MetricsSnapshot {
            timestamp: now,
            cpu_max,
            cpu_avg,
            ram_percent,
            temperatures,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_invalid_metric() {
        assert!(SystemMetric::parse("").is_err());
        assert!(SystemMetric::parse("unknown").is_err());
    }

    #[test]
    fn parse_temperature_alias() {
        match SystemMetric::parse("tempcpu") {
            Ok(SystemMetric::Temperature(alias)) => assert_eq!(alias, "cpu"),
            other => panic!("unexpected parse result: {:?}", other),
        }
    }
}
