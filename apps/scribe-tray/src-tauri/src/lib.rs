mod lanes;
pub mod menu_action;
mod notifications;
mod power;
mod privacy;
mod recording;
pub mod recording_label;
mod tray;
mod tray_listener;

use tauri::{AppHandle, Builder, Manager, Runtime};

pub fn run() -> tauri::Result<()> {
    Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![privacy::acknowledge_privacy])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            start_scribe(app.handle())
        })
        .run(tauri::generate_context!())
}

fn start_scribe<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    tray::install(app)?;
    recording::install(app)?;
    tray::show_recorder_state(app, app.state::<recording::RecordingController>().state());
    privacy::show_statement_until_acknowledged(app)?;
    Ok(())
}
