use std::time::Duration;

use tauri::async_runtime::block_on;
use tauri::{AppHandle, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    #[error(transparent)]
    Updater(#[from] tauri_plugin_updater::Error),
}

pub fn newer_release<R: Runtime>(app: &AppHandle<R>) -> Result<Option<Update>, FeedError> {
    let updater = app.updater_builder().timeout(REQUEST_TIMEOUT).build()?;
    Ok(block_on(updater.check())?)
}

pub fn download(update: &Update) -> Result<Vec<u8>, FeedError> {
    Ok(block_on(update.download(|_, _| {}, || {}))?)
}
