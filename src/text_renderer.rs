use cosmic_text::{Align, Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache, Wrap};
use image::{Rgba, RgbaImage};
use std::sync::OnceLock;

/// Padding around text when auto-sizing (percentage of image dimension)
const AUTO_SIZE_PADDING: f32 = 0.1;

/// Line spacing as multiple of font size (baseline-to-baseline)
const LINE_SPACING_FACTOR: f32 = 1.3;

/// Default maximum font size when no user preference is specified
const DEFAULT_FONT_SIZE: f32 = 28.0;

/// Cache for the detected emoji font name
static EMOJI_FONT_NAME: OnceLock<String> = OnceLock::new();

/// Get platform-specific color emoji font names in order of preference
fn get_emoji_font_candidates() -> &'static [&'static str] {
    #[cfg(target_os = "windows")]
    return &["Segoe UI Emoji", "Noto Color Emoji"];

    #[cfg(target_os = "macos")]
    return &["Apple Color Emoji", "Noto Color Emoji"];

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return &[
        "Noto Color Emoji",
        "Twitter Color Emoji",
        "Twemoji",
        "JoyPixels",
        "OpenMoji",
        "Blobmoji",
        "Symbola",
    ];
}

/// Find the first available emoji font from the candidates list
fn find_available_emoji_font(font_system: &FontSystem) -> Option<String> {
    let db = font_system.db();
    let candidates = get_emoji_font_candidates();

    for candidate in candidates {
        for face in db.faces() {
            for family in &face.families {
                if family.0.eq_ignore_ascii_case(candidate) {
                    return Some(candidate.to_string());
                }
            }
        }
    }

    None
}

/// Get the emoji font name (cached after first lookup)
fn get_emoji_font_name(font_system: &FontSystem) -> &'static str {
    EMOJI_FONT_NAME.get_or_init(|| {
        find_available_emoji_font(font_system)
            .unwrap_or_else(|| get_emoji_font_candidates()[0].to_string())
    })
}

/// Check if a character is an emoji
fn is_emoji(c: char) -> bool {
    matches!(c as u32,
        0x1F300..=0x1F9FF | // Misc Symbols and Pictographs, Emoticons, etc.
        0x2600..=0x26FF |   // Misc symbols
        0x2700..=0x27BF |   // Dingbats
        0xFE00..=0xFE0F |   // Variation Selectors
        0x1F1E6..=0x1F1FF   // Regional Indicator Symbols (flags)
    )
}

/// Build rich text spans with proper font family for emoji
fn build_rich_text_spans<'a>(text: &'a str, font_system: &FontSystem) -> Vec<(&'a str, Attrs<'a>)> {
    let mut spans = Vec::new();
    let emoji_font = get_emoji_font_name(font_system);

    let mut current_start = 0;
    let mut in_emoji = false;
    let mut emoji_start = 0;

    for (i, c) in text.char_indices() {
        let is_emoji_char = is_emoji(c);

        if is_emoji_char && !in_emoji {
            if i > current_start {
                spans.push((&text[current_start..i], Attrs::new()));
            }
            in_emoji = true;
            emoji_start = i;
        } else if !is_emoji_char && in_emoji {
            spans.push((&text[emoji_start..i], Attrs::new().family(Family::Name(emoji_font))));
            in_emoji = false;
            current_start = i;
        }
    }

    // Handle final section
    let text_len = text.len();
    if in_emoji {
        spans.push((&text[emoji_start..text_len], Attrs::new().family(Family::Name(emoji_font))));
    } else if current_start < text_len {
        spans.push((&text[current_start..text_len], Attrs::new()));
    }

    if spans.is_empty() {
        spans.push((text, Attrs::new()));
    }

    spans
}

