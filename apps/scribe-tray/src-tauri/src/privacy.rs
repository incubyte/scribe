use serde_json::json;
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_store::StoreExt;

const SETTINGS_FILE: &str = "settings.json";
const ACKNOWLEDGED_KEY: &str = "privacy_acknowledged";
const WINDOW_LABEL: &str = "privacy";
const WINDOW_ROUTE: &str = "index.html#privacy";
const WINDOW_TITLE: &str = "Privacy";
const WINDOW_SIZE: (f64, f64) = (420.0, 300.0);

pub fn is_acknowledged<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.store(SETTINGS_FILE)
        .ok()
        .and_then(|store| store.get(ACKNOWLEDGED_KEY))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

pub fn acknowledge<R: Runtime>(app: &AppHandle<R>) -> tauri_plugin_store::Result<()> {
    let store = app.store(SETTINGS_FILE)?;
    store.set(ACKNOWLEDGED_KEY, json!(true));
    store.save()
}

pub fn show_statement<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        return window.set_focus();
    }
    WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App(WINDOW_ROUTE.into()))
        .title(WINDOW_TITLE)
        .inner_size(WINDOW_SIZE.0, WINDOW_SIZE.1)
        .center()
        .resizable(false)
        .build()
        .map(|_| ())
}

pub fn show_statement_until_acknowledged<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if is_acknowledged(app) {
        return Ok(());
    }
    show_statement(app)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn acknowledge_privacy<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    acknowledge(&app).map_err(|error| error.to_string())?;
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.close().map_err(|error| error.to_string())?;
    }
    Ok(())
}
