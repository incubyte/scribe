use std::time::Instant;

use tauri::{AppHandle, Manager, Runtime};

use super::controller::{StagedPackage, UpdateController};

#[derive(Debug, Clone, Copy)]
pub enum AfterInstall {
    Exit,
    Relaunch,
}

pub fn install_staged<R: Runtime>(app: &AppHandle<R>, after: AfterInstall) {
    let Some(controller) = app.try_state::<UpdateController>() else {
        return;
    };
    let Some(package) = controller.package_to_install() else {
        return;
    };
    let version = package.update.version.clone();
    tracing::info!(version, "installing the update");
    match apply(package, after) {
        Ok(()) => finish(app, after),
        Err(error) => {
            controller.record_failed_install(Instant::now());
            tracing::error!(%error, version, "could not install the update");
        }
    }
}

fn apply(package: StagedPackage, after: AfterInstall) -> Result<(), tauri_plugin_updater::Error> {
    package
        .update
        .restart_after_install(matches!(after, AfterInstall::Relaunch))
        .install(&package.bytes)
}

fn finish<R: Runtime>(app: &AppHandle<R>, after: AfterInstall) {
    match after {
        AfterInstall::Exit => app.exit(0),
        AfterInstall::Relaunch => app.restart(),
    }
}
