pub mod blur_guard;
pub mod claude_desktop;
mod claude_desktop_connect;
mod commands;
mod lanes;
pub mod live_feed;
mod mcp_clipboard;
pub mod mcp_config;
pub mod menu_action;
mod model_setup;
mod notifications;
pub mod other_side_alert;
mod popover;
mod power;
mod privacy;
pub mod recording;
pub mod recording_label;
pub mod screenshots;
pub mod start_failure;
pub mod transcripts_location;
mod tray;
mod tray_listener;
mod webview;

use tauri::{AppHandle, Builder, Runtime};

pub fn run() -> tauri::Result<()> {
    with_commands(with_plugins(Builder::default()))
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            start_scribe(app.handle())
        })
        .run(tauri::generate_context!())
}

fn with_commands<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        privacy::acknowledge_privacy,
        commands::recorder_snapshot,
        commands::recent_sessions,
        commands::start_recording,
        commands::stop_recording,
        commands::open_session,
        commands::model_download_status,
        commands::retry_model_download,
        commands::connect_claude_desktop,
        commands::claude_desktop_status,
        commands::copy_mcp_config,
        commands::open_transcripts_folder,
        commands::change_transcripts_folder,
        commands::transcripts_folder,
        popover::hide_popover,
    ])
}

fn with_plugins<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(screenshots::plugin())
}

fn start_scribe<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    tray::install(app)?;
    recording::install(app)?;
    screenshots::install(app);
    recording::publish_state(app);
    popover::install_blur_hide(app)?;
    privacy::show_statement_until_acknowledged(app)?;
    Ok(())
}
