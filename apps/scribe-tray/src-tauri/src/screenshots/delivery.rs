use scribe_core::capture::{CaptureOutcome, Frame, MonitorPlacement};
use tauri::image::Image;
use tauri::{AppHandle, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::notifications;
use crate::screenshots::idle_notice;
use crate::screenshots::notices::{Notice, notice_for};

pub fn deliver<R: Runtime>(
    app: &AppHandle<R>,
    outcome: &CaptureOutcome,
    placement: &MonitorPlacement,
) {
    copy_to_clipboard(app, outcome.screenshot());
    show_notice(app, &notice_for(outcome));
    if matches!(outcome, CaptureOutcome::CopiedOnly { .. }) {
        show_idle_notice_unless_dismissed(app, placement);
    }
}

fn copy_to_clipboard<R: Runtime>(app: &AppHandle<R>, screenshot: &Frame) {
    let image = Image::new(screenshot.rgba(), screenshot.width(), screenshot.height());
    if let Err(error) = app.clipboard().write_image(&image) {
        tracing::error!(%error, "could not copy the screenshot to the clipboard");
    }
}

fn show_idle_notice_unless_dismissed<R: Runtime>(app: &AppHandle<R>, placement: &MonitorPlacement) {
    if idle_notice::is_dismissed(app) {
        return;
    }
    if let Err(error) = idle_notice::show(app, placement) {
        tracing::error!(%error, "could not show the idle capture notice");
    }
}

pub fn show_notice<R: Runtime>(app: &AppHandle<R>, notice: &Notice) {
    notifications::show_plain(app, notice.title, &notice.body);
}
