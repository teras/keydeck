use mirajazz_json::DeviceRegistry;
use std::path::PathBuf;

/// Example demonstrating path priority and override behavior.
/// With the parametric API, the application controls path priority.
fn main() {
    println!("=== Testing Path Priority (Parametric API) ===");
    println!();

    // Test 1: Single path
    println!("Test 1 - Loading from single path:");
    let dev_path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/devices"));
    println!("  Path: {}", dev_path.display());

    match DeviceRegistry::load_from_paths(&[&dev_path]) {
        Ok(registry) => {
            println!("  ✓ Loaded {} devices", registry.device_count());
        }
        Err(e) => {
            println!("  ✗ Error: {}", e);
        }
    }

    println!();

    // Test 2: Multiple paths (first wins if no duplicates, last wins on conflicts)
    println!("Test 2 - Loading from multiple paths:");
    let paths = vec![
        PathBuf::from("/tmp/nonexistent"),  // Doesn't exist, will be skipped
        dev_path.clone(),                    // This will load
    ];

    for (i, path) in paths.iter().enumerate() {
        println!("  {}. {} ({})",
            i + 1,
            path.display(),
            if path.exists() { "exists" } else { "skipped" }
        );
    }

    match DeviceRegistry::load_from_paths(&paths) {
        Ok(registry) => {
            println!("  ✓ Loaded {} devices", registry.device_count());
        }
        Err(e) => {
            println!("  ✗ Error: {}", e);
        }
    }

    println!();
    println!("Note: The library is now fully parametric.");
    println!("Applications provide their own path lists with desired priority.");
}
