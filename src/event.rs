use std::sync::mpsc::Sender;

pub fn send(tx: &Sender<DeviceEvent>, event: DeviceEvent) {
    tx.send(event).unwrap_or_else(|e| {
        eprintln!("Error while sending event: {}", e)
    })
}

#[derive(Debug)]
pub enum DeviceEvent {
    /// Button got pressed down
    ButtonDown { sn: String, button_id: u8 },

    /// Button got released
    ButtonUp { sn: String, button_id: u8 },

    /// Encoder got pressed down
    EncoderDown { sn: String, encoder_id: u8 },

    /// Encoder was released from being pressed down
    EncoderUp { sn: String, encoder_id: u8 },

    /// Encoder was twisted
    EncoderTwist { sn: String, encoder_id: u8, value: i8 },

    /// Touch Point got pressed down
    TouchPointDown { sn: String, point_id: u8 },

    /// Touch Point got released
    TouchPointUp { sn: String, point_id: u8 },

    /// Touch screen received short press
    TouchScreenPress { sn: String, x: u16, y: u16 },

    /// Touch screen received long press
    TouchScreenLongPress { sn: String, x: u16, y: u16 },

    /// Touch screen received a swipe
    TouchScreenSwipe { sn: String, start: (u16, u16), end: (u16, u16) },

    /// Window focus changed
    FocusChanges { class: String, title: String },

    /// A tick of the timer clock
    Tick,

    /// New device connected
    NewDevice { sn: String },

    /// Device removed
    RemovedDevice { sn: String },

    /// Exit application
    Exit,

    /// System is going to sleep/will awake
    Sleep { sleep: bool },
}