/// Calculate optimal font size using binary search
/// Works for both single-line and multi-line text
fn calculate_optimal_font_size(
    font_system: &mut FontSystem,
    lines: &[&str],
    width: u32,
    height: u32,
    preferred_size: f32,
) -> f32 {
    let target_width = width as f32 * (1.0 - AUTO_SIZE_PADDING);
    let target_height = height as f32 * (1.0 - AUTO_SIZE_PADDING);

    // Find the longest line (measured at an arbitrary test size)
    let test_size = 16.0;
    let test_metrics = Metrics::new(test_size, test_size * LINE_SPACING_FACTOR);

    let mut longest_line = lines[0];
    let mut max_width = 0.0f32;

    for line in lines.iter() {
        let mut buffer = Buffer::new(font_system, test_metrics);
        buffer.set_wrap(font_system, Wrap::None);
        buffer.set_size(font_system, Some(width as f32), None);

        let spans = build_rich_text_spans(line, font_system);
        buffer.set_rich_text(font_system, spans, &Attrs::new(), Shaping::Advanced, Some(Align::Center));
        buffer.shape_until_scroll(font_system, false);

        for run in buffer.layout_runs() {
            if run.line_w > max_width {
                max_width = run.line_w;
                longest_line = line;
            }
        }
    }

    // Binary search for optimal font size using only the longest line
    // Start from user's preferred size (or default if none)
    let mut min_size = 6.0;
    let mut max_size = preferred_size;
    let mut best_size = min_size;

    while max_size - min_size > 0.5 {
        let test_size = (min_size + max_size) / 2.0;
        let line_height = test_size * LINE_SPACING_FACTOR;
        let total_height_needed = lines.len() as f32 * line_height;

        // Check if total height fits
        if total_height_needed > target_height {
            max_size = test_size;
            continue;
        }

        // Check if the longest line fits horizontally
        let metrics = Metrics::new(test_size, line_height);
        let mut buffer = Buffer::new(font_system, metrics);
        buffer.set_wrap(font_system, Wrap::None);
        buffer.set_size(font_system, Some(target_width), None);

        let spans = build_rich_text_spans(longest_line, font_system);
        buffer.set_rich_text(font_system, spans, &Attrs::new(), Shaping::Advanced, Some(Align::Center));
        buffer.shape_until_scroll(font_system, false);

        let mut line_width = 0.0f32;
        for run in buffer.layout_runs() {
            if run.line_w > line_width {
                line_width = run.line_w;
            }
        }

        if line_width <= target_width {
            best_size = test_size;
            min_size = test_size;
        } else {
            max_size = test_size;
        }
    }

    best_size
}

/// Render text directly onto a canvas
pub fn render_text_on_canvas(
    canvas: &mut RgbaImage,
    text: &str,
    font_size: Option<f32>,
    text_color: Option<Rgba<u8>>,
    outline_color: Option<[u8; 3]>,
) {
    let width = canvas.width();
    let height = canvas.height();
    let mut font_system = FontSystem::new();

    // Split text into lines
    let lines: Vec<&str> = text.split('\n').collect();

    // Determine final font size - always run auto-scaling
    // Use user's font size as preferred (or default if not specified)
    let preferred_size = font_size.unwrap_or(DEFAULT_FONT_SIZE);
    let final_font_size = calculate_optimal_font_size(&mut font_system, &lines, width, height, preferred_size);

    // Calculate line height and total block height
    let line_height = final_font_size * LINE_SPACING_FACTOR;
    let total_block_height = lines.len() as f32 * line_height;

    // Center the block vertically
    let mut y_offset = ((height as f32 - total_block_height) / 2.0).max(0.0);

    // Render each line
    for line in lines.iter() {
        let line_canvas_height = (line_height.ceil() as u32).min(height);
        let mut line_canvas = RgbaImage::new(width, line_canvas_height);

        // Render the line onto its own canvas
        render_line_on_canvas(
            &mut line_canvas,
            line,
            final_font_size,
            text_color,
            outline_color,
        );

        // Composite onto main canvas
        let dst_y_start = y_offset as u32;
        for y in 0..line_canvas_height {
            let dst_y = dst_y_start + y;
            if dst_y >= height {
                break;
            }

            for x in 0..width {
                let src = line_canvas.get_pixel(x, y);
                let dst = canvas.get_pixel_mut(x, dst_y);

                // Alpha blend
                let alpha = src[3] as f32 / 255.0;
                let inv_alpha = 1.0 - alpha;
                dst[0] = ((src[0] as f32 * alpha + dst[0] as f32 * inv_alpha) as u8).min(255);
                dst[1] = ((src[1] as f32 * alpha + dst[1] as f32 * inv_alpha) as u8).min(255);
                dst[2] = ((src[2] as f32 * alpha + dst[2] as f32 * inv_alpha) as u8).min(255);
                dst[3] = ((src[3] as f32 + dst[3] as f32 * inv_alpha) as u8).min(255);
            }
        }

        y_offset += line_height;
    }
}

