use std::sync::atomic::{AtomicBool, Ordering};

use scribe_core::capture::{CaptureFailure, PendingCapture, ScreenshotTaker, Selection};
use scribe_core::recording::LiveSession;
use tauri::{AppHandle, Runtime};

use crate::notifications::notify_screen_recording_needed;
use crate::screenshots::delivery::{deliver, show_notice};
use crate::screenshots::notices::capture_failed_notice;

pub struct ScreenshotService {
    taker: ScreenshotTaker,
    busy: AtomicBool,
}

impl ScreenshotService {
    pub fn new(taker: ScreenshotTaker) -> ScreenshotService {
        ScreenshotService {
            taker,
            busy: AtomicBool::new(false),
        }
    }

    pub fn take_full_screen<R: Runtime>(&self, app: &AppHandle<R>, live: Option<LiveSession>) {
        let Some(_claim) = BusyClaim::take(&self.busy) else {
            return;
        };
        match self.taker.begin(live) {
            Ok(pending) => self.finish_whole_monitor(app, pending),
            Err(failure) => report_failure(app, failure),
        }
    }

    fn finish_whole_monitor<R: Runtime>(&self, app: &AppHandle<R>, pending: PendingCapture) {
        match self.taker.finish(pending, Selection::WholeMonitor) {
            Ok(outcome) => deliver(app, &outcome),
            Err(error) => {
                tracing::error!(%error, "could not take the whole monitor");
                show_notice(app, &capture_failed_notice());
            }
        }
    }
}

fn report_failure<R: Runtime>(app: &AppHandle<R>, failure: CaptureFailure) {
    match failure {
        CaptureFailure::AccessNotGranted => notify_screen_recording_needed(app),
        CaptureFailure::CaptureFailed => show_notice(app, &capture_failed_notice()),
    }
}

struct BusyClaim<'a>(&'a AtomicBool);

impl<'a> BusyClaim<'a> {
    fn take(busy: &'a AtomicBool) -> Option<BusyClaim<'a>> {
        busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| BusyClaim(busy))
    }
}

impl Drop for BusyClaim<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
