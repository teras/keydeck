// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_filled_circle_mut};
use imageproc::rect::Rect;
use std::f32::consts::PI;

/// Parse a hex color string (format: "#RRGGBB" or "0xRRGGBB") into RGB components
pub fn parse_hex_color(hex: &str) -> Result<(u8, u8, u8), String> {
    let hex = hex.trim_start_matches('#').trim_start_matches("0x");

    if hex.len() != 6 {
        return Err(format!("Invalid hex color format: {}", hex));
    }

    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| format!("Invalid red component: {}", hex))?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| format!("Invalid green component: {}", hex))?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| format!("Invalid blue component: {}", hex))?;

    Ok((r, g, b))
}

/// Interpolate between two colors based on a factor (0.0 = color1, 1.0 = color2)
pub fn interpolate_color(color1: (u8, u8, u8), color2: (u8, u8, u8), factor: f32) -> (u8, u8, u8) {
    let factor = factor.clamp(0.0, 1.0);
    let r = (color1.0 as f32 + (color2.0 as f32 - color1.0 as f32) * factor) as u8;
    let g = (color1.1 as f32 + (color2.1 as f32 - color1.1 as f32) * factor) as u8;
    let b = (color1.2 as f32 + (color2.2 as f32 - color1.2 as f32) * factor) as u8;
    (r, g, b)
}

/// Calculate color from a color map based on value percentage
/// Color map format: [(threshold, color), ...] where threshold is in range [0, 100]
/// Returns interpolated color with smooth transitions
pub fn calculate_color_from_map(
    value_percent: f32,
    color_map: &[(f32, (u8, u8, u8))],
) -> (u8, u8, u8) {
    if color_map.is_empty() {
        return (255, 255, 255); // Default to white
    }

    if color_map.len() == 1 {
        return color_map[0].1;
    }

    // Find the two color stops to interpolate between
    for i in 0..color_map.len() - 1 {
        let (threshold1, color1) = color_map[i];
        let (threshold2, color2) = color_map[i + 1];

        if value_percent >= threshold1 && value_percent <= threshold2 {
            // Interpolate between these two colors
            let range = threshold2 - threshold1;
            if range <= 0.0 {
                return color1;
            }
            let factor = (value_percent - threshold1) / range;
            return interpolate_color(color1, color2, factor);
        }
    }

    // If value is below first threshold, use first color
    if value_percent < color_map[0].0 {
        return color_map[0].1;
    }

    // If value is above last threshold, use last color
    color_map[color_map.len() - 1].1
}

