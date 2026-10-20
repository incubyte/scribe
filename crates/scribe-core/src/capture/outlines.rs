use crate::capture::{FrameSize, MonitorPlacement, PixelRect, WindowOutline};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowCandidate {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub is_app_window: bool,
    pub is_own: bool,
}

#[derive(Debug, Clone, Copy)]
struct Span {
    start: f64,
    end: f64,
}

impl Span {
    fn clipped_to(self, length: f64) -> Span {
        Span {
            start: self.start.clamp(0.0, length),
            end: self.end.clamp(0.0, length),
        }
    }

    fn scaled(self, factor: f64) -> (u32, u32) {
        (to_pixel(self.start * factor), to_pixel(self.end * factor))
    }
}

#[derive(Debug, Clone, Copy)]
struct Axis {
    origin: f64,
    extent: f64,
    scale: f64,
}

impl Axis {
    fn pixels_of(self, start: f64, length: f64) -> (u32, u32) {
        let relative_start = start - self.origin;
        Span {
            start: relative_start,
            end: relative_start + length,
        }
        .clipped_to(self.extent)
        .scaled(self.scale)
    }
}

#[must_use]
pub fn visible_outlines(
    candidates: &[WindowCandidate],
    placement: &MonitorPlacement,
    frame_size: FrameSize,
) -> Vec<WindowOutline> {
    let horizontal = Axis {
        origin: f64::from(placement.x),
        extent: f64::from(placement.width),
        scale: f64::from(frame_size.width) / f64::from(placement.width),
    };
    let vertical = Axis {
        origin: f64::from(placement.y),
        extent: f64::from(placement.height),
        scale: f64::from(frame_size.height) / f64::from(placement.height),
    };
    candidates
        .iter()
        .filter(|candidate| candidate.is_app_window && !candidate.is_own)
        .filter_map(|candidate| {
            outline_between(
                horizontal.pixels_of(candidate.left, candidate.width),
                vertical.pixels_of(candidate.top, candidate.height),
            )
        })
        .collect()
}

fn outline_between(horizontal: (u32, u32), vertical: (u32, u32)) -> Option<WindowOutline> {
    let (left, right) = horizontal;
    let (top, bottom) = vertical;
    (right > left && bottom > top).then_some(WindowOutline {
        bounds: PixelRect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        },
    })
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_pixel(value: f64) -> u32 {
    value.round() as u32
}
