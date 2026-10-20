use scribe_core::capture::{
    CaptureFailure, CaptureOutcome, InvalidSelection, MonitorPlacement, PendingCapture, PixelRect,
    ScreenshotTaker, Selection,
};
use scribe_core::recording::LiveSession;
use tauri::{AppHandle, Runtime};

use crate::notifications::notify_screen_recording_needed;
use crate::screenshots::capture_gate::{CaptureGate, RegionRefusal};
use crate::screenshots::delivery::{deliver, show_notice};
use crate::screenshots::notices::capture_failed_notice;
use crate::screenshots::region_selector;

pub struct ScreenshotService {
    taker: ScreenshotTaker,
    gate: CaptureGate,
}

impl ScreenshotService {
    pub fn new(taker: ScreenshotTaker) -> ScreenshotService {
        ScreenshotService {
            taker,
            gate: CaptureGate::default(),
        }
    }

    pub fn take_full_screen<R: Runtime>(&self, app: &AppHandle<R>, live: Option<LiveSession>) {
        let Some(_claim) = self.gate.claim() else {
            return;
        };
        match self.taker.begin(live) {
            Ok(pending) => deliver_finished(app, self.finish(pending, Selection::WholeMonitor)),
            Err(failure) => report_failure(app, failure),
        }
    }

    pub fn begin_region<R: Runtime>(&self, app: &AppHandle<R>, live: Option<LiveSession>) {
        let Some(claim) = self.gate.claim() else {
            return;
        };
        match self.taker.begin(live) {
            Ok(pending) => {
                let placement = pending.shot().placement;
                claim.park(pending);
                self.open_selector(app, &placement);
            }
            Err(failure) => report_failure(app, failure),
        }
    }

    pub fn complete_region<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        area: PixelRect,
    ) -> Result<(), RegionRefusal> {
        let selection = Selection::Area(area);
        let pending = self.gate.take_accepting(selection)?;
        region_selector::close(app);
        deliver_finished(app, self.finish(pending, selection));
        Ok(())
    }

    pub fn cancel_region(&self) {
        self.gate.discard_pending();
    }

    pub fn read_pending<T>(&self, read: impl FnOnce(&PendingCapture) -> T) -> Option<T> {
        self.gate.read_pending(read)
    }

    fn open_selector<R: Runtime>(&self, app: &AppHandle<R>, placement: &MonitorPlacement) {
        if let Err(error) = region_selector::open(app, placement) {
            tracing::error!(%error, "could not open the region selector");
            self.cancel_region();
            region_selector::close(app);
            show_notice(app, &capture_failed_notice());
        }
    }

    fn finish(&self, pending: PendingCapture, selection: Selection) -> Finished {
        let placement = pending.shot().placement;
        Finished {
            cut: self.taker.finish(pending, selection),
            placement,
        }
    }
}

struct Finished {
    cut: Result<CaptureOutcome, InvalidSelection>,
    placement: MonitorPlacement,
}

fn deliver_finished<R: Runtime>(app: &AppHandle<R>, finished: Finished) {
    match finished.cut {
        Ok(outcome) => deliver(app, &outcome, &finished.placement),
        Err(error) => {
            tracing::error!(%error, "could not cut the screenshot from the captured monitor");
            show_notice(app, &capture_failed_notice());
        }
    }
}

fn report_failure<R: Runtime>(app: &AppHandle<R>, failure: CaptureFailure) {
    match failure {
        CaptureFailure::AccessNotGranted => notify_screen_recording_needed(app),
        CaptureFailure::CaptureFailed => show_notice(app, &capture_failed_notice()),
    }
}
