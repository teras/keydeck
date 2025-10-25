use std::sync::mpsc::Sender;
use crate::error_log;

pub fn send(tx: &Sender<DeviceEvent>, event: DeviceEvent) {
    tx.send(event).unwrap_or_else(|e| {
        error_log!("Error while sending event: {}", e)
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Reload configuration
    Reload,

    /// System is going to sleep/will awake
    Sleep { sleep: bool },

    /// Timer completed for a wait action
    TimerComplete { sn: String },

    /// Set brightness on a device
    SetBrightness { sn: String, brightness: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitEventType {
    Focus,
    Page,
    Tick,
    Sleep,
    NewDevice,
    RemovedDevice,
    Timer,
}

impl WaitEventType {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "focus" => Ok(WaitEventType::Focus),
            "page" => Ok(WaitEventType::Page),
            "tick" => Ok(WaitEventType::Tick),
            "sleep" => Ok(WaitEventType::Sleep),
            "newdevice" => Ok(WaitEventType::NewDevice),
            "removeddevice" => Ok(WaitEventType::RemovedDevice),
            "timer" => Ok(WaitEventType::Timer),
            _ => Err(format!("Unsupported waitFor event type: '{}'", s)),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            WaitEventType::Focus => "focus",
            WaitEventType::Page => "page",
            WaitEventType::Tick => "tick",
            WaitEventType::Sleep => "sleep",
            WaitEventType::NewDevice => "newdevice",
            WaitEventType::RemovedDevice => "removeddevice",
            WaitEventType::Timer => "timer",
        }
    }
}

impl DeviceEvent {
    /// Extract the WaitEventType from this event, if it corresponds to a waitable event
    pub fn wait_event_type(&self) -> Option<WaitEventType> {
        match self {
            DeviceEvent::FocusChanges { .. } => Some(WaitEventType::Focus),
            DeviceEvent::Tick => Some(WaitEventType::Tick),
            DeviceEvent::Sleep { .. } => Some(WaitEventType::Sleep),
            DeviceEvent::NewDevice { .. } => Some(WaitEventType::NewDevice),
            DeviceEvent::RemovedDevice { .. } => Some(WaitEventType::RemovedDevice),
            DeviceEvent::TimerComplete { .. } => Some(WaitEventType::Timer),
            _ => None,
        }
    }
}