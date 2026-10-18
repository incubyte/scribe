use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

pub fn emit<R: Runtime, P: Serialize + Clone>(app: &AppHandle<R>, event: &str, payload: P) {
    if let Err(error) = app.emit(event, payload) {
        tracing::warn!(event, %error, "could not notify the webview");
    }
}
