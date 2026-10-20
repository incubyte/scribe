pub mod bindings;
mod capture;
mod capturers;
mod delivery;
pub mod notices;
mod shortcuts;

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
    shortcuts::register_defaults(app);
}
