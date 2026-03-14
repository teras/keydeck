// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use image::imageops::overlay;
use image::{Rgba, RgbaImage};
use keydeck_types::PressEffectConfig;

/// Resolved press effect with concrete parameters
enum PressEffect {
    Shrink(u32),
    Translate(u32),
}

impl From<&PressEffectConfig> for PressEffect {
    fn from(config: &PressEffectConfig) -> Self {
        match config {
            PressEffectConfig::Shrink { percent } => PressEffect::Shrink(*percent),
            PressEffectConfig::Translate { pixels } => PressEffect::Translate(*pixels),
        }
    }
}

/// Apply a press visual effect to a button canvas, returning a modified copy
pub fn apply_press_effect(canvas: &RgbaImage, config: &PressEffectConfig) -> RgbaImage {
    let effect = PressEffect::from(config);
    let (w, h) = (canvas.width(), canvas.height());
    match effect {
        PressEffect::Shrink(margin_pct) => {
            let scale = 1.0 - (margin_pct as f32 / 100.0);
            let new_w = (w as f32 * scale) as u32;
            let new_h = (h as f32 * scale) as u32;
            let shrunk = image::imageops::resize(
                canvas,
                new_w,
                new_h,
                image::imageops::FilterType::Nearest,
            );
            let mut pressed = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
            let x_off = (w - new_w) / 2;
            let y_off = (h - new_h) / 2;
            overlay(&mut pressed, &shrunk, x_off as i64, y_off as i64);
            pressed
        }
        PressEffect::Translate(pixels) => {
            let mut pressed = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
            overlay(&mut pressed, canvas, pixels as i64, pixels as i64);
            pressed
        }
    }
}
