mod destination;
mod frame;
mod geometry;
mod monitor;
mod reference;
mod screenshot_name;
mod taker;

pub use destination::{Destination, SessionMoment};
pub use frame::{Frame, InvalidFrame};
pub use geometry::{InvalidSelection, PixelRect, Selection};
pub use monitor::{MonitorPlacement, MonitorShot, PlacementUnits, WindowOutline};
pub use reference::{ScreenshotReference, ServedImage};
pub use screenshot_name::{InvalidScreenshotName, ScreenshotName};
pub use taker::{CaptureFailure, CaptureOutcome, PendingCapture, ScreenshotTaker};