/// Render a single line of text onto a canvas
fn render_line_on_canvas(
    canvas: &mut RgbaImage,
    text: &str,
    font_size: f32,
    text_color: Option<Rgba<u8>>,
    outline_color: Option<[u8; 3]>,
) {
    let width = canvas.width();
    let height = canvas.height();
    let mut font_system = FontSystem::new();

    // Create metrics and buffer
    let line_height = font_size * LINE_SPACING_FACTOR;
    let metrics = Metrics::new(font_size, line_height);
    let mut buffer = Buffer::new(&mut font_system, metrics);

    buffer.set_wrap(&mut font_system, Wrap::None);
    buffer.set_size(&mut font_system, Some(width as f32), Some(height as f32));

    // Set text with emoji-aware rich text and center alignment
    let spans = build_rich_text_spans(text, &font_system);
    buffer.set_rich_text(&mut font_system, spans, &Attrs::new(), Shaping::Advanced, Some(Align::Center));
    buffer.shape_until_scroll(&mut font_system, false);

    // Calculate vertical centering
    let layout_runs: Vec<_> = buffer.layout_runs().collect();
    let total_height = if let Some(last_run) = layout_runs.last() {
        last_run.line_top + last_run.line_height
    } else {
        0.0
    };

    let y_offset = ((height as f32 - total_height) / 2.0).max(0.0);

    // Text color (default to white)
    let text_color = text_color.unwrap_or(Rgba([255u8, 255u8, 255u8, 255u8]));
    let cosmic_color = Color::rgba(text_color[0], text_color[1], text_color[2], text_color[3]);

    let mut swash_cache = SwashCache::new();

    // Draw outline if specified
    if let Some(outline_rgb) = outline_color {
        let outline_color = Color::rgba(outline_rgb[0], outline_rgb[1], outline_rgb[2], 255);
        let offsets = [(0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)];

        for &(dx, dy) in &offsets {
            buffer.draw(&mut font_system, &mut swash_cache, outline_color, |x, y, w, h, color| {
                let final_x = (x as f32 + dx) as i32;
                let final_y = (y as f32 + y_offset + dy) as i32;

                if final_x >= 0 && final_y >= 0 {
                    draw_glyph(canvas, final_x, final_y, w, h, color);
                }
            });
        }
    }

    // Draw main text
    buffer.draw(&mut font_system, &mut swash_cache, cosmic_color, |x, y, w, h, color| {
        let final_x = x as i32;
        let final_y = (y as f32 + y_offset) as i32;

        if final_x >= 0 && final_y >= 0 {
            draw_glyph(canvas, final_x, final_y, w, h, color);
        }
    });
}

/// Helper function to draw a single glyph onto the canvas
fn draw_glyph(canvas: &mut RgbaImage, x: i32, y: i32, w: u32, h: u32, color: Color) {
    let canvas_width = canvas.width() as i32;
    let canvas_height = canvas.height() as i32;

    for row in 0..h {
        for col in 0..w {
            let px = x + col as i32;
            let py = y + row as i32;

            if px < 0 || py < 0 || px >= canvas_width || py >= canvas_height {
                continue;
            }

            let pixel = canvas.get_pixel_mut(px as u32, py as u32);

            // Alpha compositing
            let alpha = color.a() as f32 / 255.0;
            let inv_alpha = 1.0 - alpha;

            pixel[0] = ((color.r() as f32 * alpha + pixel[0] as f32 * inv_alpha) as u8).min(255);
            pixel[1] = ((color.g() as f32 * alpha + pixel[1] as f32 * inv_alpha) as u8).min(255);
            pixel[2] = ((color.b() as f32 * alpha + pixel[2] as f32 * inv_alpha) as u8).min(255);
            pixel[3] = ((color.a() as f32 + pixel[3] as f32 * inv_alpha) as u8).min(255);
        }
    }
}
