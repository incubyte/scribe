#![allow(unsafe_code)]

use crate::outlines::window_candidates;
use scribe_core::capture::{
    Frame, MonitorPlacement, MonitorShot, PlacementUnits, visible_outlines,
};
use scribe_core::ports::{CaptureAccess, ScreenCaptureError, ScreenCapturer};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use xcap::{Monitor, XCapError};

#[derive(Debug, Default)]
pub struct DesktopCapturer;

impl DesktopCapturer {
    #[must_use]
    pub fn new() -> DesktopCapturer {
        DesktopCapturer
    }
}

impl ScreenCapturer for DesktopCapturer {
    fn access(&self) -> CaptureAccess {
        CaptureAccess::Granted
    }

    fn request_access(&self) {}

    fn capture_under_pointer(&self) -> Result<MonitorShot, ScreenCaptureError> {
        let pointer = pointer_location()?;
        let monitor = Monitor::from_point(pointer.x, pointer.y)
            .map_err(|_| ScreenCaptureError::NoMonitorUnderPointer)?;
        let windows = window_candidates(std::process::id());
        let image = monitor.capture_image().map_err(failed)?;
        let frame = Frame::new(image.width(), image.height(), image.into_raw())
            .map_err(|error| ScreenCaptureError::Failed(error.to_string()))?;
        let placement = placement_of(&monitor)?;
        Ok(MonitorShot {
            outlines: visible_outlines(&windows, &placement, frame.size()),
            frame,
            placement,
        })
    }
}

fn pointer_location() -> Result<POINT, ScreenCaptureError> {
    let mut pointer = POINT::default();
    unsafe { GetCursorPos(&raw mut pointer) }
        .map_err(|error| ScreenCaptureError::Failed(error.to_string()))?;
    Ok(pointer)
}

fn placement_of(monitor: &Monitor) -> Result<MonitorPlacement, ScreenCaptureError> {
    Ok(MonitorPlacement {
        x: monitor.x().map_err(failed)?,
        y: monitor.y().map_err(failed)?,
        width: monitor.width().map_err(failed)?,
        height: monitor.height().map_err(failed)?,
        units: PlacementUnits::Pixels,
    })
}

#[allow(clippy::needless_pass_by_value)]
fn failed(error: XCapError) -> ScreenCaptureError {
    ScreenCaptureError::Failed(error.to_string())
}
