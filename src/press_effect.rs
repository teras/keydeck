// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use image::imageops::{overlay, FilterType};
use image::{Rgba, RgbaImage};
use keydeck_types::PressEffectConfig;

const TRANSPARENT: Rgba<u8> = Rgba([0, 0, 0, 0]);
const DEFAULT_EMBOSS_COLOR: Rgba<u8> = Rgba([128, 128, 128, 255]);

fn brighten(c: u8, factor: f32) -> u8 {
    (c as f32 * factor).min(255.0) as u8
}

fn darken(c: u8, factor: f32) -> u8 {
    (c as f32 * factor) as u8
}

fn highlight(base: Rgba<u8>) -> Rgba<u8> {
    let [r, g, b, a] = base.0;
    Rgba([brighten(r, 1.25), brighten(g, 1.25), brighten(b, 1.25), a])
}

fn shadow(base: Rgba<u8>) -> Rgba<u8> {
    let [r, g, b, a] = base.0;
    Rgba([darken(r, 0.75), darken(g, 0.75), darken(b, 0.75), a])
}

/// Fill a rectangular strip with a solid color
fn fill_rect(img: &mut RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32, color: Rgba<u8>) {
    for y in y0..y1 {
        for x in x0..x1 {
            img.put_pixel(x, y, color);
        }
    }
}

/// Draw emboss bevel on the 4 border strips only.
/// `tl`: thickness of top+left border. `br`: thickness of bottom+right border.
/// `raised`: true = highlight top+left / shadow bottom+right.
fn draw_bevel(img: &mut RgbaImage, tl: u32, br: u32, base: Rgba<u8>, raised: bool) {
    let (w, h) = (img.width(), img.height());
    let (tl_color, br_color) = if raised {
        (highlight(base), shadow(base))
    } else {
        (shadow(base), highlight(base))
    };

    // Top edge (full width, excluding top-right corner)
    fill_rect(img, 0, 0, w - br, tl, tl_color);
    // Left edge (excluding corners)
    fill_rect(img, 0, tl, tl, h - br, tl_color);
    // Bottom edge (full width, excluding bottom-left corner)
    fill_rect(img, tl, h - br, w, h, br_color);
    // Right edge (excluding corners)
    fill_rect(img, w - br, tl, w, h - br, br_color);

    // Bottom-left corner: diagonal split tl_color / br_color
    if tl > 0 && br > 0 {
        for y_local in 0..br {
            let split = (tl * (br - 1 - y_local)) / br;
            for x_local in 0..tl {
                let color = if x_local <= split { tl_color } else { br_color };
                img.put_pixel(x_local, (h - br) + y_local, color);
            }
        }
    }
    // Top-right corner: diagonal split tl_color / br_color
    if br > 0 && tl > 0 {
        for y_local in 0..tl {
            let split = (br * (tl - 1 - y_local)) / tl;
            for x_local in 0..br {
                let color = if x_local <= split { tl_color } else { br_color };
                img.put_pixel((w - br) + x_local, y_local, color);
            }
        }
    }
}

/// Compose a button image with press effect applied.
/// Called for EVERY render (both pressed and unpressed).
/// `canvas` is rendered at the reduced size (after `canvas_reduction()`).
/// Returns full device-size image.
pub fn compose_button(
    canvas: &RgbaImage,
    device_w: u32,
    device_h: u32,
    config: &PressEffectConfig,
    pressed: bool,
    border_rgba: Option<Rgba<u8>>,
) -> RgbaImage {
    match config {
        PressEffectConfig::Shrink { pixels, .. } => {
            if !pressed {
                return canvas.clone();
            }
            let t = *pixels;
            let new_w = device_w.saturating_sub(2 * t);
            let new_h = device_h.saturating_sub(2 * t);
            if new_w == 0 || new_h == 0 {
                return canvas.clone();
            }
            let shrunk =
                image::imageops::resize(canvas, new_w, new_h, FilterType::Lanczos3);
            let mut out = RgbaImage::from_pixel(device_w, device_h, TRANSPARENT);
            overlay(&mut out, &shrunk, t as i64, t as i64);
            if let Some(bg) = border_rgba {
                fill_rect(&mut out, 0, 0, device_w, t, bg);
                fill_rect(&mut out, 0, device_h - t, device_w, device_h, bg);
                fill_rect(&mut out, 0, t, t, device_h - t, bg);
                fill_rect(&mut out, device_w - t, t, device_w, device_h - t, bg);
            }
            out
        }
        PressEffectConfig::Shift { pixels, .. } => {
            let t = *pixels;
            if !pressed && border_rgba.is_none() {
                // No shift, no border — content at (0,0) fills the output as-is
                let mut out = RgbaImage::from_pixel(device_w, device_h, TRANSPARENT);
                overlay(&mut out, canvas, 0, 0);
                return out;
            }
            let mut out = RgbaImage::from_pixel(device_w, device_h, TRANSPARENT);
            if pressed {
                overlay(&mut out, canvas, t as i64, t as i64);
                if let Some(bg) = border_rgba {
                    fill_rect(&mut out, 0, 0, device_w, t, bg);
                    fill_rect(&mut out, 0, t, t, device_h, bg);
                }
            } else {
                overlay(&mut out, canvas, 0, 0);
                let bg = border_rgba.unwrap(); // safe: None returned early above
                fill_rect(&mut out, 0, device_h - t, device_w, device_h, bg);
                fill_rect(&mut out, device_w - t, 0, device_w, device_h - t, bg);
            }
            out
        }
        PressEffectConfig::Emboss { pixels, .. } => {
            let t = *pixels;
            let base = border_rgba.unwrap_or(DEFAULT_EMBOSS_COLOR);
            let mut out = RgbaImage::from_pixel(device_w, device_h, TRANSPARENT);
            if pressed {
                // Content at (2T, 2T), bevel: top+left=2T, bottom+right=T
                overlay(&mut out, canvas, (2 * t) as i64, (2 * t) as i64);
                draw_bevel(&mut out, 2 * t, t, base, false);
            } else {
                // Content at (T, T), bevel: top+left=T, bottom+right=2T
                overlay(&mut out, canvas, t as i64, t as i64);
                draw_bevel(&mut out, t, 2 * t, base, true);
            }
            out
        }
    }
}
