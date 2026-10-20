use scribe_core::capture::MonitorPlacement;
use serde_json::json;
use store_markdown::SETTINGS_FILE_NAME;
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_store::StoreExt;

use crate::screenshots::placement::centred_in;

const DISMISSED_KEY: &str = "idle_notice_dismissed";
const WINDOW_LABEL: &str = "idle-notice";
const WINDOW_ROUTE: &str = "index.html#idle-notice";
const WINDOW_TITLE: &str = "Copied to clipboard";
const WINDOW_SIZE: (f64, f64) = (420.0, 220.0);

pub fn is_dismissed<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.store(SETTINGS_FILE_NAME)
        .ok()
        .and_then(|store| store.get(DISMISSED_KEY))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn dismiss_for_good<R: Runtime>(app: &AppHandle<R>) -> tauri_plugin_store::Result<()> {
    let store = app.store(SETTINGS_FILE_NAME)?;
    store.set(DISMISSED_KEY, json!(true));
    store.save()
}

pub fn show<R: Runtime>(app: &AppHandle<R>, placement: &MonitorPlacement) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        return window.set_focus();
    }
    let window = build_hidden(app)?;
    window.set_position(centred_in(placement, WINDOW_SIZE))?;
    window.show()?;
    window.set_focus()
}

fn build_hidden<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<WebviewWindow<R>> {
    WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App(WINDOW_ROUTE.into()))
        .title(WINDOW_TITLE)
        .inner_size(WINDOW_SIZE.0, WINDOW_SIZE.1)
        .resizable(false)
        .always_on_top(true)
        .focused(true)
        .visible(false)
        .build()
}

#[tauri::command]
pub async fn dismiss_idle_notice<R: Runtime>(
    app: AppHandle<R>,
    window: WebviewWindow<R>,
    dont_show_again: bool,
) -> Result<(), String> {
    if dont_show_again {
        dismiss_for_good(&app).map_err(|error| error.to_string())?;
    }
    window.close().map_err(|error| error.to_string())
}
