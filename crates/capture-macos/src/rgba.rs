use cidre::cg;
use scribe_core::capture::Frame;
use scribe_core::ports::ScreenCaptureError;

const BITS_PER_COMPONENT: usize = 8;
const BYTES_PER_PIXEL: usize = 4;

pub(crate) fn rgba_of(image: &cg::Image) -> Result<Frame, ScreenCaptureError> {
    let (width, height) = (image.width(), image.height());
    let bytes_per_row = width * BYTES_PER_PIXEL;
    let mut rgba = vec![0_u8; bytes_per_row * height];
    let srgb = cg::ColorSpace::with_name(cg::color_space::names::srgb())
        .ok_or_else(|| failed("no sRGB colour space"))?;
    let mut context = cg::Context::new_bitmap(
        rgba.as_mut_ptr().cast(),
        width,
        height,
        BITS_PER_COMPONENT,
        bytes_per_row,
        &srgb,
        cg::BitmapInfo::with_alpha(cg::ImageAlphaInfo::PremultipliedLast),
    )
    .ok_or_else(|| failed("no bitmap context"))?;
    context.draw_image(whole_image(width, height), image);
    drop(context);
    Frame::new(pixels(width)?, pixels(height)?, rgba).map_err(|error| failed(&error.to_string()))
}

#[allow(clippy::cast_precision_loss)]
fn whole_image(width: usize, height: usize) -> cg::Rect {
    cg::Rect::new(0.0, 0.0, width as cg::Float, height as cg::Float)
}

fn pixels(length: usize) -> Result<u32, ScreenCaptureError> {
    u32::try_from(length).map_err(|_| failed("the image is too large"))
}

fn failed(reason: &str) -> ScreenCaptureError {
    ScreenCaptureError::Failed(reason.to_owned())
}
