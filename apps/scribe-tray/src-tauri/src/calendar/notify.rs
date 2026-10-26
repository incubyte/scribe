use tauri::{AppHandle, Runtime};

use crate::notifications::{self, RECORDING_NOT_STARTED_TITLE};
use crate::start_failure::StartFailureKind;

const RECORDING_TITLE: &str = "Scribe started recording";
const SAVED_TITLE: &str = "Transcript saved";
const MISSED_TITLE: &str = "No Teams call detected";

pub fn notify_meeting_recording<R: Runtime>(app: &AppHandle<R>, subject: &str) {
    let body = format!("Recording {subject}. Scribe stops when the call ends.");
    notifications::show_plain(app, RECORDING_TITLE, &body);
}

pub fn notify_meeting_saved<R: Runtime>(app: &AppHandle<R>, subject: &str) {
    notifications::show_plain(app, SAVED_TITLE, &format!("Saved {subject}"));
}

pub fn notify_meeting_missed<R: Runtime>(app: &AppHandle<R>, subject: &str) {
    let body = format!("{subject} is set to record, but no Teams call was detected.");
    notifications::show_plain(app, MISSED_TITLE, &body);
}

pub fn notify_meeting_not_started<R: Runtime>(
    app: &AppHandle<R>,
    subject: &str,
    kind: StartFailureKind,
) {
    let body = format!("{subject} wasn't recorded. {}", kind.advice());
    notifications::show_plain(app, RECORDING_NOT_STARTED_TITLE, &body);
}
