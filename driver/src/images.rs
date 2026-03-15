use image::codecs::bmp::BmpEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{ColorType, DynamicImage, GenericImageView, ImageEncoder, ImageError, Rgba};

use crate::error::MirajazzError;
use crate::types::{ImageFormat, ImageMirroring, ImageMode, ImageRotation};

/// Composites RGBA image onto black background, returning RGB pixel data.
/// Properly handles alpha blending so transparent pixels become black.
fn flatten_to_rgb(image: &DynamicImage) -> Vec<u8> {
    let rgba = image.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for pixel in rgba.pixels() {
        let Rgba([r, g, b, a]) = *pixel;
        let alpha = a as f32 / 255.0;
        rgb.push((r as f32 * alpha) as u8);
        rgb.push((g as f32 * alpha) as u8);
        rgb.push((b as f32 * alpha) as u8);
    }
    rgb
}

/// Applies resize, rotation and mirroring transformations to an image
fn transform_image(image: DynamicImage, size: (usize, usize), rotation: ImageRotation, mirror: ImageMirroring) -> DynamicImage {
    let (ws, hs) = size;
    let image = image.resize_exact(ws as u32, hs as u32, FilterType::Nearest);

    let image = match rotation {
        ImageRotation::Rot0 => image,
        ImageRotation::Rot90 => image.rotate90(),
        ImageRotation::Rot180 => image.rotate180(),
        ImageRotation::Rot270 => image.rotate270(),
    };

    match mirror {
        ImageMirroring::None => image,
        ImageMirroring::X => image.fliph(),
        ImageMirroring::Y => image.flipv(),
        ImageMirroring::Both => image.fliph().flipv(),
    }
}

/// Converts image into image data depending on provided image format
pub fn convert_image_with_format(
    image_format: ImageFormat,
    image: DynamicImage,
) -> Result<Vec<u8>, ImageError> {
    let image = transform_image(image, image_format.size, image_format.rotation, image_format.mirror);
    let (w, h) = image.dimensions();

    match image_format.mode {
        ImageMode::None => Ok(vec![]),
        ImageMode::BMP | ImageMode::JPEG => {
            // Composite RGBA onto black background so transparent pixels become black
            let image_data = flatten_to_rgb(&image);
            let mut buf = Vec::new();
            if matches!(image_format.mode, ImageMode::BMP) {
                let mut encoder = BmpEncoder::new(&mut buf);
                encoder.encode(&image_data, w, h, ColorType::Rgb8.into())?;
            } else {
                let mut encoder = JpegEncoder::new_with_quality(&mut buf, 90);
                encoder.encode(&image_data, w, h, ColorType::Rgb8.into())?;
            }
            Ok(buf)
        }
        ImageMode::PNG => {
            let image_data = image.into_rgba8().to_vec();
            let mut buf = Vec::new();
            let encoder = PngEncoder::new(&mut buf);
            encoder.write_image(&image_data, w, h, ColorType::Rgba8.into())?;
            Ok(buf)
        }
    }
}

/// Rect to be used when trying to send image to lcd screen
pub struct ImageRect {
    /// Width of the image
    pub w: u16,

    /// Height of the image
    pub h: u16,

    /// Data of the image row by row as RGB
    pub data: Vec<u8>,
}

impl ImageRect {
    /// Converts image to image rect
    pub fn from_image(image: DynamicImage) -> Result<ImageRect, MirajazzError> {
        let (image_w, image_h) = image.dimensions();

        let image_data = image.into_rgb8().to_vec();

        let mut buf = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut buf, 90);
        encoder.encode(&image_data, image_w, image_h, ColorType::Rgb8.into())?;

        Ok(ImageRect {
            w: image_w as u16,
            h: image_h as u16,
            data: buf,
        })
    }
}
