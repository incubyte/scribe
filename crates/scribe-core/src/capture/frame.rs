use crate::capture::{InvalidSelection, PixelRect};

const BYTES_PER_PIXEL: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the pixels do not fill a frame of the given size")]
pub struct InvalidFrame;

impl Frame {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Frame, InvalidFrame> {
        let has_area = width > 0 && height > 0;
        if has_area && byte_count(width, height) == Some(rgba.len()) {
            Ok(Frame {
                width,
                height,
                rgba,
            })
        } else {
            Err(InvalidFrame)
        }
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    #[must_use]
    pub fn into_rgba(self) -> Vec<u8> {
        self.rgba
    }

    #[must_use]
    pub fn can_crop(&self, area: PixelRect) -> bool {
        area.is_selectable_within(self.width, self.height)
    }

    pub fn crop(&self, area: PixelRect) -> Result<Frame, InvalidSelection> {
        if !self.can_crop(area) {
            return Err(InvalidSelection);
        }
        let row_length = self.width as usize * BYTES_PER_PIXEL;
        let left = area.x as usize * BYTES_PER_PIXEL;
        let right = left + area.width as usize * BYTES_PER_PIXEL;
        let rgba = self
            .rgba
            .chunks_exact(row_length)
            .skip(area.y as usize)
            .take(area.height as usize)
            .flat_map(|row| &row[left..right])
            .copied()
            .collect();
        Ok(Frame {
            width: area.width,
            height: area.height,
            rgba,
        })
    }
}

fn byte_count(width: u32, height: u32) -> Option<usize> {
    (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(BYTES_PER_PIXEL)
}
