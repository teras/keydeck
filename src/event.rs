use std::sync::mpsc::Sender;

pub fn send(tx: &Sender<DeviceEvent>, event: DeviceEvent) {
    tx.send(event).unwrap_or_else(|e| {
        eprintln!("Error while sending event: {}", e)
    })
}

#[derive(Debug)]
pub enum DeviceEvent {
    /// Button got pressed down
    ButtonDown(String, u8),

    /// Button got released
    ButtonUp(String, u8),

    /// Encoder got pressed down
    EncoderDown(String, u8),

    /// Encoder was released from being pressed down
    EncoderUp(String, u8),

    /// Encoder was twisted
    EncoderTwist(String, u8, i8),

    /// Touch Point got pressed down
    TouchPointDown(String, u8),

    /// Touch Point got released
    TouchPointUp(String, u8),

    /// Touch screen received short press
    TouchScreenPress(String, u16, u16),

    /// Touch screen received long press
    TouchScreenLongPress(String, u16, u16),

    /// Touch screen received a swipe
    TouchScreenSwipe(String, (u16, u16), (u16, u16)),

    /// Window focus changed
    FocusChanges(String, String),

    /// A tick of the timer clock
    Tick,

    /// New device connected
    NewDevice(String),

    /// Device removed
    RemovedDevice(String),
}