use std::thread;
use std::time::Duration;

use scribe_core::recording::RecorderState;
use store_markdown::{StoreError, TranscriptsFolder};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_opener::OpenerExt;

use crate::menu_action::{
    MENU_OPEN_FOLDER, MENU_PRIVACY, MENU_QUIT, MENU_TOGGLE_RECORDING, MenuAction,
};
use crate::notifications;
use crate::privacy;
use crate::recording::{RecordingController, StartError};
use crate::recording_label::{START_RECORDING, stop_recording_label};

const TRAY_ID: &str = "main";
const IDLE_TOOLTIP: &str = "Scribe";
const RECORDING_TOOLTIP: &str = "Scribe — recording";
const IDLE_ICON: Image<'static> = tauri::include_image!("icons/icon.png");
const RECORDING_ICON: Image<'static> = tauri::include_image!("icons/tray-recording.png");

struct RecordingMenuItem<R: Runtime>(MenuItem<R>);

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<TrayIcon<R>> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(IDLE_ICON)
        .icon_as_template(true)
        .tooltip(IDLE_TOOLTIP)
        .show_menu_on_left_click(true)
        .menu(&build_menu(app)?)
        .on_menu_event(|app, event| dispatch(app, event.id.as_ref()))
        .build(app)
}

fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let toggle = MenuItem::with_id(
        app,
        MENU_TOGGLE_RECORDING,
        START_RECORDING,
        false,
        None::<&str>,
    )?;
    let open_folder = MenuItem::with_id(
        app,
        MENU_OPEN_FOLDER,
        "Open transcripts folder",
        true,
        None::<&str>,
    )?;
    let privacy = MenuItem::with_id(app, MENU_PRIVACY, "Privacy", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Scribe", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&toggle, &separator, &open_folder, &privacy, &quit])?;
    app.manage(RecordingMenuItem(toggle));
    Ok(menu)
}

pub fn show_recorder_state<R: Runtime>(app: &AppHandle<R>, state: RecorderState) {
    let recording = state == RecorderState::Recording;
    let label = if recording {
        stop_recording_label(Duration::ZERO)
    } else {
        START_RECORDING.to_owned()
    };
    update_toggle(app, &label, state != RecorderState::Locked);
    update_icon(app, recording);
}

pub fn show_elapsed<R: Runtime>(app: &AppHandle<R>, elapsed: Duration) {
    update_toggle(app, &stop_recording_label(elapsed), true);
}

pub fn show_tooltip<R: Runtime>(app: &AppHandle<R>, tooltip: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        log_failure(
            tray.set_tooltip(Some(tooltip)),
            "could not update the tray tooltip",
        );
    }
}

pub fn show_idle_tooltip<R: Runtime>(app: &AppHandle<R>) {
    show_tooltip(app, IDLE_TOOLTIP);
}

fn update_toggle<R: Runtime>(app: &AppHandle<R>, label: &str, enabled: bool) {
    let toggle = &app.state::<RecordingMenuItem<R>>().0;
    log_failure(
        toggle.set_text(label),
        "could not update the recording menu item",
    );
    log_failure(
        toggle.set_enabled(enabled),
        "could not update the recording menu item",
    );
}

fn update_icon<R: Runtime>(app: &AppHandle<R>, recording: bool) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let (icon, tooltip) = if recording {
        (RECORDING_ICON, RECORDING_TOOLTIP)
    } else {
        (IDLE_ICON, IDLE_TOOLTIP)
    };
    log_failure(tray.set_icon(Some(icon)), "could not update the tray icon");
    log_failure(
        tray.set_icon_as_template(true),
        "could not update the tray icon",
    );
    log_failure(
        tray.set_tooltip(Some(tooltip)),
        "could not update the tray tooltip",
    );
}

fn log_failure(outcome: tauri::Result<()>, message: &str) {
    if let Err(error) = outcome {
        tracing::warn!(%error, "{message}");
    }
}

fn dispatch<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match MenuAction::from_id(id) {
        Some(MenuAction::ToggleRecording) => toggle_recording(app),
        Some(MenuAction::OpenTranscriptsFolder) => open_transcripts_folder(app),
        Some(MenuAction::ShowPrivacy) => show_privacy(app),
        Some(MenuAction::Quit) => quit(app),
        None => {}
    }
}

fn toggle_recording<R: Runtime>(app: &AppHandle<R>) {
    let controller = app.state::<RecordingController>();
    match controller.state() {
        RecorderState::Recording => controller.stop(),
        RecorderState::Idle | RecorderState::Locked => start_in_background(app.clone()),
    }
}

fn start_in_background<R: Runtime>(app: AppHandle<R>) {
    thread::spawn(move || {
        if let Err(error) = app.state::<RecordingController>().start() {
            report_start_failure(&app, &error);
        }
    });
}

fn report_start_failure<R: Runtime>(app: &AppHandle<R>, error: &StartError) {
    tracing::warn!(%error, "could not start recording");
    if error.is_microphone_denied() {
        notifications::notify_microphone_denied(app);
    }
}

fn quit<R: Runtime>(app: &AppHandle<R>) {
    app.state::<RecordingController>().stop();
    app.exit(0);
}

fn show_privacy<R: Runtime>(app: &AppHandle<R>) {
    if let Err(error) = privacy::show_statement(app) {
        tracing::error!(%error, "could not show the privacy statement");
    }
}

fn open_transcripts_folder<R: Runtime>(app: &AppHandle<R>) {
    if let Err(error) = try_open_transcripts_folder(app) {
        tracing::error!(%error, "could not open the transcripts folder");
    }
}

fn try_open_transcripts_folder<R: Runtime>(app: &AppHandle<R>) -> Result<(), OpenFolderError> {
    let folder = TranscriptsFolder::default_location()?;
    let path = folder.ensure_exists()?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum OpenFolderError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Opener(#[from] tauri_plugin_opener::Error),
}
