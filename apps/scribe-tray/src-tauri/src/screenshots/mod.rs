pub mod bindings;
mod capture;
pub mod capture_gate;
mod capturers;
pub(crate) mod commands;
mod delivery;
pub mod idle_notice;
pub mod notices;
pub mod placement;
pub(crate) mod region_selector;
mod shortcuts;
mod window_level;

use std::sync::Arc;

use scribe_core::capture::ScreenshotTaker;
use store_markdown::MarkdownTranscriptStore;
use tauri::{AppHandle, Manager, Runtime};

use capture::ScreenshotService;

pub use shortcuts::plugin;

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    let store = Arc::clone(app.state::<Arc<MarkdownTranscriptStore>>().inner());
    let taker = ScreenshotTaker::new(capturers::screen_capturer(), store);
    app.manage(ScreenshotService::new(taker));
    shortcuts::install(app);
}
