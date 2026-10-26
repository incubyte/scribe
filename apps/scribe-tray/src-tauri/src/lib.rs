pub mod blur_guard;
pub mod calendar;
mod call_probes;
mod call_watch;
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
pub mod tray_anchor;
mod tray_listener;
mod updates;
mod webview;

use std::fmt::Display;

use tauri::{AppHandle, Builder, Runtime};

pub fn run() -> tauri::Result<()> {
    with_commands(with_plugins(Builder::default()))
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            start_scribe(app.handle());
            Ok(())
        })
        .run(tauri::generate_context!())
}

#[allow(clippy::too_many_lines)]
fn with_commands<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        privacy::acknowledge_privacy,
        screenshots::idle_notice::dismiss_idle_notice,
        screenshots::region_selector::region_frame,
        screenshots::region_selector::region_context,
        screenshots::region_selector::complete_region,
        screenshots::region_selector::cancel_region,
        screenshots::commands::screenshot_shortcuts,
        screenshots::commands::pause_screenshot_shortcuts,
        screenshots::commands::resume_screenshot_shortcuts,
        screenshots::commands::rebind_screenshot_shortcut,
        screenshots::commands::reset_screenshot_shortcut,
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
        calendar::commands::outlook_connection,
        calendar::commands::connect_outlook,
        calendar::commands::disconnect_outlook,
        calendar::commands::todays_agenda,
        calendar::commands::refresh_agenda,
        calendar::commands::call_meeting,
        calendar::commands::choose_meeting,
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(screenshots::plugin())
}

fn start_scribe<R: Runtime>(app: &AppHandle<R>) {
    updates::install(app);
    log_startup_failure(tray::install(app).map(drop), "could not show the tray icon");
    start_recorder(app);
    log_startup_failure(
        popover::install_blur_hide(app),
        "could not hide the popover when it loses focus",
    );
    call_watch::install(app);
    calendar::install(app);
    log_startup_failure(
        privacy::show_statement_until_acknowledged(app),
        "could not show the privacy statement",
    );
}

fn start_recorder<R: Runtime>(app: &AppHandle<R>) {
    match recording::install(app) {
        Ok(()) => {
            screenshots::install(app);
            recording::publish_state(app);
        }
        Err(error) => tracing::error!(%error, "could not start the recorder"),
    }
}

fn log_startup_failure<E: Display>(outcome: Result<(), E>, message: &str) {
    if let Err(error) = outcome {
        tracing::error!(%error, "{message}");
    }
}
