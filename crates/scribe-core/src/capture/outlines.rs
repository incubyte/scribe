use crate::capture::{MonitorPlacement, PixelRect, WindowOutline};

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

#[must_use]
pub fn visible_outlines(
    candidates: &[WindowCandidate],
    placement: &MonitorPlacement,
    frame_width: u32,
    frame_height: u32,
) -> Vec<WindowOutline> {
    let scale_x = f64::from(frame_width) / f64::from(placement.width);
    let scale_y = f64::from(frame_height) / f64::from(placement.height);
    candidates
        .iter()
        .filter(|candidate| candidate.is_app_window && !candidate.is_own)
        .filter_map(|candidate| {
            let horizontal = Span {
                start: candidate.left - f64::from(placement.x),
                end: candidate.left - f64::from(placement.x) + candidate.width,
            }
            .clipped_to(f64::from(placement.width))
            .scaled(scale_x);
            let vertical = Span {
                start: candidate.top - f64::from(placement.y),
                end: candidate.top - f64::from(placement.y) + candidate.height,
            }
            .clipped_to(f64::from(placement.height))
            .scaled(scale_y);
            outline_between(horizontal, vertical)
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
