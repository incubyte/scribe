use scribe_core::capture::{CaptureOutcome, MonitorPlacement};
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
    let screenshot = outcome.screenshot();
    let image = Image::new(screenshot.rgba(), screenshot.width(), screenshot.height());
    if let Err(error) = app.clipboard().write_image(&image) {
        tracing::error!(%error, "could not copy the screenshot to the clipboard");
    }
    show_notice(app, &notice_for(outcome));
    if matches!(outcome, CaptureOutcome::CopiedOnly { .. }) && !idle_notice::is_dismissed(app) {
        if let Err(error) = idle_notice::show(app, placement) {
            tracing::error!(%error, "could not show the idle capture notice");
        }
    }
}

pub fn show_notice<R: Runtime>(app: &AppHandle<R>, notice: &Notice) {
    notifications::show_plain(app, notice.title, &notice.body);
}