/// Direction for bar rendering
#[derive(Debug, Clone, Copy)]
pub enum BarDirection {
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

/// Render a progress bar directly onto canvas with support for all four directions
pub fn render_bar(
    canvas: &mut RgbaImage,
    x: i64,
    y: i64,
    value: f32,
    range: (f32, f32),
    width: u32,
    height: u32,
    color: (u8, u8, u8),
    segments: Option<u32>,
    direction: BarDirection,
) {
    // Calculate percentage
    let (min, max) = range;
    let value = value.clamp(min, max);
    let percent = if max > min { (value - min) / (max - min) } else { 0.0 };

    let color_rgba = Rgba([color.0, color.1, color.2, 255]);

    match direction {
        BarDirection::LeftToRight | BarDirection::RightToLeft => {
            // Horizontal bar
            if let Some(seg_count) = segments {
                // Segmented bar
                if seg_count > 0 {
                    let segment_spacing = 2;
                    let total_spacing = (seg_count - 1) * segment_spacing;
                    let segment_width = (width - total_spacing) / seg_count;

                    // Calculate remaining space and distribute as padding
                    let used_width = seg_count * segment_width + total_spacing;
                    let remaining = width - used_width;
                    let offset_x = remaining / 2;

                    let filled_segments = (percent * seg_count as f32).floor() as u32;

                    for i in 0..filled_segments {
                        let seg_x = if matches!(direction, BarDirection::LeftToRight) {
                            // Fill from left
                            x + offset_x as i64 + (i * (segment_width + segment_spacing)) as i64
                        } else {
                            // Fill from right
                            x + (width - offset_x - ((i + 1) * (segment_width + segment_spacing))) as i64
                        };

                        draw_filled_rect_mut(
                            canvas,
                            Rect::at(seg_x as i32, y as i32).of_size(segment_width, height),
                            color_rgba,
                        );
                    }
                }
            } else {
                // Continuous bar
                let filled_width = (width as f32 * percent) as u32;
                if filled_width > 0 {
                    let bar_x = if matches!(direction, BarDirection::LeftToRight) {
                        x
                    } else {
                        x + (width - filled_width) as i64
                    };

                    draw_filled_rect_mut(
                        canvas,
                        Rect::at(bar_x as i32, y as i32).of_size(filled_width, height),
                        color_rgba,
                    );
                }
            }
        }
        BarDirection::TopToBottom | BarDirection::BottomToTop => {
            // Vertical bar
            if let Some(seg_count) = segments {
                // Segmented bar
                if seg_count > 0 {
                    let segment_spacing = 2;
                    let total_spacing = (seg_count - 1) * segment_spacing;
                    let segment_height = (height - total_spacing) / seg_count;

                    // Calculate remaining space and distribute as padding
                    let used_height = seg_count * segment_height + total_spacing;
                    let remaining = height - used_height;
                    let offset_y = remaining / 2;

                    let filled_segments = (percent * seg_count as f32).floor() as u32;

                    for i in 0..filled_segments {
                        let seg_y = if matches!(direction, BarDirection::BottomToTop) {
                            // Fill from bottom
                            y + (height - offset_y - ((i + 1) * (segment_height + segment_spacing))) as i64
                        } else {
                            // Fill from top
                            y + offset_y as i64 + (i * (segment_height + segment_spacing)) as i64
                        };

                        draw_filled_rect_mut(
                            canvas,
                            Rect::at(x as i32, seg_y as i32).of_size(width, segment_height),
                            color_rgba,
                        );
                    }
                }
            } else {
                // Continuous bar
                let filled_height = (height as f32 * percent) as u32;
                if filled_height > 0 {
                    let bar_y = if matches!(direction, BarDirection::BottomToTop) {
                        y + (height - filled_height) as i64
                    } else {
                        y
                    };

                    draw_filled_rect_mut(
                        canvas,
                        Rect::at(x as i32, bar_y as i32).of_size(width, filled_height),
                        color_rgba,
                    );
                }
            }
        }
    }
}

/// Render a circular gauge (arc from bottom going clockwise) directly onto canvas
pub fn render_gauge(
    canvas: &mut RgbaImage,
    x: i64,
    y: i64,
    value: f32,
    range: (f32, f32),
    width: u32,
    height: u32,
    color: (u8, u8, u8),
) {
    // Calculate percentage
    let (min, max) = range;
    let value = value.clamp(min, max);
    let percent = if max > min { (value - min) / (max - min) } else { 0.0 };

    let color_rgba = Rgba([color.0, color.1, color.2, 255]);

    // Calculate center and radius
    let center_x = x + (width / 2) as i64;
    let center_y = y + (height / 2) as i64;
    let radius = (width.min(height) / 2).saturating_sub(5);

    // Draw arc from 7:30 position (bottom-left) to 4:30 position (bottom-right) at 100%
    let start_angle = 135.0 * PI / 180.0;
    let arc_range = 270.0 * PI / 180.0;
    let end_angle = start_angle + (arc_range * percent);

    // Draw the arc with thick line (using multiple circles)
    let thickness = (radius / 4).max(3);
    let steps = (radius * 2) as i32;

    for step in 0..steps {
        let angle = start_angle + (end_angle - start_angle) * (step as f32 / steps as f32);
        let px = center_x + (radius as f32 * angle.cos()) as i64;
        let py = center_y + (radius as f32 * angle.sin()) as i64;

        draw_filled_circle_mut(canvas, (px as i32, py as i32), thickness as i32, color_rgba);
    }
}

/// Render multiple bars with individual colors directly onto canvas
/// Supports all 4 directions: bars can be arranged horizontally or vertically,
/// and each bar can fill in any of the 4 directions
pub fn render_multi_bar(
    canvas: &mut RgbaImage,
    x: i64,
    y: i64,
    values: &[f32],
    range: (f32, f32),
    width: u32,
    height: u32,
    colors: &[(u8, u8, u8)],
    bar_spacing: u32,
    segments: Option<u32>,
    direction: BarDirection,
) {
    if values.is_empty() {
        return;
    }

    let bar_count = values.len() as u32;

    match direction {
        BarDirection::LeftToRight | BarDirection::RightToLeft => {
            // Horizontal bars stacked vertically
            let total_spacing = (bar_count - 1) * bar_spacing;
            let bar_height = (height - total_spacing) / bar_count;

            for (i, &value) in values.iter().enumerate() {
                let bar_y = y + (i as u32 * (bar_height + bar_spacing)) as i64;
                let color = colors.get(i).copied().unwrap_or((255, 255, 255));
                render_bar(
                    canvas,
                    x,
                    bar_y,
                    value,
                    range,
                    width,
                    bar_height,
                    color,
                    segments,
                    direction,
                );
            }
        }
        BarDirection::TopToBottom | BarDirection::BottomToTop => {
            // Vertical bars side-by-side
            let total_spacing = (bar_count - 1) * bar_spacing;
            let bar_width = (width - total_spacing) / bar_count;

            for (i, &value) in values.iter().enumerate() {
                let bar_x = x + (i as u32 * (bar_width + bar_spacing)) as i64;
                let color = colors.get(i).copied().unwrap_or((255, 255, 255));
                render_bar(
                    canvas,
                    bar_x,
                    y,
                    value,
                    range,
                    bar_width,
                    height,
                    color,
                    segments,
                    direction,
                );
            }
        }
    }
}
