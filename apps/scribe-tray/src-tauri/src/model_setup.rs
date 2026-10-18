#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};

#[cfg(target_os = "macos")]
use macos::prepare_speech_engine;
#[cfg(not(any(target_os = "macos", windows)))]
use unsupported::prepare_speech_engine;
#[cfg(windows)]
use windows::prepare_speech_engine;

use crate::lanes::SpeechEngine;
use crate::recording::{self, RecordingController};
use crate::tray;
use crate::webview;

const PROGRESSED_EVENT: &str = "model-download://progressed";
const COMPLETED_EVENT: &str = "model-download://completed";
const FAILED_EVENT: &str = "model-download://failed";
const PREPARING_TOOLTIP: &str = "Scribe — preparing speech model…";
const FAILED_TOOLTIP: &str = "Scribe — speech model setup failed";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum ModelDownloadView {
    Ready,
    Downloading { percent: Option<u8> },
    Failed { reason_kind: &'static str },
}

pub struct SetupFailed {
    pub reason_kind: &'static str,
}

struct ModelSetup {
    view: Mutex<ModelDownloadView>,
    attempt_running: AtomicBool,
}

impl ModelSetup {
    fn new() -> ModelSetup {
        ModelSetup {
            view: Mutex::new(ModelDownloadView::Ready),
            attempt_running: AtomicBool::new(false),
        }
    }

    fn begin_attempt(&self) -> bool {
        self.attempt_running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn end_attempt(&self) {
        self.attempt_running.store(false, Ordering::Release);
    }

    fn change_view(&self, next: &ModelDownloadView) -> bool {
        let mut view = self.view();
        let changed = *view != *next;
        view.clone_from(next);
        changed
    }

    fn current_view(&self) -> ModelDownloadView {
        self.view().clone()
    }

    fn view(&self) -> MutexGuard<'_, ModelDownloadView> {
        self.view.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    app.manage(ModelSetup::new());
    spawn_attempt(app);
}

pub fn status<R: Runtime>(app: &AppHandle<R>) -> ModelDownloadView {
    app.state::<ModelSetup>().current_view()
}

pub fn retry<R: Runtime>(app: &AppHandle<R>) {
    if matches!(status(app), ModelDownloadView::Failed { .. }) {
        spawn_attempt(app);
    }
}

fn spawn_attempt<R: Runtime>(app: &AppHandle<R>) {
    if !app.state::<ModelSetup>().begin_attempt() {
        return;
    }
    let app = app.clone();
    thread::spawn(move || {
        match prepare_speech_engine(&|percent| show_progress(&app, percent)) {
            Ok(engine) => show_ready(&app, engine),
            Err(failure) => show_failure(&app, &failure),
        }
        app.state::<ModelSetup>().end_attempt();
    });
}

fn show_progress<R: Runtime>(app: &AppHandle<R>, percent: Option<u8>) {
    let view = ModelDownloadView::Downloading { percent };
    if app.state::<ModelSetup>().change_view(&view) {
        tray::show_tooltip(app, &progress_tooltip(percent));
        webview::emit(app, PROGRESSED_EVENT, view);
    }
}

fn progress_tooltip(percent: Option<u8>) -> String {
    percent.map_or_else(
        || PREPARING_TOOLTIP.to_owned(),
        |percent| format!("Scribe — downloading speech model {percent}%"),
    )
}

fn show_ready<R: Runtime>(app: &AppHandle<R>, engine: SpeechEngine) {
    app.state::<ModelSetup>()
        .change_view(&ModelDownloadView::Ready);
    app.state::<RecordingController>().mark_model_ready(engine);
    tray::show_idle_tooltip(app);
    recording::publish_state(app);
    webview::emit(app, COMPLETED_EVENT, ModelDownloadView::Ready);
}

fn show_failure<R: Runtime>(app: &AppHandle<R>, failure: &SetupFailed) {
    tracing::error!(
        kind = failure.reason_kind,
        "the speech model is not available"
    );
    let view = ModelDownloadView::Failed {
        reason_kind: failure.reason_kind,
    };
    app.state::<ModelSetup>().change_view(&view);
    tray::show_tooltip(app, FAILED_TOOLTIP);
    webview::emit(app, FAILED_EVENT, view);
}
