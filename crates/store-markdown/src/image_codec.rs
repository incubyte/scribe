use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::imageops::{self, FilterType as ResizeFilter};
use image::{ExtendedColorType, ImageEncoder, ImageFormat, RgbaImage};
use scribe_core::capture::{Frame, ServedImage};
use scribe_core::ports::StoreError;

pub(crate) fn encode_png(
    frame: &Frame,
    compression: CompressionType,
) -> Result<Vec<u8>, StoreError> {
    encode_rgba(frame.rgba(), (frame.width(), frame.height()), compression)
}

pub(crate) fn scale_png_to_fit(png: &[u8], max_long_edge: u32) -> Option<ServedImage> {
    let original = image::load_from_memory_with_format(png, ImageFormat::Png)
        .ok()?
        .to_rgba8();
    let (original_width, original_height) = original.dimensions();
    let (width, height) = fit_long_edge(original_width, original_height, max_long_edge);
    let fitted = resized(original, width, height);
    let png = encode_rgba(fitted.as_raw(), (width, height), CompressionType::Fast).ok()?;
    Some(ServedImage {
        png,
        width,
        height,
        original_width,
        original_height,
    })
}

pub(crate) fn fit_long_edge(width: u32, height: u32, max_long_edge: u32) -> (u32, u32) {
    let long_edge = width.max(height);
    if long_edge <= max_long_edge {
        return (width, height);
    }
    (
        scaled_edge(width, max_long_edge, long_edge),
        scaled_edge(height, max_long_edge, long_edge),
    )
}

fn scaled_edge(edge: u32, max_long_edge: u32, long_edge: u32) -> u32 {
    let long_edge = u64::from(long_edge);
    let scaled = (u64::from(edge) * u64::from(max_long_edge) + long_edge / 2) / long_edge;
    u32::try_from(scaled).unwrap_or(max_long_edge).max(1)
}

fn resized(original: RgbaImage, width: u32, height: u32) -> RgbaImage {
    if original.dimensions() == (width, height) {
        original
    } else {
        imageops::resize(&original, width, height, ResizeFilter::Lanczos3)
    }
}

fn encode_rgba(
    rgba: &[u8],
    (width, height): (u32, u32),
    compression: CompressionType,
) -> Result<Vec<u8>, StoreError> {
    let mut png = Vec::new();
    PngEncoder::new_with_quality(&mut png, compression, FilterType::Adaptive)
        .write_image(rgba, width, height, ExtendedColorType::Rgba8)
        .map_err(|_| StoreError::ScreenshotNotEncoded)?;
    Ok(png)
}
