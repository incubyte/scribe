use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

use chrono::Local;
use scribe_core::ports::{
    CaptureError, RecordingListener, StoreError, TranscribeError, TranscriptStore,
};
use scribe_core::recording::{
    Readiness, Recorder, RecorderError, RecorderState, SessionHeader, SessionId,
};
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder};
use tauri::{AppHandle, Manager, Runtime};

use crate::lanes;
use crate::live_feed::LiveFeed;
use crate::model_setup;
use crate::notifications;
use crate::privacy;
use crate::tray_listener::TrayRecordingListener;

pub struct RecordingController {
    recorder: Mutex<Recorder>,
    listener: Arc<dyn RecordingListener>,
}

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error(transparent)]
    Recorder(#[from] RecorderError),
    #[error(transparent)]
    Engine(#[from] TranscribeError),
}

impl StartError {
    #[must_use]
    pub fn is_microphone_denied(&self) -> bool {
        matches!(
            self,
            StartError::Recorder(RecorderError::Capture(CaptureError::PermissionDenied))
        )
    }
}

impl RecordingController {
    #[must_use]
    pub fn new(recorder: Recorder, listener: Arc<dyn RecordingListener>) -> RecordingController {
        RecordingController {
            recorder: Mutex::new(recorder),
            listener,
        }
    }

    pub fn start(&self) -> Result<SessionId, StartError> {
        let mut recorder = self.recorder();
        recorder.ensure_idle()?;
        let lanes = lanes::build_lanes(&self.listener)?;
        let header =
            SessionHeader::recording(Local::now().fixed_offset(), lanes::ENGINE, lanes::LANGUAGE);
        Ok(recorder.start(lanes, header)?)
    }

    pub fn stop(&self) {
        if let Err(error) = self.recorder().stop() {
            tracing::error!(%error, "the recording did not finish cleanly");
        }
    }

    pub fn state(&self) -> RecorderState {
        self.recorder().state()
    }

    pub fn mark_privacy_acknowledged(&self) {
        self.update_readiness(|readiness| readiness.privacy_acknowledged = true);
    }

    pub fn mark_model_ready(&self) {
        self.update_readiness(|readiness| readiness.model_ready = true);
    }

    fn update_readiness(&self, change: impl FnOnce(&mut Readiness)) {
        let mut recorder = self.recorder();
        let mut readiness = recorder.readiness();
        change(&mut readiness);
        recorder.set_readiness(readiness);
    }

    fn recorder(&self) -> MutexGuard<'_, Recorder> {
        self.recorder.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub fn start_in_background<R: Runtime>(app: AppHandle<R>) {
    thread::spawn(move || {
        if let Err(error) = app.state::<RecordingController>().start() {
            report_start_failure(&app, &error);
        }
    });
}

fn report_start_failure<R: Runtime>(app: &AppHandle<R>, error: &StartError) {
    tracing::warn!(%error, "could not start recording");
    if error.is_microphone_denied() {
        notifications::notify_microphone_denied(app);
    }
}

pub fn install<R: Runtime>(app: &AppHandle<R>) -> Result<(), StoreError> {
    let store = Arc::new(MarkdownTranscriptStore::new(
        TranscriptsFolder::default_location()?,
    ));
    mark_abandoned_sessions(&store);
    app.manage(LiveFeed::default());
    let listener: Arc<dyn RecordingListener> = Arc::new(TrayRecordingListener::spawn(app.clone()));
    let mut recorder = Recorder::new(store.clone(), Arc::clone(&listener));
    recorder.set_readiness(Readiness {
        privacy_acknowledged: privacy::is_acknowledged(app),
        model_ready: false,
    });
    app.manage(store);
    app.manage(RecordingController::new(recorder, listener));
    model_setup::install(app);
    stop_before_sleep(app);
    Ok(())
}

fn mark_abandoned_sessions(store: &MarkdownTranscriptStore) {
    match store.mark_abandoned_incomplete() {
        Ok(marked) => tracing::info!(marked, "marked sessions left recording as incomplete"),
        Err(error) => tracing::error!(%error, "could not check for sessions left recording"),
    }
}

#[cfg(any(target_os = "macos", windows))]
fn stop_before_sleep<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    crate::power::on_will_sleep(move || app.state::<RecordingController>().stop());
}

#[cfg(not(any(target_os = "macos", windows)))]
fn stop_before_sleep<R: Runtime>(_app: &AppHandle<R>) {}
