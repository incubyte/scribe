use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder};
use scribe_core::capture::Frame;
use scribe_core::ports::StoreError;

pub(crate) fn encode_png(
    frame: &Frame,
    compression: CompressionType,
) -> Result<Vec<u8>, StoreError> {
    let mut png = Vec::new();
    PngEncoder::new_with_quality(&mut png, compression, FilterType::Adaptive)
        .write_image(
            frame.rgba(),
            frame.width(),
            frame.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|_| StoreError::ScreenshotNotEncoded)?;
    Ok(png)
}
