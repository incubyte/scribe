mod controller;
mod install;
mod release_feed;

use std::thread;
use std::time::{Duration, Instant};

use scribe_core::recording::RecorderState;
use scribe_core::update::{
    CHECK_INTERVAL, CheckSchedule, InstallConditions, InstallMoment, ReleaseVersion, install_moment,
};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_updater::Update;

use crate::call_watch::CallPresence;
use crate::popover;
use crate::recording::RecordingController;
use crate::tray;

pub use install::AfterInstall;
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

pub fn install_on_quit<R: Runtime>(app: &AppHandle<R>) {
    install::install_staged(app, AfterInstall::Exit);
}

pub fn restart_to_update<R: Runtime>(app: &AppHandle<R>) {
    let recording = app
        .try_state::<RecordingController>()
        .is_some_and(|recorder| recorder.state() == RecorderState::Recording);
    if !recording {
        install::install_staged(app, AfterInstall::Relaunch);
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
    install_if_idle(app, now);
}

fn install_if_idle<R: Runtime>(app: &AppHandle<R>, now: Instant) {
    if install_moment(&current_conditions(app, now)) == InstallMoment::Now {
        install::install_staged(app, AfterInstall::Relaunch);
    }
}

fn current_conditions<R: Runtime>(app: &AppHandle<R>, now: Instant) -> InstallConditions {
    InstallConditions {
        install_due: app
            .try_state::<UpdateController>()
            .is_some_and(|controller| controller.is_install_due(now)),
        recorder: app
            .try_state::<RecordingController>()
            .map_or(RecorderState::Locked, |recorder| recorder.state()),
        in_call: app
            .try_state::<CallPresence>()
            .is_none_or(|presence| presence.is_in_call()),
        popover_open: popover::is_open(app),
    }
}

fn check_and_stage<R: Runtime>(app: &AppHandle<R>) {
    match release_feed::newer_release(app) {
        Err(error) => tracing::warn!(%error, "could not check for an update"),
        Ok(None) => tracing::debug!("no newer release is available"),
        Ok(Some(update)) => {
            let version = ReleaseVersion::new(update.version.clone());
            download_and_stage(app, update, &version);
        }
    }
}

fn download_and_stage<R: Runtime>(app: &AppHandle<R>, update: Update, version: &ReleaseVersion) {
    let controller = app.state::<UpdateController>();
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
            tray::show_restart_to_update(app);
        }
    }
}
