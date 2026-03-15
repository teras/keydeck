use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error in file {file}: {error}")]
    JsonParse {
        file: String,
        error: serde_json::Error,
    },

    #[error("Invalid hardware ID field '{field}' with value '{value}': {error}")]
    InvalidHardwareId {
        field: String,
        value: String,
        error: std::num::ParseIntError,
    },

    #[error("Duplicate device definition for VID 0x{vid:04x} PID 0x{pid:04x}")]
    DuplicateDevice {
        vid: u16,
        pid: u16,
    },

    #[error("No device definitions found")]
    NoDevicesFound,
}
