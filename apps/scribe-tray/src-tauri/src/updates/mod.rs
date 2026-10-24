mod controller;
mod release_feed;

use std::thread;
use std::time::{Duration, Instant};

use scribe_core::update::{CHECK_INTERVAL, CheckSchedule, ReleaseVersion};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_updater::Update;

use controller::{StagedPackage, UpdateController};

const THREAD_NAME: &str = "update-check";
const TICK_INTERVAL: Duration = Duration::from_secs(30);

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    if cfg!(debug_assertions) {
        return;
    }
    app.manage(UpdateController::new());
    let handle = app.clone();
    let spawned = thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn(move || check_forever(&handle));
    if let Err(error) = spawned {
        tracing::error!(%error, "could not start checking for updates");
    }
}

fn check_forever<R: Runtime>(app: &AppHandle<R>) {
    let mut schedule = CheckSchedule::new(CHECK_INTERVAL);
    loop {
        tick(app, &mut schedule, Instant::now());
        thread::sleep(TICK_INTERVAL);
    }
}

fn tick<R: Runtime>(app: &AppHandle<R>, schedule: &mut CheckSchedule, now: Instant) {
    if schedule.is_check_due(now) {
        schedule.mark_checked(now);
        check_and_stage(app);
    }
}

fn check_and_stage<R: Runtime>(app: &AppHandle<R>) {
    match release_feed::newer_release(app) {
        Err(error) => tracing::warn!(%error, "could not check for an update"),
        Ok(None) => tracing::debug!("no newer release is available"),
        Ok(Some(update)) => {
            let version = ReleaseVersion::new(update.version.clone());
            download_and_stage(&app.state::<UpdateController>(), update, &version);
        }
    }
}

fn download_and_stage(controller: &UpdateController, update: Update, version: &ReleaseVersion) {
    if !controller.begin_download(version) {
        return;
    }
    match release_feed::download(&update) {
        Err(error) => {
            controller.abandon();
            tracing::error!(%error, version = %version, "could not download the update");
        }
        Ok(bytes) => {
            controller.stage(StagedPackage {
                update,
                bytes: bytes.into(),
            });
            tracing::info!(version = %version, "an update is ready");
        }
    }
}
