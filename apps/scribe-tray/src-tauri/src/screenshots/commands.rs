use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};

use crate::screenshots::bindings::{PressedKeys, RebindOutcome, ShortcutKind};
use crate::screenshots::shortcuts::ShortcutRegistry;

#[derive(Debug, Serialize)]
pub struct ShortcutView {
    kind: ShortcutKind,
    label: String,
}

#[tauri::command]
pub async fn screenshot_shortcuts<R: Runtime>(app: AppHandle<R>) -> Vec<ShortcutView> {
    app.state::<ShortcutRegistry>()
        .views()
        .into_iter()
        .map(|(kind, label)| ShortcutView { kind, label })
        .collect()
}

#[tauri::command]
pub async fn pause_screenshot_shortcuts<R: Runtime>(app: AppHandle<R>) {
    app.state::<ShortcutRegistry>().pause(&app);
}

#[tauri::command]
pub async fn resume_screenshot_shortcuts<R: Runtime>(app: AppHandle<R>) {
    app.state::<ShortcutRegistry>().resume(&app);
}

#[tauri::command]
pub async fn rebind_screenshot_shortcut<R: Runtime>(
    app: AppHandle<R>,
    kind: ShortcutKind,
    pressed: PressedKeys,
) -> Result<RebindOutcome, String> {
    app.state::<ShortcutRegistry>()
        .rebind(&app, kind, &pressed)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn reset_screenshot_shortcut<R: Runtime>(
    app: AppHandle<R>,
    kind: ShortcutKind,
) -> Result<RebindOutcome, String> {
    app.state::<ShortcutRegistry>()
        .reset(&app, kind)
        .map_err(|error| error.to_string())
}
