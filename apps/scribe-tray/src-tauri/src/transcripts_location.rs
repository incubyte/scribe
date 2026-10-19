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
use crate::popover;
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
    new_folder: TranscriptsFolder,
    new_folder_text: String,
}

struct MoveFailed;

enum Preflight {
    Proceed(FolderSwitch),
    Stop(FolderChange),
}

pub fn change<R: Runtime>(app: &AppHandle<R>) -> Result<FolderChange, FolderChangeError> {
    match preflight(app)? {
        Preflight::Proceed(switch) => switch_folder(app, &switch),
        Preflight::Stop(outcome) => Ok(outcome),
    }
}

fn preflight<R: Runtime>(app: &AppHandle<R>) -> Result<Preflight, FolderChangeError> {
    if app.state::<RecordingController>().state() == RecorderState::Recording {
        return Ok(Preflight::Stop(FolderChange::Recording));
    }
    let previous = app.state::<Arc<MarkdownTranscriptStore>>().folder();
    let Some(picked) = pick_folder(app, previous.path())? else {
        return Ok(Preflight::Stop(FolderChange::Cancelled));
    };
    let new_folder = TranscriptsFolder::at(picked);
    if new_folder == previous {
        return Ok(Preflight::Stop(FolderChange::Unchanged));
    }
    let new_folder_text = path_as_utf8(new_folder.path())?;
    Ok(Preflight::Proceed(FolderSwitch {
        previous,
        new_folder,
        new_folder_text,
    }))
}

#[must_use]
pub fn folder_changed_message(folder_name: &str, moved: usize, kept: usize) -> String {
    let saved = format!("New transcripts will be saved in {folder_name}.");
    let moved_note = match moved {
        0 => String::new(),
        1 => " Moved 1 transcript.".to_owned(),
        count => format!(" Moved {count} transcripts."),
    };
    let kept_note = match kept {
        0 => String::new(),
        1 => " 1 transcript was left in the old folder.".to_owned(),
        count => format!(" {count} transcripts were left in the old folder."),
    };
    format!("{saved}{moved_note}{kept_note}")
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
    controller.relocate_store(&store, switch.new_folder.clone())?;
    persist_folder(app, &switch.new_folder_text).inspect_err(|_| {
        if let Err(error) = controller.relocate_store(&store, switch.previous.clone()) {
            tracing::error!(%error, "could not restore the previous transcripts folder");
        }
    })
}

fn finish<R: Runtime>(app: &AppHandle<R>, switch: &FolderSwitch) -> FolderChange {
    let report = if let Ok(report) = offer_to_move(app, &switch.previous, &switch.new_folder) {
        notifications::notify_transcripts_folder_changed(app, switch.new_folder.path(), report);
        report
    } else {
        notifications::notify_transcripts_not_moved(app);
        MoveReport::default()
    };
    FolderChange::Changed {
        folder: switch.new_folder_text.clone(),
        moved: report.moved,
        kept: report.kept,
    }
}

pub fn current_folder<R: Runtime>(app: &AppHandle<R>) -> String {
    let folder = app.state::<Arc<MarkdownTranscriptStore>>().folder();
    folder.path().to_string_lossy().into_owned()
}

fn path_as_utf8(path: &Path) -> Result<String, FolderChangeError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or(FolderChangeError::UnsupportedPath)
}

fn pick_folder<R: Runtime>(
    app: &AppHandle<R>,
    starting_at: &Path,
) -> Result<Option<PathBuf>, FolderChangeError> {
    popover::keep_open_during(app, || {
        app.dialog()
            .file()
            .set_title(PICKER_TITLE)
            .set_directory(starting_at)
            .blocking_pick_folder()
    })
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
) -> Result<MoveReport, MoveFailed> {
    let outcome = match count_transcripts(from) {
        Ok(count) if count > 0 && ask_to_move(app, count) => move_transcripts(from, to),
        Ok(_) => Ok(MoveReport::default()),
        Err(error) => Err(error),
    };
    outcome.map_err(|_| {
        tracing::error!("could not move the existing transcripts");
        MoveFailed
    })
}

fn ask_to_move<R: Runtime>(app: &AppHandle<R>, count: usize) -> bool {
    popover::keep_open_during(app, || {
        app.dialog()
            .message(format!("Move {count} transcripts to the new folder?"))
            .title(MOVE_PROMPT_TITLE)
            .buttons(MessageDialogButtons::OkCancelCustom(
                MOVE_LABEL.to_owned(),
                KEEP_LABEL.to_owned(),
            ))
            .blocking_show()
    })
}
