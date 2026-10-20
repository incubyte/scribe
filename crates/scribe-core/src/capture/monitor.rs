use crate::capture::{Frame, PixelRect};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorShot {
    pub frame: Frame,
    pub outlines: Vec<WindowOutline>,
    pub placement: MonitorPlacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorPlacement {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub units: PlacementUnits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementUnits {
    Points,
    Pixels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowOutline {
    pub bounds: PixelRect,
}
