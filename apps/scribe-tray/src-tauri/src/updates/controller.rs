use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use scribe_core::update::{ReleaseVersion, UpdateLifecycle};
use tauri_plugin_updater::Update;

#[derive(Clone)]
pub struct StagedPackage {
    pub update: Update,
    pub bytes: Arc<[u8]>,
}

pub struct UpdateController {
    lifecycle: Mutex<UpdateLifecycle<StagedPackage>>,
}

impl UpdateController {
    pub fn new() -> Self {
        Self {
            lifecycle: Mutex::new(UpdateLifecycle::new()),
        }
    }

    pub fn begin_download(&self, version: &ReleaseVersion) -> bool {
        self.lifecycle().begin_download(version)
    }

    pub fn stage(&self, package: StagedPackage) {
        self.lifecycle().finish_download(package);
    }

    pub fn abandon(&self) {
        self.lifecycle().abandon_download();
    }

    pub fn package_to_install(&self) -> Option<StagedPackage> {
        self.lifecycle().package_to_install()
    }

    fn lifecycle(&self) -> MutexGuard<'_, UpdateLifecycle<StagedPackage>> {
        self.lifecycle
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}
