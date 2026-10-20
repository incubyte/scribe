use std::sync::{Mutex, MutexGuard};
use std::thread;

use serde_json::json;
use store_markdown::SETTINGS_FILE_NAME;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{
    Builder, GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState,
};
use tauri_plugin_store::StoreExt;

use crate::recording::RecordingController;
use crate::screenshots::bindings::{
    Assignment, Bindings, Platform, RebindOutcome, SHORTCUT_KINDS, ShortcutBinding, ShortcutKind,
};
use crate::screenshots::capture::ScreenshotService;
use crate::screenshots::delivery::show_notice;
use crate::screenshots::notices::shortcut_unavailable_notice;

pub struct ShortcutRegistry {
    bindings: Mutex<Bindings>,
}

impl ShortcutRegistry {
    fn load<R: Runtime>(app: &AppHandle<R>) -> ShortcutRegistry {
        let store = app.store(SETTINGS_FILE_NAME).ok();
        let stored = |kind: ShortcutKind| {
            store
                .as_ref()
                .and_then(|store| store.get(kind.storage_key()))
        };
        ShortcutRegistry {
            bindings: Mutex::new(Bindings::from_stored(
                stored(ShortcutKind::FullScreen).as_ref(),
                stored(ShortcutKind::Region).as_ref(),
                Platform::current(),
            )),
        }
    }

    pub fn views(&self) -> Vec<(ShortcutKind, String)> {
        let bindings = self.lock();
        SHORTCUT_KINDS
            .into_iter()
            .map(|kind| (kind, bindings.get(kind).label(Platform::current())))
            .collect()
    }

    pub fn pause<R: Runtime>(&self, app: &AppHandle<R>) {
        let _held = self.lock();
        unregister_all(app);
    }

    pub fn resume<R: Runtime>(&self, app: &AppHandle<R>) {
        let bindings = self.lock();
        let refused = apply(app, &bindings);
        log_refused(&refused);
    }

    pub fn reset<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        kind: ShortcutKind,
    ) -> tauri_plugin_store::Result<RebindOutcome> {
        self.assign(
            app,
            Assignment {
                kind,
                binding: ShortcutBinding::default_for(kind, Platform::current()),
            },
        )
    }

    pub fn assign<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        assignment: Assignment,
    ) -> tauri_plugin_store::Result<RebindOutcome> {
        let Assignment { kind, binding } = assignment;
        let mut current = self.lock();
        let candidate = match current.replacing(kind, binding) {
            Ok(candidate) => candidate,
            Err(problem) => return Ok(RebindOutcome::refused(problem, Platform::current())),
        };
        if apply(app, &candidate).contains(&kind) {
            apply(app, &current);
            return Ok(RebindOutcome::InUseByAnotherApp);
        }
        if let Err(error) = persist(app, kind, candidate.get(kind)) {
            apply(app, &current);
            return Err(error);
        }
        let label = candidate.get(kind).label(Platform::current());
        *current = candidate;
        Ok(RebindOutcome::Accepted { label })
    }

    fn register_at_launch<R: Runtime>(&self, app: &AppHandle<R>) {
        let bindings = self.lock();
        for kind in apply(app, &bindings) {
            let label = bindings.get(kind).label(Platform::current());
            show_notice(app, &shortcut_unavailable_notice(&label));
        }
    }

    fn kind_of(&self, pressed: &Shortcut) -> Option<ShortcutKind> {
        let bindings = self.lock();
        SHORTCUT_KINDS.into_iter().find(|kind| {
            bindings
                .get(*kind)
                .keys()
                .parse::<Shortcut>()
                .is_ok_and(|shortcut| shortcut == *pressed)
        })
    }

    fn lock(&self) -> MutexGuard<'_, Bindings> {
        self.bindings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new().with_handler(on_shortcut).build()
}

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    app.manage(ShortcutRegistry::load(app));
    app.state::<ShortcutRegistry>().register_at_launch(app);
}

fn apply<R: Runtime>(app: &AppHandle<R>, bindings: &Bindings) -> Vec<ShortcutKind> {
    unregister_all(app);
    SHORTCUT_KINDS
        .into_iter()
        .filter(|kind| {
            let keys = bindings.get(*kind).keys();
            app.global_shortcut().register(keys).is_err()
        })
        .collect()
}

fn unregister_all<R: Runtime>(app: &AppHandle<R>) {
    if let Err(error) = app.global_shortcut().unregister_all() {
        tracing::warn!(%error, "could not release the screenshot shortcuts");
    }
}

fn log_refused(refused: &[ShortcutKind]) {
    for kind in refused {
        tracing::warn!(?kind, "could not register a screenshot shortcut");
    }
}

fn persist<R: Runtime>(
    app: &AppHandle<R>,
    kind: ShortcutKind,
    binding: &ShortcutBinding,
) -> tauri_plugin_store::Result<()> {
    let store = app.store(SETTINGS_FILE_NAME)?;
    store.set(kind.storage_key(), json!(binding.keys()));
    store.save()
}

fn on_shortcut<R: Runtime>(app: &AppHandle<R>, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state != ShortcutState::Pressed {
        return;
    }
    if let Some(kind) = app.state::<ShortcutRegistry>().kind_of(shortcut) {
        capture_in_background(app, kind);
    }
}

fn capture_in_background<R: Runtime>(app: &AppHandle<R>, kind: ShortcutKind) {
    let live = app.state::<RecordingController>().live_session();
    let app = app.clone();
    thread::spawn(move || {
        let service = app.state::<ScreenshotService>();
        match kind {
            ShortcutKind::FullScreen => service.take_full_screen(&app, live),
            ShortcutKind::Region => service.begin_region(&app, live),
        }
    });
}
