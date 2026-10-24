use std::time::Instant;

use tauri::{AppHandle, Manager, Runtime};

use super::controller::{StagedPackage, UpdateController};

#[derive(Debug, Clone, Copy)]
enum AfterInstall {
    Exit,
    Relaunch,
}

pub fn install_before_exit<R: Runtime>(app: &AppHandle<R>) {
    if let Some(controller) = app.try_state::<UpdateController>() {
        install_claimed(&controller, AfterInstall::Exit);
    }
}

pub fn install_and_relaunch<R: Runtime>(
    app: &AppHandle<R>,
    may_relaunch: fn(&AppHandle<R>) -> bool,
) {
    let Some(controller) = app.try_state::<UpdateController>() else {
        return;
    };
    let installed =
        controller.is_installed() || install_claimed(&controller, AfterInstall::Relaunch);
    if !installed {
        return;
    }
    if may_relaunch(app) {
        app.restart();
    }
    tracing::info!("the update is installed and relaunches once Scribe is idle");
}

fn install_claimed(controller: &UpdateController, after: AfterInstall) -> bool {
    let Some(package) = controller.claim_install() else {
        return false;
    };
    let version = package.update.version.clone();
    tracing::info!(version, "installing the update");
    match apply(&package, after) {
        Ok(()) => {
            controller.finish_install();
            true
        }
        Err(error) => {
            tracing::error!(%error, version, "could not install the update");
            controller.release_failed_install(package, Instant::now());
            false
        }
    }
}

fn apply(package: &StagedPackage, after: AfterInstall) -> tauri_plugin_updater::Result<()> {
    package
        .update
        .clone()
        .restart_after_install(matches!(after, AfterInstall::Relaunch))
        .install(&package.bytes)
}
