use std::sync::Arc;

use scribe_core::library::SessionSummaryView;
use scribe_core::ports::TranscriptStore;
use scribe_core::recording::SessionId;
use store_markdown::MarkdownTranscriptStore;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_opener::OpenerExt;

use crate::claude_desktop::ConnectionStatus;
use crate::live_feed::{LiveFeed, RecorderSnapshot};
use crate::model_setup::{self, ModelDownloadView};
use crate::notifications;
use crate::recording::{self, RecordingController};
use crate::transcripts_location::{self, FolderChange};

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
    if !path.is_file() {
        return Err(format!("Transcript not found: {id}"));
    }
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn model_download_status<R: Runtime>(app: AppHandle<R>) -> ModelDownloadView {
    model_setup::status(&app)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn retry_model_download<R: Runtime>(app: AppHandle<R>) {
    model_setup::retry(&app);
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn connect_claude_desktop<R: Runtime>(app: AppHandle<R>) -> ConnectionStatus {
    crate::claude_desktop_connect::connect_claude_desktop(&app);
    crate::claude_desktop_connect::claude_desktop_status()
}

#[tauri::command]
pub fn claude_desktop_status() -> ConnectionStatus {
    crate::claude_desktop_connect::claude_desktop_status()
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn copy_mcp_config<R: Runtime>(app: AppHandle<R>) {
    crate::mcp_clipboard::copy_config(&app);
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn open_transcripts_folder<R: Runtime>(app: AppHandle<R>) {
    crate::tray::open_transcripts_folder(&app);
}

#[tauri::command]
pub async fn change_transcripts_folder<R: Runtime>(
    app: AppHandle<R>,
) -> Result<FolderChange, String> {
    transcripts_location::change(&app).map_err(|error| {
        notifications::notify_transcripts_folder_not_changed(&app);
        error.to_string()
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn transcripts_folder<R: Runtime>(app: AppHandle<R>) -> String {
    transcripts_location::current_folder(&app)
}
