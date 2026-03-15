pub mod device;
pub mod error;
pub mod images;
pub mod registry;
pub mod state;
pub mod types;

// Re-export registry types for convenience
pub use registry::{
    DeviceRegistry, DeviceDefinition, BackgroundConfig,
    ButtonImageFormat, ImageMode as RegistryImageMode, Rotation, Mirror,
    MIRAJAZZ_USAGE_PAGE, MIRAJAZZ_USAGE_ID,
};
