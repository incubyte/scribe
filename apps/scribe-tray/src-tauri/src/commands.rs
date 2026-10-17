use std::sync::Arc;

use scribe_core::ports::TranscriptStore;
use scribe_core::recording::{SessionId, SessionSummary};
use serde::Serialize;
use store_markdown::MarkdownTranscriptStore;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_opener::OpenerExt;

use crate::live_feed::{LiveFeed, RecorderSnapshot};
use crate::recording::{self, RecordingController};

#[derive(Serialize)]
pub struct SessionSummaryView {
    id: String,
    title: String,
    started_at: String,
    duration_seconds: Option<u64>,
    status: &'static str,
}

impl From<SessionSummary> for SessionSummaryView {
    fn from(summary: SessionSummary) -> SessionSummaryView {
        SessionSummaryView {
            id: summary.id.to_string(),
            title: summary.title,
            started_at: summary.started_at.to_rfc3339(),
            duration_seconds: summary.duration_seconds,
            status: summary.status.as_str(),
        }
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn recorder_snapshot<R: Runtime>(app: AppHandle<R>) -> RecorderSnapshot {
    let state = app.state::<RecordingController>().state();
    app.state::<LiveFeed>().snapshot(state)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn recent_sessions<R: Runtime>(
    app: AppHandle<R>,
    limit: u8,
) -> Result<Vec<SessionSummaryView>, String> {
    let store = app.state::<Arc<MarkdownTranscriptStore>>();
    let summaries = store
        .list(usize::from(limit))
        .map_err(|error| error.to_string())?;
    Ok(summaries
        .into_iter()
        .map(SessionSummaryView::from)
        .collect())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn start_recording<R: Runtime>(app: AppHandle<R>) {
    recording::start_in_background(app);
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn stop_recording<R: Runtime>(app: AppHandle<R>) {
    app.state::<RecordingController>().stop();
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn open_session<R: Runtime>(app: AppHandle<R>, id: String) -> Result<(), String> {
    let id = SessionId::parse(&id).map_err(|error| error.to_string())?;
    let path = app.state::<Arc<MarkdownTranscriptStore>>().path_for(&id);
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|error| error.to_string())
}
