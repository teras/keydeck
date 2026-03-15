use mirajazz_json::DeviceRegistry;
use std::path::PathBuf;

/// Example showing how to load device definitions from multiple paths.
/// Applications should define their own search paths based on their needs.
fn main() {
    println!("=== Device Registry - Parametric Path Loading ===");
    println!();

    // Example: Define search paths for your application
    // The library no longer has hardcoded paths - you provide them!
    let search_paths = vec![
        // Development path (source tree)
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/devices")),

        // Example: If you want user config directory
        // dirs::home_dir().map(|h| h.join(".config/myapp/devices")),

        // Example: If you want system-wide installation
        // PathBuf::from("/usr/share/myapp/devices"),
    ];

    println!("Searching for device definitions in:");
    for (i, path) in search_paths.iter().enumerate() {
        let exists = path.exists();
        println!("  {}. {} ({})",
            i + 1,
            path.display(),
            if exists { "exists" } else { "not found" }
        );
    }
    println!();

    // Load from multiple paths - later paths override earlier ones
    match DeviceRegistry::load_from_paths(&search_paths) {
        Ok(registry) => {
            println!("✓ Loaded {} device definitions", registry.device_count());
            println!();
            println!("Supported devices:");
            for device in registry.all_devices() {
                println!("  - {} (VID: {}, PID: {})",
                    device.info.human_name,
                    device.hardware.vendor_id,
                    device.hardware.product_id
                );
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to load device registry: {}", e);
            std::process::exit(1);
        }
    }
}
