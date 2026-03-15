use super::definition::DeviceDefinition;
use super::error::RegistryError;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Device registry loaded from JSON files
pub struct DeviceRegistry {
    /// Map of (vendor_id, product_id) → DeviceDefinition
    devices: HashMap<(u16, u16), DeviceDefinition>,
}

impl DeviceRegistry {
    /// Load device definitions from multiple directories in priority order.
    /// Later paths take precedence over earlier ones if duplicate VID/PID are found.
    /// Skips directories that don't exist.
    pub fn load_from_paths<P: AsRef<Path>>(paths: &[P]) -> Result<Self, RegistryError> {
        let mut devices = HashMap::new();

        for path in paths {
            let path = path.as_ref();

            // Skip non-existent directories
            if !path.exists() {
                continue;
            }

            // Load all JSON files from this directory
            let dir = match fs::read_dir(path) {
                Ok(dir) => dir,
                Err(_) => continue, // Skip directories we can't read
            };

            for entry in dir {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue, // Skip unreadable entries
                };

                let file_path = entry.path();

                // Only process .json files
                if file_path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }

                // Read and parse JSON file (each file contains a single device object)
                let content = match fs::read_to_string(&file_path) {
                    Ok(c) => c,
                    Err(_) => continue, // Skip unreadable files
                };

                let def: DeviceDefinition = match serde_json::from_str(&content) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {}", file_path.display(), e);
                        continue;
                    }
                };

                // Parse VID/PID
                let vid = match def.hardware.vendor_id_u16() {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Warning: Invalid vendor_id in {}: {}", file_path.display(), e);
                        continue;
                    }
                };

                let pid = match def.hardware.product_id_u16() {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Warning: Invalid product_id in {}: {}", file_path.display(), e);
                        continue;
                    }
                };

                // Later paths override earlier ones (insert replaces existing)
                devices.insert((vid, pid), def);
            }
        }

        if devices.is_empty() {
            return Err(RegistryError::NoDevicesFound);
        }

        Ok(Self { devices })
    }

    /// Load all device definitions from a directory containing JSON files
    pub fn load_from_directory<P: AsRef<Path>>(path: P) -> Result<Self, RegistryError> {
        Self::load_from_paths(&[path])
    }

    /// Load a single device definition from a JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, RegistryError> {
        let content = fs::read_to_string(&path)?;
        let def: DeviceDefinition = serde_json::from_str(&content)
            .map_err(|e| RegistryError::JsonParse {
                file: path.as_ref().display().to_string(),
                error: e,
            })?;

        let vid = def.hardware.vendor_id_u16()
            .map_err(|e| RegistryError::InvalidHardwareId {
                field: "vendor_id".to_string(),
                value: def.hardware.vendor_id.clone(),
                error: e,
            })?;

        let pid = def.hardware.product_id_u16()
            .map_err(|e| RegistryError::InvalidHardwareId {
                field: "product_id".to_string(),
                value: def.hardware.product_id.clone(),
                error: e,
            })?;

        let mut devices = HashMap::new();
        devices.insert((vid, pid), def);

        Ok(Self { devices })
    }

    /// Find a device definition by VID/PID
    pub fn find_by_vid_pid(&self, vendor_id: u16, product_id: u16) -> Option<&DeviceDefinition> {
        self.devices.get(&(vendor_id, product_id))
    }

    /// Check if a VID/PID is supported
    pub fn is_supported(&self, vendor_id: u16, product_id: u16) -> bool {
        self.devices.contains_key(&(vendor_id, product_id))
    }

    /// Get all device definitions
    pub fn all_devices(&self) -> impl Iterator<Item = &DeviceDefinition> {
        self.devices.values()
    }

    /// Get count of registered devices
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Generate HID device queries for all registered devices (for mirajazz)
    pub fn generate_device_queries(&self) -> Vec<crate::device::DeviceQuery> {
        use super::definition::{MIRAJAZZ_USAGE_PAGE, MIRAJAZZ_USAGE_ID};

        self.devices
            .values()
            .map(|def| {
                crate::device::DeviceQuery::new(
                    MIRAJAZZ_USAGE_PAGE,
                    MIRAJAZZ_USAGE_ID,
                    def.hardware.vendor_id_u16().unwrap(),
                    def.hardware.product_id_u16().unwrap(),
                )
            })
            .collect()
    }

    /// Get all vendor IDs (for legacy list_devices)
    pub fn vendor_ids(&self) -> Vec<u16> {
        self.devices
            .keys()
            .map(|(vid, _)| *vid)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::definition::*;

    #[test]
    fn test_empty_registry() {
        let registry = DeviceRegistry { devices: HashMap::new() };
        assert_eq!(registry.device_count(), 0);
        assert!(!registry.is_supported(0x0300, 0x1020));
    }

    #[test]
    fn test_find_device() {
        let mut devices = HashMap::new();
        let def = create_test_device(0x0300, 0x1020);
        devices.insert((0x0300, 0x1020), def);

        let registry = DeviceRegistry { devices };

        assert!(registry.is_supported(0x0300, 0x1020));
        assert!(!registry.is_supported(0x0300, 0x9999));
        assert_eq!(registry.device_count(), 1);

        let found = registry.find_by_vid_pid(0x0300, 0x1020);
        assert!(found.is_some());
        assert_eq!(found.unwrap().info.human_name, "Test Device");
    }

    #[cfg(test)]
    fn create_test_device(vid: u16, pid: u16) -> DeviceDefinition {
        DeviceDefinition {
            hardware: HardwareId {
                vendor_id: format!("0x{:04x}", vid),
                product_id: format!("0x{:04x}", pid),
            },
            info: DeviceInfo {
                human_name: "Test Device".to_string(),
                device_namespace: "test".to_string(),
                manufacturer: None,
                model: None,
            },
            protocol: ProtocolConfig {
                protocol_version: 1,
            },
            layout: Layout {
                rows: 3,
                cols: 6,
                encoder_count: 0,
            },
            image_format: ImageFormatConfig {
                mode: ImageMode::JPEG,
                default_size: [85, 85],
                rotation: Rotation::Rot90,
                mirror: Mirror::Both,
                per_button_overrides: HashMap::new(),
            },
            input_mapping: InputMapping::default(),
            quirks: Quirks::default(),
        }
    }
}
