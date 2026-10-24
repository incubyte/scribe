use std::sync::Arc;
use std::time::Duration;

use scribe_core::recording::RecorderState;
use store_markdown::{MarkdownTranscriptStore, StoreError};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_opener::OpenerExt;

use crate::claude_desktop_connect;
use crate::mcp_clipboard;
use crate::menu_action::{
    MENU_CONNECT_CLAUDE_DESKTOP, MENU_COPY_MCP_CONFIG, MENU_OPEN_FOLDER, MENU_PRIVACY, MENU_QUIT,
    MENU_TOGGLE_RECORDING, MenuAction,
};
use crate::popover;
use crate::privacy;
use crate::recording::{self, RecordingController};
use crate::recording_label::{START_RECORDING, stop_recording_label};
use crate::tray_anchor::ScreenArea;
use crate::updates;

const TRAY_ID: &str = "main";
const IDLE_TOOLTIP: &str = "Scribe";
const RECORDING_TOOLTIP: &str = "Scribe — recording";
const IDLE_ICON: Image<'static> = tauri::include_image!("icons/tray-idle.png");
const RECORDING_ICON: Image<'static> = tauri::include_image!("icons/tray-recording.png");

struct RecordingMenuItem<R: Runtime>(MenuItem<R>);

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<TrayIcon<R>> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(IDLE_ICON)
        .icon_as_template(true)
        .tooltip(IDLE_TOOLTIP)
        .show_menu_on_left_click(false)
        .menu(&build_menu(app)?)
        .on_menu_event(|app, event| dispatch(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| on_tray_icon_event(tray.app_handle(), &event))
        .build(app)
}

pub fn icon_area<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Option<ScreenArea>> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(None);
    };
    Ok(tray.rect()?.map(|rect| ScreenArea {
        position: rect.position.to_physical(1.0),
        size: rect.size.to_physical(1.0),
    }))
}

fn on_tray_icon_event<R: Runtime>(app: &AppHandle<R>, event: &TrayIconEvent) {
    tauri_plugin_positioner::on_tray_event(app, event);
    if is_left_click_release(event) {
        popover::toggle(app);
    }
}

fn is_left_click_release(event: &TrayIconEvent) -> bool {
    matches!(
        event,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        }
    )
}

fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let toggle = recording_toggle_item(app)?;
    let open_folder = enabled_item(app, MENU_OPEN_FOLDER, "Open transcripts folder")?;
    let copy_mcp_config = enabled_item(app, MENU_COPY_MCP_CONFIG, "Copy MCP config")?;
    let connect_claude_desktop = connect_claude_desktop_item(app)?;
    let privacy = enabled_item(app, MENU_PRIVACY, "Privacy")?;
    let quit = enabled_item(app, MENU_QUIT, "Quit Scribe")?;
    let menu = Menu::with_items(
        app,
        &[
            &toggle,
            &PredefinedMenuItem::separator(app)?,
            &open_folder,
            &copy_mcp_config,
            &connect_claude_desktop,
            &privacy,
            &quit,
        ],
    )?;
    app.manage(RecordingMenuItem(toggle));
    Ok(menu)
}

fn connect_claude_desktop_item<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<MenuItem<R>> {
    enabled_item(
        app,
        MENU_CONNECT_CLAUDE_DESKTOP,
        "Connect to Claude Desktop",
    )
}

fn recording_toggle_item<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<MenuItem<R>> {
    MenuItem::with_id(
        app,
        MENU_TOGGLE_RECORDING,
        START_RECORDING,
        false,
        None::<&str>,
    )
}

fn enabled_item<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
    label: &str,
) -> tauri::Result<MenuItem<R>> {
    MenuItem::with_id(app, id, label, true, None::<&str>)
}

struct TrayLook {
    icon: Image<'static>,
    tooltip: &'static str,
}

const IDLE_LOOK: TrayLook = TrayLook {
    icon: IDLE_ICON,
    tooltip: IDLE_TOOLTIP,
};
const RECORDING_LOOK: TrayLook = TrayLook {
    icon: RECORDING_ICON,
    tooltip: RECORDING_TOOLTIP,
};

pub fn show_recorder_state<R: Runtime>(app: &AppHandle<R>, state: RecorderState) {
    match state {
        RecorderState::Locked => {
            disable_toggle(app, START_RECORDING);
            show_look(app, &IDLE_LOOK);
        }
        RecorderState::Idle => {
            enable_toggle(app, START_RECORDING);
            show_look(app, &IDLE_LOOK);
        }
        RecorderState::Recording => {
            enable_toggle(app, &stop_recording_label(Duration::ZERO));
            show_look(app, &RECORDING_LOOK);
        }
    }
}

pub fn show_elapsed<R: Runtime>(app: &AppHandle<R>, elapsed: Duration) {
    enable_toggle(app, &stop_recording_label(elapsed));
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

fn enable_toggle<R: Runtime>(app: &AppHandle<R>, label: &str) {
    let toggle = relabel_toggle(app, label);
    log_failure(
        toggle.set_enabled(true),
        "could not update the recording menu item",
    );
}

fn disable_toggle<R: Runtime>(app: &AppHandle<R>, label: &str) {
    let toggle = relabel_toggle(app, label);
    log_failure(
        toggle.set_enabled(false),
        "could not update the recording menu item",
    );
}

fn relabel_toggle<'a, R: Runtime>(app: &'a AppHandle<R>, label: &str) -> &'a MenuItem<R> {
    let toggle = &app.state::<RecordingMenuItem<R>>().inner().0;
    log_failure(
        toggle.set_text(label),
        "could not update the recording menu item",
    );
    toggle
}

fn show_look<R: Runtime>(app: &AppHandle<R>, look: &TrayLook) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    log_failure(
        tray.set_icon(Some(look.icon.clone())),
        "could not update the tray icon",
    );
    log_failure(
        tray.set_icon_as_template(true),
        "could not update the tray icon",
    );
    log_failure(
        tray.set_tooltip(Some(look.tooltip)),
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
        Some(MenuAction::CopyMcpConfig) => mcp_clipboard::copy_config(app),
        Some(MenuAction::ConnectClaudeDesktop) => {
            claude_desktop_connect::connect_claude_desktop(app);
        }
        Some(MenuAction::ShowPrivacy) => show_privacy(app),
        Some(MenuAction::Quit) => quit(app),
        None => {}
    }
}

fn toggle_recording<R: Runtime>(app: &AppHandle<R>) {
    let controller = app.state::<RecordingController>();
    match controller.state() {
        RecorderState::Recording => controller.stop(),
        RecorderState::Idle | RecorderState::Locked => recording::start_in_background(app.clone()),
    }
}

fn quit<R: Runtime>(app: &AppHandle<R>) {
    app.state::<RecordingController>().stop();
    updates::install_on_quit(app);
    app.exit(0);
}

fn show_privacy<R: Runtime>(app: &AppHandle<R>) {
    if let Err(error) = privacy::show_statement(app) {
        tracing::error!(%error, "could not show the privacy statement");
    }
}

pub(crate) fn open_transcripts_folder<R: Runtime>(app: &AppHandle<R>) {
    if let Err(error) = try_open_transcripts_folder(app) {
        tracing::error!(%error, "could not open the transcripts folder");
    }
}

fn try_open_transcripts_folder<R: Runtime>(app: &AppHandle<R>) -> Result<(), OpenFolderError> {
    let folder = app.state::<Arc<MarkdownTranscriptStore>>().folder();
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
