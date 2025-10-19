use ab_glyph::{FontArc, PxScale, Font, ScaleFont};
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use image::{DynamicImage, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    /// Cache for rendered text images to avoid re-rendering the same text
    static ref TEXT_CACHE: Mutex<HashMap<String, DynamicImage>> = Mutex::new(HashMap::new());
}

/// Padding around text when auto-sizing (percentage of image dimension)
const AUTO_SIZE_PADDING: f32 = 0.1;

/// Generate a cache key from text, font size, dimensions, background color, and image presence
fn generate_cache_key(text: &str, font_size: f32, width: u32, height: u32, bg_color: &str, has_image: bool) -> String {
    format!("{}:{}:{}x{}:{}:{}", text, font_size, width, height, bg_color, has_image)
}

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
    let mut max_size = 54.0;
    let mut best_size = min_size;

    // Binary search for optimal font size, starting at 12
    let mut test_size = 12.0;

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
        // 12 fits, search upward (12-200)
        best_size = test_size;
        min_size = test_size;
    } else {
        // 12 doesn't fit, search downward (6-12)
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

/// Render text on an image with the specified parameters
///
/// # Arguments
/// * `text` - The text to render
/// * `font_size` - Optional font size (defaults to auto-sizing)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `bg_color` - Background color as RGB array (defaults to transparent if None)
/// * `text_color` - Text color as Rgba (defaults to white with full opacity)
/// * `outline_color` - Optional outline color as RGB array (draws 1px outline around text)
/// * `background_image` - Optional background image to draw first (scaled to fit while maintaining aspect ratio)
///
/// # Returns
/// A DynamicImage with the rendered text
pub fn render_text(
    text: &str,
    font_size: Option<f32>,
    width: u32,
    height: u32,
    bg_color: Option<[u8; 3]>,
    text_color: Option<Rgba<u8>>,
    outline_color: Option<[u8; 3]>,
    background_image: Option<&DynamicImage>,
) -> Result<DynamicImage, String> {
    // Load system font using fontconfig
    let font = load_system_font()?;

    // Determine font size: use provided value, or auto-calculate, or use default
    let font_size = match font_size {
        Some(size) => size,
        None => calculate_optimal_font_size(&font, text, width, height),
    };

    // Create background RGBA (transparent by default, or use RGB if provided)
    let bg_rgba = bg_color.map_or(
        Rgba([0, 0, 0, 0]), // Transparent background
        |rgb| Rgba([rgb[0], rgb[1], rgb[2], 255]) // Opaque background with provided RGB
    );

    let bg_color_str = format!("{:?}", bg_rgba);
    let cache_key = generate_cache_key(text, font_size, width, height, &bg_color_str, background_image.is_some());

    // Check cache first
    {
        let cache = TEXT_CACHE.lock().unwrap();
        if let Some(cached_image) = cache.get(&cache_key) {
            return Ok(cached_image.clone());
        }
    }

    // Create background image
    let mut image = RgbaImage::from_pixel(width, height, bg_rgba);

    // If a background image is provided, draw it first (scaled to fit while maintaining aspect ratio)
    if let Some(bg_img) = background_image {
        let img_width = bg_img.width();
        let img_height = bg_img.height();

        // Calculate scaling factor to fit while maintaining aspect ratio
        let scale_x = width as f32 / img_width as f32;
        let scale_y = height as f32 / img_height as f32;
        let scale = scale_x.min(scale_y);

        let new_width = (img_width as f32 * scale) as u32;
        let new_height = (img_height as f32 * scale) as u32;

        // Center the image
        let x_offset = (width - new_width) / 2;
        let y_offset = (height - new_height) / 2;

        // Resize and overlay the background image
        let resized = bg_img.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3);
        imageproc::drawing::draw_filled_rect_mut(
            &mut image,
            imageproc::rect::Rect::at(0, 0).of_size(width, height),
            bg_rgba,
        );
        image::imageops::overlay(&mut image, &resized, x_offset as i64, y_offset as i64);
    }

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

                draw_text_mut(&mut image, outline_rgba, x, y, scale, &font, line);
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
        draw_text_mut(&mut image, text_color, x, y, scale, &font, line);

        // Move to next line
        y += line_height as i32;
    }

    // Fix alpha channel: imageproc's draw_text_mut blends with background,
    // so on transparent backgrounds it produces low-alpha pixels.
    // We need to boost the alpha channel to make text visible when overlaid.
    for pixel in image.pixels_mut() {
        if pixel[3] > 0 {
            // If there's any alpha, set it to full opacity
            // The RGB values are already correctly anti-aliased
            pixel[3] = 255;
        }
    }

    let dynamic_image = DynamicImage::ImageRgba8(image);

    // Cache the result
    {
        let mut cache = TEXT_CACHE.lock().unwrap();
        cache.insert(cache_key, dynamic_image.clone());
    }

    Ok(dynamic_image)
}
