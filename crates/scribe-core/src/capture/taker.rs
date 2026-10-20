use std::sync::Arc;

use crate::capture::{
    Destination, Frame, InvalidSelection, MonitorShot, ScreenshotName, Selection, SessionMoment,
};
use crate::ports::{ScreenCapturer, ScreenshotStore, StoreError};
use crate::recording::{LiveSession, SessionId};

pub struct ScreenshotTaker {
    capturer: Arc<dyn ScreenCapturer>,
    store: Arc<dyn ScreenshotStore>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCapture {
    shot: MonitorShot,
    destination: Destination,
}

impl PendingCapture {
    #[must_use]
    pub fn shot(&self) -> &MonitorShot {
        &self.shot
    }

    #[must_use]
    pub fn destination(&self) -> &Destination {
        &self.destination
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureOutcome {
    SavedIntoSession {
        screenshot: Frame,
        name: ScreenshotName,
    },
    CopiedOnly {
        screenshot: Frame,
    },
    CopiedButNotSaved {
        screenshot: Frame,
    },
}

impl CaptureOutcome {
    #[must_use]
    pub fn screenshot(&self) -> &Frame {
        match self {
            CaptureOutcome::SavedIntoSession { screenshot, .. }
            | CaptureOutcome::CopiedOnly { screenshot }
            | CaptureOutcome::CopiedButNotSaved { screenshot } => screenshot,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CaptureFailure {
    #[error("screen recording access has not been granted")]
    AccessNotGranted,
    #[error("the screen could not be captured")]
    CaptureFailed,
}

impl ScreenshotTaker {
    #[must_use]
    pub fn new(
        capturer: Arc<dyn ScreenCapturer>,
        store: Arc<dyn ScreenshotStore>,
    ) -> ScreenshotTaker {
        ScreenshotTaker { capturer, store }
    }

    pub fn begin(&self, live: Option<LiveSession>) -> Result<PendingCapture, CaptureFailure> {
        let destination = Destination::at_press(live);
        let shot = self.capturer.capture_under_pointer().map_err(|error| {
            tracing::warn!(%error, "could not capture the screen");
            CaptureFailure::CaptureFailed
        })?;
        Ok(PendingCapture { shot, destination })
    }

    pub fn finish(
        &self,
        pending: PendingCapture,
        selection: Selection,
    ) -> Result<CaptureOutcome, InvalidSelection> {
        let screenshot = cut(pending.shot.frame, selection)?;
        Ok(match pending.destination {
            Destination::IntoSession(moment) => self.save(&moment, screenshot),
            Destination::ClipboardOnly => CaptureOutcome::CopiedOnly { screenshot },
        })
    }

    fn save(&self, moment: &SessionMoment, screenshot: Frame) -> CaptureOutcome {
        match self.store.save(moment, &screenshot) {
            Ok(name) => CaptureOutcome::SavedIntoSession { screenshot, name },
            Err(error) => {
                log_unsaved(&moment.session, &error);
                CaptureOutcome::CopiedButNotSaved { screenshot }
            }
        }
    }
}

fn cut(frame: Frame, selection: Selection) -> Result<Frame, InvalidSelection> {
    match selection {
        Selection::WholeMonitor => Ok(frame),
        Selection::Area(area) => frame.crop(area),
    }
}

fn log_unsaved(session: &SessionId, error: &StoreError) {
    match error {
        StoreError::Io { path, source } => {
            let file = path.file_name().unwrap_or_default().to_string_lossy();
            tracing::error!(%session, %file, error = %source, "could not save the screenshot");
        }
        other => tracing::error!(%session, error = %other, "could not save the screenshot"),
    }
}
