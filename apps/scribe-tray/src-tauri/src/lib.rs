pub mod blur_guard;
mod commands;
mod lanes;
pub mod live_feed;
pub mod menu_action;
mod notifications;
mod popover;
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
        .plugin(tauri_plugin_positioner::init())
        .invoke_handler(tauri::generate_handler![
            privacy::acknowledge_privacy,
            commands::recorder_snapshot,
            commands::recent_sessions,
            commands::start_recording,
            commands::stop_recording,
            commands::open_session,
            popover::hide_popover,
        ])
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
    popover::install_blur_hide(app)?;
    privacy::show_statement_until_acknowledged(app)?;
    Ok(())
}
