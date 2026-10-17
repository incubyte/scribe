use std::thread;

use tauri::{AppHandle, Manager, Runtime};

use super::ModelDownloadView;
use crate::lanes;
use crate::recording::RecordingController;
use crate::tray;

const PREPARING_MODEL: &str = "Preparing speech model…";

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    thread::spawn(move || {
        let controller = app.state::<RecordingController>();
        match lanes::ensure_speech_model(&|| tray::show_tooltip(&app, PREPARING_MODEL)) {
            Ok(()) => controller.mark_model_ready(),
            Err(error) => tracing::error!(%error, "the speech model is not available"),
        }
        tray::show_idle_tooltip(&app);
        tray::show_recorder_state(&app, controller.state());
    });
}

pub fn status<R: Runtime>(_app: &AppHandle<R>) -> ModelDownloadView {
    ModelDownloadView::Ready
}

pub fn retry<R: Runtime>(_app: &AppHandle<R>) {}
