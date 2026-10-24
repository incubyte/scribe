use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use scribe_core::update::{ReleaseVersion, UpdateLifecycle};
use tauri_plugin_updater::Update;

pub struct StagedPackage {
    pub update: Update,
    pub bytes: Arc<[u8]>,
}

pub struct UpdateController {
    lifecycle: Mutex<UpdateLifecycle<StagedPackage>>,
    quitting: AtomicBool,
}

impl UpdateController {
    pub fn new() -> Self {
        Self {
            lifecycle: Mutex::new(UpdateLifecycle::new()),
            quitting: AtomicBool::new(false),
        }
    }

    pub fn begin_quitting(&self) {
        self.quitting.store(true, Ordering::Release);
    }

    pub fn is_quitting(&self) -> bool {
        self.quitting.load(Ordering::Acquire)
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

    pub fn is_update_waiting(&self, now: Instant) -> bool {
        self.lifecycle().is_update_waiting(now)
    }

    pub fn claim_install(&self) -> Option<StagedPackage> {
        self.lifecycle().claim_install()
    }

    pub fn release_failed_install(&self, package: StagedPackage, now: Instant) {
        self.lifecycle().release_failed_install(package, now);
    }

    pub fn finish_install(&self) {
        self.lifecycle().finish_install();
    }

    pub fn is_installed(&self) -> bool {
        self.lifecycle().is_installed()
    }

    pub fn is_installing(&self) -> bool {
        self.lifecycle().is_installing()
    }

    fn lifecycle(&self) -> MutexGuard<'_, UpdateLifecycle<StagedPackage>> {
        self.lifecycle
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}
