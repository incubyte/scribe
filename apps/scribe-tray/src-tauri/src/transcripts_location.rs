use std::path::{Path, PathBuf};
use std::sync::Arc;

use scribe_core::recording::{RecorderError, RecorderState};
use serde::Serialize;
use serde_json::json;
use store_markdown::{
    MarkdownTranscriptStore, MoveReport, SETTINGS_FILE_NAME, TRANSCRIPTS_FOLDER_KEY,
    TranscriptsFolder, count_transcripts, move_transcripts,
};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_store::StoreExt;

use crate::notifications;
use crate::recording::RecordingController;

const PICKER_TITLE: &str = "Choose where Scribe saves transcripts";
const MOVE_PROMPT_TITLE: &str = "Move existing transcripts?";
const MOVE_LABEL: &str = "Move";
const KEEP_LABEL: &str = "Keep in old folder";

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum FolderChange {
    Cancelled,
    Unchanged,
    Recording,
    Changed {
        folder: String,
        moved: usize,
        kept: usize,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum FolderChangeError {
    #[error("the chosen folder cannot be saved as a setting")]
    UnsupportedPath,
    #[error("could not save the setting: {0}")]
    Settings(#[from] tauri_plugin_store::Error),
    #[error("could not switch folders: {0}")]
    Recorder(#[from] RecorderError),
}

struct FolderSwitch {
    previous: TranscriptsFolder,
    new: TranscriptsFolder,
    new_text: String,
}

pub fn change<R: Runtime>(app: &AppHandle<R>) -> Result<FolderChange, FolderChangeError> {
    if app.state::<RecordingController>().state() == RecorderState::Recording {
        return Ok(FolderChange::Recording);
    }
    let previous = app.state::<Arc<MarkdownTranscriptStore>>().folder();
    let Some(picked) = pick_folder(app, previous.path())? else {
        return Ok(FolderChange::Cancelled);
    };
    let new = TranscriptsFolder::at(picked);
    if new == previous {
        return Ok(FolderChange::Unchanged);
    }
    let new_text = utf8(new.path())?;
    switch_folder(
        app,
        &FolderSwitch {
            previous,
            new,
            new_text,
        },
    )
}

fn switch_folder<R: Runtime>(
    app: &AppHandle<R>,
    switch: &FolderSwitch,
) -> Result<FolderChange, FolderChangeError> {
    match relocate_and_persist(app, switch) {
        Err(FolderChangeError::Recorder(RecorderError::AlreadyRecording)) => {
            Ok(FolderChange::Recording)
        }
        Err(error) => Err(error),
        Ok(()) => Ok(finish(app, switch)),
    }
}

fn relocate_and_persist<R: Runtime>(
    app: &AppHandle<R>,
    switch: &FolderSwitch,
) -> Result<(), FolderChangeError> {
    let controller = app.state::<RecordingController>();
    let store = app.state::<Arc<MarkdownTranscriptStore>>();
    controller.relocate_store(&store, switch.new.clone())?;
    persist_folder(app, &switch.new_text).inspect_err(|_| {
        if let Err(error) = controller.relocate_store(&store, switch.previous.clone()) {
            tracing::error!(%error, "could not restore the previous transcripts folder");
        }
    })
}

fn finish<R: Runtime>(app: &AppHandle<R>, switch: &FolderSwitch) -> FolderChange {
    let report = offer_to_move(app, &switch.previous, &switch.new);
    notifications::notify_transcripts_folder_changed(app, switch.new.path(), report.moved);
    FolderChange::Changed {
        folder: switch.new_text.clone(),
        moved: report.moved,
        kept: report.kept,
    }
}

pub fn current_folder<R: Runtime>(app: &AppHandle<R>) -> String {
    let folder = app.state::<Arc<MarkdownTranscriptStore>>().folder();
    folder.path().to_string_lossy().into_owned()
}

fn utf8(path: &Path) -> Result<String, FolderChangeError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or(FolderChangeError::UnsupportedPath)
}

fn pick_folder<R: Runtime>(
    app: &AppHandle<R>,
    starting_at: &Path,
) -> Result<Option<PathBuf>, FolderChangeError> {
    app.dialog()
        .file()
        .set_title(PICKER_TITLE)
        .set_directory(starting_at)
        .blocking_pick_folder()
        .map(|picked| {
            picked
                .into_path()
                .map_err(|_| FolderChangeError::UnsupportedPath)
        })
        .transpose()
}

fn persist_folder<R: Runtime>(app: &AppHandle<R>, folder: &str) -> Result<(), FolderChangeError> {
    let store = app.store(SETTINGS_FILE_NAME)?;
    store.set(TRANSCRIPTS_FOLDER_KEY, json!(folder));
    Ok(store.save()?)
}

fn offer_to_move<R: Runtime>(
    app: &AppHandle<R>,
    from: &TranscriptsFolder,
    to: &TranscriptsFolder,
) -> MoveReport {
    let outcome = match count_transcripts(from) {
        Ok(count) if count > 0 && ask_to_move(app, count) => move_transcripts(from, to),
        Ok(_) => Ok(MoveReport::default()),
        Err(error) => Err(error),
    };
    outcome.unwrap_or_else(|error| {
        tracing::error!(%error, "could not move the existing transcripts");
        MoveReport::default()
    })
}

fn ask_to_move<R: Runtime>(app: &AppHandle<R>, count: usize) -> bool {
    app.dialog()
        .message(format!("Move {count} transcripts to the new folder?"))
        .title(MOVE_PROMPT_TITLE)
        .buttons(MessageDialogButtons::OkCancelCustom(
            MOVE_LABEL.to_owned(),
            KEEP_LABEL.to_owned(),
        ))
        .blocking_show()
}
