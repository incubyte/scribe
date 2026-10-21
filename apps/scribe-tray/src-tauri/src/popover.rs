use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use tauri::{AppHandle, Manager, Runtime, WebviewWindow, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::blur_guard::BlurGuard;

const WINDOW_LABEL: &str = "popover";

#[cfg(target_os = "macos")]
const FALLBACK_POSITION: Position = Position::TopRight;
#[cfg(not(target_os = "macos"))]
const FALLBACK_POSITION: Position = Position::BottomRight;

pub fn toggle<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(WINDOW_LABEL) else {
        tracing::error!("the popover window is not configured");
        return;
    };
    if window.is_visible().unwrap_or(false) {
        log_failure(window.hide(), "could not hide the popover");
    } else if with_guard(app, |guard| guard.click_may_show(Instant::now())) {
        log_failure(show(app, &window), "could not show the popover");
    }
}

pub fn open_for_call<R: Runtime>(app: &AppHandle<R>) {
    let window = match popover_window(app) {
        Ok(window) => window,
        Err(error) => {
            tracing::error!(%error, "the popover window is not configured");
            return;
        }
    };
    if window.is_visible().unwrap_or(false) {
        return;
    }
    log_failure(place_at_tray(&window), "could not place the popover");
    with_guard(app, |guard| guard.shown_awaiting_focus(Instant::now()));
    if window.show().is_err() {
        tracing::warn!("could not show the popover for a call");
    }
}

pub fn install_blur_hide<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.manage(Mutex::new(BlurGuard::default()));
    let window = popover_window(app)?;
    let handle = app.clone();
    let target = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Focused(false) => hide_after_blur(&handle, &target),
        WindowEvent::Focused(true) => with_guard(&handle, |guard| guard.focused(Instant::now())),
        _ => {}
    });
    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn hide_popover<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let window = popover_window(&app).map_err(|error| error.to_string())?;
    window.hide().map_err(|error| error.to_string())
}

pub fn keep_open_during<R: Runtime, T>(app: &AppHandle<R>, show_dialog: impl FnOnce() -> T) -> T {
    with_guard(app, BlurGuard::dialog_opened);
    let answer = show_dialog();
    with_guard(app, BlurGuard::dialog_closed);
    if let Ok(window) = popover_window(app) {
        log_failure(window.set_focus(), "could not refocus the popover");
    }
    answer
}

fn show<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) -> tauri::Result<()> {
    log_failure(place_at_tray(window), "could not place the popover");
    window.show()?;
    window.set_focus()?;
    with_guard(app, |guard| guard.shown(Instant::now()));
    Ok(())
}

fn place_at_tray<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    window
        .move_window_constrained(Position::TrayCenter)
        .or_else(|error| {
            tracing::debug!(%error, "tray position not known yet, using the fallback position");
            window.move_window(FALLBACK_POSITION)
        })
}

fn hide_after_blur<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) {
    if with_guard(app, |guard| guard.blur_should_hide(Instant::now())) {
        log_failure(window.hide(), "could not hide the popover");
    }
}

fn popover_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<WebviewWindow<R>> {
    app.get_webview_window(WINDOW_LABEL)
        .ok_or(tauri::Error::WindowNotFound)
}

fn with_guard<R: Runtime, T>(app: &AppHandle<R>, act: impl FnOnce(&mut BlurGuard) -> T) -> T {
    let guard = app.state::<Mutex<BlurGuard>>();
    let mut guard = guard.lock().unwrap_or_else(PoisonError::into_inner);
    act(&mut guard)
}

fn log_failure(outcome: tauri::Result<()>, message: &str) {
    if let Err(error) = outcome {
        tracing::warn!(%error, "{message}");
    }
}
