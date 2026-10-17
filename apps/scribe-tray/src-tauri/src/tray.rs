use store_markdown::{StoreError, TranscriptsFolder};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Runtime};
use tauri_plugin_opener::OpenerExt;

use crate::menu_action::{MENU_OPEN_FOLDER, MENU_PRIVACY, MENU_QUIT, MenuAction};
use crate::privacy;

const TRAY_ID: &str = "main";
const TOOLTIP: &str = "Scribe";

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<TrayIcon<R>> {
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .icon_as_template(true)
        .tooltip(TOOLTIP)
        .show_menu_on_left_click(true)
        .menu(&build_menu(app)?)
        .on_menu_event(|app, event| dispatch(app, event.id.as_ref()));
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)
}

fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let open_folder = MenuItem::with_id(
        app,
        MENU_OPEN_FOLDER,
        "Open transcripts folder",
        true,
        None::<&str>,
    )?;
    let privacy = MenuItem::with_id(app, MENU_PRIVACY, "Privacy", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Scribe", true, None::<&str>)?;
    Menu::with_items(app, &[&open_folder, &privacy, &quit])
}

fn dispatch<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match MenuAction::from_id(id) {
        Some(MenuAction::OpenTranscriptsFolder) => open_transcripts_folder(app),
        Some(MenuAction::ShowPrivacy) => show_privacy(app),
        Some(MenuAction::Quit) => app.exit(0),
        None => {}
    }
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
