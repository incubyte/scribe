use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;

use stt_parakeet::{DownloadError, HttpsFetcher, ModelProvisioner, default_models_dir};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use super::ModelDownloadView;
use crate::recording::RecordingController;
use crate::tray;

const PROGRESSED_EVENT: &str = "model-download://progressed";
const COMPLETED_EVENT: &str = "model-download://completed";
const FAILED_EVENT: &str = "model-download://failed";
const FAILED_TOOLTIP: &str = "Scribe — speech model download failed";

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
        match provision(&app) {
            Ok(()) => show_ready(&app),
            Err(error) => show_failure(&app, &error),
        }
        app.state::<ModelSetup>().end_attempt();
    });
}

fn provision<R: Runtime>(app: &AppHandle<R>) -> Result<(), DownloadError> {
    let provisioner = ModelProvisioner::new(default_models_dir()?, Box::new(HttpsFetcher::new()?));
    if provisioner.is_ready() {
        return Ok(());
    }
    show_progress(app, 0);
    provisioner.provision(&|progress| show_progress(app, progress.percent()))
}

fn show_progress<R: Runtime>(app: &AppHandle<R>, percent: u8) {
    let view = ModelDownloadView::Downloading { percent };
    if app.state::<ModelSetup>().change_view(&view) {
        tray::show_tooltip(
            app,
            &format!("Scribe — downloading speech model {percent}%"),
        );
        emit(app, PROGRESSED_EVENT, &view);
    }
}

fn show_ready<R: Runtime>(app: &AppHandle<R>) {
    app.state::<ModelSetup>()
        .change_view(&ModelDownloadView::Ready);
    let controller = app.state::<RecordingController>();
    controller.mark_model_ready();
    tray::show_idle_tooltip(app);
    tray::show_recorder_state(app, controller.state());
    emit(app, COMPLETED_EVENT, &ModelDownloadView::Ready);
}

fn show_failure<R: Runtime>(app: &AppHandle<R>, error: &DownloadError) {
    tracing::error!(kind = error.kind(), "the speech model download failed");
    let view = ModelDownloadView::Failed {
        reason_kind: error.kind(),
    };
    app.state::<ModelSetup>().change_view(&view);
    tray::show_tooltip(app, FAILED_TOOLTIP);
    emit(app, FAILED_EVENT, &view);
}

fn emit<R: Runtime>(app: &AppHandle<R>, event: &str, view: &ModelDownloadView) {
    if let Err(error) = app.emit(event, view) {
        tracing::warn!(event, %error, "could not notify the webview");
    }
}
