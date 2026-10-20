pub const MIN_SELECTION_EDGE_PX: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    pub(crate) fn is_selectable_within(&self, width: u32, height: u32) -> bool {
        let is_large_enough =
            self.width >= MIN_SELECTION_EDGE_PX && self.height >= MIN_SELECTION_EDGE_PX;
        let right_fits = self
            .x
            .checked_add(self.width)
            .is_some_and(|right| right <= width);
        let bottom_fits = self
            .y
            .checked_add(self.height)
            .is_some_and(|bottom| bottom <= height);
        is_large_enough && right_fits && bottom_fits
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    WholeMonitor,
    Area(PixelRect),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the selected area is too small or does not lie inside the captured monitor")]
pub struct InvalidSelection;
