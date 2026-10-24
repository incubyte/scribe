mod controller;
mod install;
mod menu_item;
mod release_feed;

use std::thread;
use std::time::{Duration, Instant};

use scribe_core::recording::RecorderState;
use scribe_core::update::{
    CHECK_DELAY_AFTER_WAKE, CHECK_INTERVAL, CheckSchedule, InstallConditions, InstallMoment,
    ReleaseVersion, install_moment,
};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_updater::Update;

use crate::call_watch::CallPresence;
use crate::popover;
use crate::recording::{self, RecordingController};

use controller::{StagedPackage, UpdateController};

const THREAD_NAME: &str = "update-check";
const TICK_INTERVAL: Duration = Duration::from_secs(30);

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    if cfg!(debug_assertions) {
        return;
    }
    app.manage(UpdateController::new());
    check_after_wake(app);
    let handle = app.clone();
    let spawned = thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn(move || check_forever(&handle));
    if let Err(error) = spawned {
        tracing::error!(%error, "could not start checking for updates");
    }
}

pub fn install_on_quit<R: Runtime>(app: &AppHandle<R>) {
    install::install_before_exit(app);
}

pub fn restart_to_update<R: Runtime>(app: &AppHandle<R>) {
    if is_free_to_restart(app) {
        install::install_and_relaunch(app, is_free_to_restart);
    }
}

pub fn refresh_restart_item<R: Runtime>(app: &AppHandle<R>) {
    menu_item::enable(app, is_free_to_restart(app));
}

fn is_free_to_restart<R: Runtime>(app: &AppHandle<R>) -> bool {
    !recording::is_recording(app) && !is_in_call(app)
}

fn is_in_call<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.try_state::<CallPresence>()
        .is_none_or(|presence| presence.is_in_call())
}

#[cfg(any(target_os = "macos", windows))]
fn check_after_wake<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    crate::power::on_did_wake(move || app.state::<UpdateController>().note_wake());
}

#[cfg(not(any(target_os = "macos", windows)))]
fn check_after_wake<R: Runtime>(_app: &AppHandle<R>) {}

fn check_forever<R: Runtime>(app: &AppHandle<R>) {
    let mut schedule = CheckSchedule::new(CHECK_INTERVAL);
    loop {
        tick(app, &mut schedule, Instant::now());
        thread::sleep(TICK_INTERVAL);
    }
}

fn tick<R: Runtime>(app: &AppHandle<R>, schedule: &mut CheckSchedule, now: Instant) {
    if app.state::<UpdateController>().take_wake() {
        schedule.check_soon(now + CHECK_DELAY_AFTER_WAKE);
    }
    if schedule.is_check_due(now) {
        schedule.mark_checked(now);
        check_and_stage(app);
    }
    install_if_idle(app, now);
}

fn install_if_idle<R: Runtime>(app: &AppHandle<R>, now: Instant) {
    if is_install_moment(app, now) {
        install::install_and_relaunch(app, is_install_moment_now);
    }
}

fn is_install_moment_now<R: Runtime>(app: &AppHandle<R>) -> bool {
    is_install_moment(app, Instant::now())
}

fn is_install_moment<R: Runtime>(app: &AppHandle<R>, now: Instant) -> bool {
    install_moment(&current_conditions(app, now)) == InstallMoment::Now
}

fn current_conditions<R: Runtime>(app: &AppHandle<R>, now: Instant) -> InstallConditions {
    InstallConditions {
        update_waiting: app
            .try_state::<UpdateController>()
            .is_some_and(|controller| controller.is_update_waiting(now)),
        recorder: app
            .try_state::<RecordingController>()
            .map_or(RecorderState::Locked, |recorder| recorder.state()),
        in_call: is_in_call(app),
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
            menu_item::show(app, is_free_to_restart(app));
        }
    }
}
