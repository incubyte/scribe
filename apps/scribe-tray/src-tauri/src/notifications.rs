use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

const MICROPHONE_DENIED_TITLE: &str = "Scribe can't use the microphone";
const MICROPHONE_DENIED_BODY: &str = "Allow Scribe in System Settings > Privacy & Security > Microphone, then start recording again.";

pub fn notify_microphone_denied<R: Runtime>(app: &AppHandle<R>) {
    let shown = app
        .notification()
        .builder()
        .title(MICROPHONE_DENIED_TITLE)
        .body(MICROPHONE_DENIED_BODY)
        .show();
    if let Err(error) = shown {
        tracing::warn!(%error, "could not show the microphone permission notification");
    }
}
