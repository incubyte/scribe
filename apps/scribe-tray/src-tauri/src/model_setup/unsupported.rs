use tauri::{AppHandle, Runtime};

use super::ModelDownloadView;

pub fn install<R: Runtime>(_app: &AppHandle<R>) {}

pub fn status<R: Runtime>(_app: &AppHandle<R>) -> ModelDownloadView {
    ModelDownloadView::Ready
}

pub fn retry<R: Runtime>(_app: &AppHandle<R>) {}
