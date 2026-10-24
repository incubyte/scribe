use std::time::Duration;

use tauri::async_runtime::block_on;
use tauri::{AppHandle, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5 * 60);

pub fn newer_release<R: Runtime>(
    app: &AppHandle<R>,
) -> tauri_plugin_updater::Result<Option<Update>> {
    let updater = app.updater_builder().timeout(REQUEST_TIMEOUT).build()?;
    block_on(updater.check())
}

pub fn download(update: &Update) -> tauri_plugin_updater::Result<Vec<u8>> {
    block_on(update.download(|_, _| {}, || {}))
}
