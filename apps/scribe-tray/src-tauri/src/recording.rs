use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread;

use chrono::Local;
use scribe_core::ports::{RecordingListener, StoreError, TranscriptStore};
use scribe_core::recording::{
    CapturingLanes, LiveSession, LiveSessionSlot, Readiness, Recorder, RecorderError,
    RecorderState, SessionHeader, SessionId,
};
use serde::Serialize;
use store_markdown::{MarkdownTranscriptStore, SETTINGS_FILE_NAME, TranscriptsFolder};
use tauri::{AppHandle, Manager, Runtime};

use crate::lanes::{self, SpeechEngine};
use crate::live_feed::{LiveFeed, state_name};
use crate::model_setup;
use crate::notifications;
use crate::privacy;
use crate::start_failure::StartError;
use crate::tray;
use crate::tray_listener::TrayRecordingListener;
use crate::webview;

const STATE_EVENT: &str = "recorder://state";
const START_FAILED_EVENT: &str = "recording://start-failed";

pub struct RecordingController {
    recorder: Mutex<Recorder>,
    live: LiveSessionSlot,
    listener: Arc<dyn RecordingListener>,
    engine: OnceLock<SpeechEngine>,
    starting: AtomicBool,
}

impl RecordingController {
    #[must_use]
    pub fn new(recorder: Recorder, listener: Arc<dyn RecordingListener>) -> RecordingController {
        RecordingController {
            live: recorder.live_session_slot(),
            recorder: Mutex::new(recorder),
            listener,
            engine: OnceLock::new(),
            starting: AtomicBool::new(false),
        }
    }

    pub fn start(&self) -> Result<SessionId, StartError> {
        let _claim = StartClaim::take(&self.starting)?;
        self.recorder().ensure_idle()?;
        let engine = self.engine.get().ok_or(RecorderError::NotReady)?;
        let capturing = CapturingLanes::start(lanes::build_lanes(&self.listener, engine)?)?;
        let header =
            SessionHeader::recording(Local::now().fixed_offset(), lanes::ENGINE, lanes::LANGUAGE);
        Ok(self.recorder().record(capturing, header)?)
    }

    pub fn stop(&self) {
        log_unclean_finish(self.recorder().stop());
    }

    pub fn stop_session(&self, id: &SessionId) {
        log_unclean_finish(self.recorder().stop_session(id));
    }

    pub fn stop_if_recording(&self, id: &SessionId) -> bool {
        let mut recorder = self.recorder();
        recorder.is_recording_session(id) && finished_cleanly(recorder.stop())
    }

    pub fn state(&self) -> RecorderState {
        self.recorder().state()
    }

    pub fn live_session(&self) -> Option<LiveSession> {
        self.live.current()
    }

    pub fn mark_privacy_acknowledged(&self) {
        self.update_readiness(|readiness| readiness.privacy_acknowledged = true);
    }

    pub fn mark_model_ready(&self, engine: SpeechEngine) {
        if self.engine.set(engine).is_err() {
            tracing::info!("the speech engine was already prepared");
        }
        self.update_readiness(|readiness| readiness.model_ready = true);
    }

    pub fn relocate_store(
        &self,
        store: &MarkdownTranscriptStore,
        folder: TranscriptsFolder,
    ) -> Result<(), RecorderError> {
        let recorder = self.recorder();
        if recorder.state() == RecorderState::Recording {
            return Err(RecorderError::AlreadyRecording);
        }
        store.relocate(folder);
        drop(recorder);
        Ok(())
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

fn finished_cleanly(outcome: Result<(), RecorderError>) -> bool {
    let clean = outcome.is_ok();
    log_unclean_finish(outcome);
    clean
}

fn log_unclean_finish(outcome: Result<(), RecorderError>) {
    if let Err(error) = outcome {
        tracing::error!(%error, "the recording did not finish cleanly");
    }
}

struct StartClaim<'a>(&'a AtomicBool);

impl<'a> StartClaim<'a> {
    fn take(starting: &'a AtomicBool) -> Result<StartClaim<'a>, RecorderError> {
        starting
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| StartClaim(starting))
            .map_err(|_| RecorderError::AlreadyRecording)
    }
}

impl Drop for StartClaim<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[derive(Clone, Serialize)]
struct StatePayload {
    state: &'static str,
}

#[derive(Clone, Serialize)]
struct StartFailedPayload {
    reason_kind: &'static str,
}

pub fn publish_state<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<RecordingController>().state();
    tray::show_recorder_state(app, state);
    let payload = StatePayload {
        state: state_name(state),
    };
    webview::emit(app, STATE_EVENT, payload);
}

pub fn start_in_background<R: Runtime>(app: AppHandle<R>) {
    thread::spawn(move || {
        if let Err(error) = app.state::<RecordingController>().start() {
            report_start_failure(&app, &error);
        }
    });
}

fn report_start_failure<R: Runtime>(app: &AppHandle<R>, error: &StartError) {
    let kind = error.kind();
    tracing::warn!(kind = kind.as_str(), "could not start recording");
    notifications::notify_recording_not_started(app, kind);
    let payload = StartFailedPayload {
        reason_kind: kind.as_str(),
    };
    webview::emit(app, START_FAILED_EVENT, payload);
}

pub fn install<R: Runtime>(app: &AppHandle<R>) -> Result<(), StoreError> {
    let store = Arc::new(MarkdownTranscriptStore::new(
        TranscriptsFolder::configured_by(settings_path(app).as_deref())?,
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

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    let app_data = app.path().app_data_dir().ok()?;
    Some(app_data.join(SETTINGS_FILE_NAME))
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
