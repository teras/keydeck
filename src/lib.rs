// Public library interface for keydeck
// This allows other crates (like keydeck-config) to use keydeck's types

pub mod pages;
pub mod device_info;
pub mod dynamic_detection;

// Re-export commonly used types
pub use pages::{
    KeyDeckConf, Pages, Page, Button, ButtonConfig, Action, TextConfig, DrawConfig,
    ServiceConfig, Macro, MacroCall, FocusChangeRestorePolicy, GraphicType, Direction,
    ColorMapEntry, RefreshTarget,
};

pub use device_info::{
    DeviceInfo, ButtonLayout, ButtonImage, LcdStrip,
};
