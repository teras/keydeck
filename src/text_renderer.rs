use ab_glyph::{FontArc, PxScale, Font, ScaleFont};
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;

/// Padding around text when auto-sizing (percentage of image dimension)
const AUTO_SIZE_PADDING: f32 = 0.1;

/// Calculate line width for a given text and font
fn calculate_line_width(font: &FontArc, line: &str, scale: PxScale) -> f32 {
    let scaled_font = font.as_scaled(scale);
    let mut width = 0.0;
    for c in line.chars() {
        let glyph_id = font.glyph_id(c);
        width += scaled_font.h_advance(glyph_id);
    }
    width
}

/// Calculate optimal font size to fit multi-line text within given dimensions
/// Uses binary search to find the largest font size that fits
fn calculate_optimal_font_size(
    font: &FontArc,
    text: &str,
    width: u32,
    height: u32,
) -> f32 {
    let target_width = width as f32 * (1.0 - AUTO_SIZE_PADDING);
    let target_height = height as f32 * (1.0 - AUTO_SIZE_PADDING);

    let lines: Vec<&str> = text.split('\n').collect();
    let line_count = lines.len();

    let mut min_size = 6.0;
    let mut max_size = 32.0;
    let mut best_size = min_size;

    // Binary search for optimal font size, starting at 32
    let mut test_size = 32.0;

    // First, check if 12 fits
    let scale = PxScale::from(test_size);
    let scaled_font = font.as_scaled(scale);

    // Find the widest line
    let max_line_width = lines.iter()
        .map(|line| calculate_line_width(font, line, scale))
        .fold(0.0f32, f32::max);

    let ascent = scaled_font.ascent();
    let descent = scaled_font.descent();
    let line_height = ascent - descent;
    let total_text_height = line_height * line_count as f32;

    if max_line_width <= target_width && total_text_height <= target_height {
        // 32 fits, use it
        best_size = test_size;
        min_size = test_size;
    } else {
        // 32 doesn't fit, search downward (6-32)
        max_size = test_size;
    }

    // Continue binary search
    while max_size - min_size > 0.5 {
        test_size = (min_size + max_size) / 2.0;
        let scale = PxScale::from(test_size);
        let scaled_font = font.as_scaled(scale);

        // Find the widest line
        let max_line_width = lines.iter()
            .map(|line| calculate_line_width(font, line, scale))
            .fold(0.0f32, f32::max);

        // Calculate total text height
        let ascent = scaled_font.ascent();
        let descent = scaled_font.descent();
        let line_height = ascent - descent;
        let total_text_height = line_height * line_count as f32;

        // Check if text fits
        if max_line_width <= target_width && total_text_height <= target_height {
            best_size = test_size;
            min_size = test_size;
        } else {
            max_size = test_size;
        }
    }

    best_size
}

/// Find and load a system font using fontconfig
/// Tries to find DejaVu Sans, then Liberation Sans, then falls back to SansSerif
fn load_system_font() -> Result<FontArc, String> {
    let source = SystemSource::new();

    // Try font families in order of preference
    let font_families = vec![
        FamilyName::Title("DejaVu Sans".to_string()),
        FamilyName::Title("Liberation Sans".to_string()),
        FamilyName::SansSerif,
    ];

    for family in font_families {
        if let Ok(handle) = source.select_best_match(&[family], &Properties::new()) {
            if let Ok(font_data) = handle.load() {
                if let Some(font_vec) = font_data.copy_font_data() {
                    if let Ok(font) = FontArc::try_from_vec(font_vec.to_vec()) {
                        return Ok(font);
                    }
                }
            }
        }
    }

    Err("No suitable system font found. Please install DejaVu Sans or Liberation Sans fonts.".to_string())
}

/// Render text directly onto a canvas (in-place modification)
///
/// # Arguments
/// * `canvas` - Mutable reference to the canvas to draw on
/// * `text` - The text to render
/// * `font_size` - Optional font size (defaults to auto-sizing based on canvas dimensions)
/// * `text_color` - Text color as Rgba (defaults to white with full opacity)
/// * `outline_color` - Optional outline color as RGB array (draws 1px outline around text)
pub fn render_text_on_canvas(
    canvas: &mut RgbaImage,
    text: &str,
    font_size: Option<f32>,
    text_color: Option<Rgba<u8>>,
    outline_color: Option<[u8; 3]>,
) {
    // Load system font using fontconfig
    let font = match load_system_font() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to load system font: {}", e);
            return;
        }
    };

    let width = canvas.width();
    let height = canvas.height();

    // Determine font size: use provided value, or auto-calculate
    let font_size = match font_size {
        Some(size) => size,
        None => calculate_optimal_font_size(&font, text, width, height),
    };

    // Calculate scale for the font
    let scale = PxScale::from(font_size);

    // Text color (default to white with full opacity)
    let text_color = text_color.unwrap_or(Rgba([255u8, 255u8, 255u8, 255u8]));

    let scaled_font = font.as_scaled(scale);

    // Split text into lines
    let lines: Vec<&str> = text.split('\n').collect();

    // Calculate line height and total text block height
    let ascent = scaled_font.ascent();
    let descent = scaled_font.descent();
    let line_height = ascent - descent;
    let total_text_height = line_height * lines.len() as f32;

    // Start y position (vertically center the entire text block)
    let base_y = ((height as f32 - total_text_height) / 2.0) as i32;

    // Draw outline if specified (draw text in 4 directions: up, down, left, right)
    if let Some(outline_rgb) = outline_color {
        let outline_rgba = Rgba([outline_rgb[0], outline_rgb[1], outline_rgb[2], 255]);

        // Outline offsets: up, down, left, right
        let offsets = [(0, -1), (0, 1), (-1, 0), (1, 0)];

        for &(dx, dy) in &offsets {
            let mut y = base_y + dy;

            for line in &lines {
                let line_width = calculate_line_width(&font, line, scale);
                let x = ((width as f32 - line_width) / 2.0).max(0.0) as i32 + dx;

                draw_text_mut(canvas, outline_rgba, x, y, scale, &font, line);
                y += line_height as i32;
            }
        }
    }

    // Draw main text on top
    let mut y = base_y;
    for line in lines {
        // Calculate line width
        let line_width = calculate_line_width(&font, line, scale);

        // Center horizontally
        let x = ((width as f32 - line_width) / 2.0).max(0.0) as i32;

        // Draw the line
        draw_text_mut(canvas, text_color, x, y, scale, &font, line);

        // Move to next line
        y += line_height as i32;
    }

    // Fix alpha channel: imageproc's draw_text_mut blends with background,
    // so on transparent backgrounds it produces low-alpha pixels.
    // We need to boost the alpha channel to make text visible when overlaid.
    for pixel in canvas.pixels_mut() {
        if pixel[3] > 0 {
            // If there's any alpha, set it to full opacity
            // The RGB values are already correctly anti-aliased
            pixel[3] = 255;
        }
    }
}
