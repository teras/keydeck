use serde::Serialize;

#[derive(Serialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub serial: String,
    pub model: String,
    pub button_layout: ButtonLayout,
    pub button_image: ButtonImage,
    #[serde(skip_serializing_if = "is_zero")]
    pub encoders: u8,
    #[serde(skip_serializing_if = "is_zero")]
    pub touchpoints: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lcd_strip: Option<LcdStrip>,
    pub is_visual: bool,
}

#[derive(Serialize)]
pub struct ButtonLayout {
    pub rows: u8,
    pub columns: u8,
    pub total: u8,
}

#[derive(Serialize)]
pub struct ButtonImage {
    pub width: usize,
    pub height: usize,
    pub format: String,
}

#[derive(Serialize)]
pub struct LcdStrip {
    pub width: usize,
    pub height: usize,
}

fn is_zero(num: &u8) -> bool {
    *num == 0
}
