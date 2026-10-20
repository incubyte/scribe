use std::sync::mpsc::{self, SyncSender};
use std::time::Duration;

use cidre::{arc, blocks, cg, ns, sc};
use scribe_core::capture::{MonitorPlacement, MonitorShot, PlacementUnits};
use scribe_core::ports::{CaptureAccess, ScreenCaptureError, ScreenCapturer};

use crate::rgba::rgba_of;

const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);

type ShotDelivery = SyncSender<Result<MonitorShot, ScreenCaptureError>>;

#[derive(Debug, Default)]
pub struct ScreenCaptureKitCapturer;

impl ScreenCaptureKitCapturer {
    #[must_use]
    pub fn new() -> ScreenCaptureKitCapturer {
        ScreenCaptureKitCapturer
    }
}

impl ScreenCapturer for ScreenCaptureKitCapturer {
    fn access(&self) -> CaptureAccess {
        if cg::screen_capture_access::preflight() {
            CaptureAccess::Granted
        } else {
            CaptureAccess::NotGranted
        }
    }

    fn request_access(&self) {
        let granted = cg::screen_capture_access::request();
        tracing::info!(granted, "asked macOS for screen recording access");
    }

    fn capture_under_pointer(&self) -> Result<MonitorShot, ScreenCaptureError> {
        let pointer = pointer_location()?;
        let (delivery, delivered) = mpsc::sync_channel(1);
        sc::ShareableContent::current_with_ch(move |content, error| {
            let display = content
                .ok_or_else(|| capture_error(error))
                .and_then(|content| display_under(content, pointer));
            capture_or_report(display, &delivery);
        });
        delivered
            .recv_timeout(CAPTURE_TIMEOUT)
            .unwrap_or_else(|_| Err(failed("ScreenCaptureKit did not answer in time")))
    }
}

fn pointer_location() -> Result<cg::Point, ScreenCaptureError> {
    cg::Event::with_src(None)
        .map(|event| event.location())
        .ok_or_else(|| failed("the pointer location is unavailable"))
}

fn capture_or_report(
    display: Result<arc::R<sc::Display>, ScreenCaptureError>,
    delivery: &ShotDelivery,
) {
    match display {
        Ok(display) => capture_display(&display, delivery.clone()),
        Err(error) => {
            let _ = delivery.send(Err(error));
        }
    }
}

fn display_under(
    content: &sc::ShareableContent,
    pointer: cg::Point,
) -> Result<arc::R<sc::Display>, ScreenCaptureError> {
    content
        .displays()
        .iter()
        .find(|display| display.frame().contains_point(&pointer))
        .map(sc::Display::retained)
        .ok_or(ScreenCaptureError::NoMonitorUnderPointer)
}

fn capture_display(display: &sc::Display, delivery: ShotDelivery) {
    let placement = placement_of(display.frame());
    let filter = sc::ContentFilter::with_display_excluding_windows(display, &ns::Array::new());
    let config = native_size_config(&filter);
    let mut on_captured = blocks::ResultCh::new2(
        move |image: Option<&cg::Image>, error: Option<&ns::Error>| {
            let shot = image
                .ok_or_else(|| capture_error(error))
                .and_then(rgba_of)
                .map(|frame| MonitorShot {
                    frame,
                    outlines: Vec::new(),
                    placement,
                });
            let _ = delivery.send(shot);
        },
    );
    sc::ScreenshotManager::capture_image_ch(&filter, &config, Some(&mut on_captured));
}

fn native_size_config(filter: &sc::ContentFilter) -> arc::R<sc::StreamCfg> {
    let size_in_points = filter.content_rect().size;
    let scale = cg::Float::from(filter.point_pixel_scale());
    let mut config = sc::StreamCfg::new();
    config.set_width(native_pixels(size_in_points.width, scale));
    config.set_height(native_pixels(size_in_points.height, scale));
    config.set_shows_cursor(false);
    config
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn native_pixels(points: cg::Float, scale: cg::Float) -> usize {
    (points * scale).round() as usize
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn placement_of(frame: cg::Rect) -> MonitorPlacement {
    MonitorPlacement {
        x: frame.origin.x.round() as i32,
        y: frame.origin.y.round() as i32,
        width: frame.size.width.round() as u32,
        height: frame.size.height.round() as u32,
        units: PlacementUnits::Points,
    }
}

fn capture_error(error: Option<&ns::Error>) -> ScreenCaptureError {
    match error.map(ns::Error::code) {
        Some(sc::error::code::USER_DECLINED) => ScreenCaptureError::AccessNotGranted,
        Some(code) => failed(&format!("ScreenCaptureKit error {code}")),
        None => failed("ScreenCaptureKit returned nothing"),
    }
}

fn failed(reason: &str) -> ScreenCaptureError {
    ScreenCaptureError::Failed(reason.to_owned())
}
