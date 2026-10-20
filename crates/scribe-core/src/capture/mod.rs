mod destination;
mod frame;
mod geometry;
mod monitor;
mod outlines;
mod reference;
mod screenshot_name;
mod taker;

pub use destination::{Destination, SessionMoment};
pub use frame::{Frame, FrameSize, InvalidFrame};
pub use geometry::{InvalidSelection, MIN_SELECTION_EDGE_PX, PixelRect, Selection};
pub use monitor::{MonitorPlacement, MonitorShot, PlacementUnits, WindowOutline};
pub use outlines::{WindowCandidate, visible_outlines};
pub use reference::{InvalidScreenshotReference, ScreenshotReference, ServedImage};
pub use screenshot_name::{InvalidScreenshotName, ScreenshotName};
pub use taker::{CaptureFailure, CaptureOutcome, PendingCapture, ScreenshotTaker};
