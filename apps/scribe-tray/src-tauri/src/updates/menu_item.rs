use tauri::menu::{Menu, MenuItem, MenuItemKind};
use tauri::{AppHandle, Runtime};

use crate::menu_action::{MENU_QUIT, MENU_RESTART_TO_UPDATE};
use crate::tray;

const RESTART_TO_UPDATE_LABEL: &str = "Restart to update";

pub fn show<R: Runtime>(app: &AppHandle<R>, enabled: bool) {
    let Some(menu) = tray::menu(app) else {
        return;
    };
    if menu.get(MENU_RESTART_TO_UPDATE).is_none() {
        log_failure(
            insert_above_quit(app, &menu, enabled),
            "could not add the restart to update menu item",
        );
    }
}

pub fn enable<R: Runtime>(app: &AppHandle<R>, enabled: bool) {
    let item = tray::menu(app).and_then(|menu| menu.get(MENU_RESTART_TO_UPDATE));
    if let Some(MenuItemKind::MenuItem(item)) = item {
        log_failure(
            item.set_enabled(enabled),
            "could not update the restart to update menu item",
        );
    }
}

fn insert_above_quit<R: Runtime>(
    app: &AppHandle<R>,
    menu: &Menu<R>,
    enabled: bool,
) -> tauri::Result<()> {
    let item = MenuItem::with_id(
        app,
        MENU_RESTART_TO_UPDATE,
        RESTART_TO_UPDATE_LABEL,
        enabled,
        None::<&str>,
    )?;
    let quit_position = menu
        .items()?
        .iter()
        .position(|existing| existing.id() == MENU_QUIT)
        .unwrap_or(0);
    menu.insert(&item, quit_position)
}

fn log_failure(outcome: tauri::Result<()>, message: &str) {
    if let Err(error) = outcome {
        tracing::warn!(%error, "{message}");
    }
}
