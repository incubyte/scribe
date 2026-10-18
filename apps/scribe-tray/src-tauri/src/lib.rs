pub mod blur_guard;
mod commands;
mod lanes;
pub mod live_feed;
mod mcp_clipboard;
pub mod mcp_config;
pub mod menu_action;
mod model_setup;
mod notifications;
mod popover;
mod power;
mod privacy;
mod recording;
pub mod recording_label;
pub mod start_failure;
mod tray;
mod tray_listener;
mod webview;

use tauri::{AppHandle, Builder, Runtime};

pub fn run() -> tauri::Result<()> {
    with_plugins(Builder::default())
        .invoke_handler(tauri::generate_handler![
            privacy::acknowledge_privacy,
            commands::recorder_snapshot,
            commands::recent_sessions,
            commands::start_recording,
            commands::stop_recording,
            commands::open_session,
            commands::model_download_status,
            commands::retry_model_download,
            popover::hide_popover,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            start_scribe(app.handle())
        })
        .run(tauri::generate_context!())
}

fn with_plugins<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_positioner::init())
}

fn start_scribe<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    tray::install(app)?;
    recording::install(app)?;
    recording::publish_state(app);
    popover::install_blur_hide(app)?;
    privacy::show_statement_until_acknowledged(app)?;
    Ok(())
}
