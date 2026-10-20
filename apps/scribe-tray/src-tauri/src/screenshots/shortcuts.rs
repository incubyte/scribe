use std::thread;

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{
    Builder, GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState,
};

use crate::recording::RecordingController;
use crate::screenshots::bindings::{DefaultShortcut, SHORTCUT_KINDS, ShortcutKind};
use crate::screenshots::capture::ScreenshotService;
use crate::screenshots::delivery::show_notice;
use crate::screenshots::notices::shortcut_unavailable_notice;

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new().with_handler(on_shortcut).build()
}

pub fn register_defaults<R: Runtime>(app: &AppHandle<R>) {
    for kind in SHORTCUT_KINDS {
        register(app, kind.default_shortcut());
    }
}

fn register<R: Runtime>(app: &AppHandle<R>, shortcut: DefaultShortcut) {
    if let Err(error) = app.global_shortcut().register(shortcut.keys) {
        tracing::warn!(%error, shortcut = shortcut.keys, "could not register a screenshot shortcut");
        show_notice(app, &shortcut_unavailable_notice(shortcut.label));
    }
}

fn on_shortcut<R: Runtime>(app: &AppHandle<R>, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state != ShortcutState::Pressed {
        return;
    }
    if let Some(ShortcutKind::FullScreen) = kind_of(shortcut) {
        take_full_screen_in_background(app);
    }
}

fn kind_of(pressed: &Shortcut) -> Option<ShortcutKind> {
    SHORTCUT_KINDS.into_iter().find(|kind| {
        kind.default_shortcut()
            .keys
            .parse::<Shortcut>()
            .is_ok_and(|shortcut| shortcut == *pressed)
    })
}

fn take_full_screen_in_background<R: Runtime>(app: &AppHandle<R>) {
    let live = app.state::<RecordingController>().live_session();
    let app = app.clone();
    thread::spawn(move || {
        app.state::<ScreenshotService>()
            .take_full_screen(&app, live);
    });
}
