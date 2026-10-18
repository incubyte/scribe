use scribe_core::ports::{CaptureError, StoreError, TranscribeError};
use scribe_core::recording::RecorderError;
use scribe_tray::start_failure::{StartError, StartFailureKind};

fn kind_of(error: impl Into<StartError>) -> StartFailureKind {
    error.into().kind()
}

#[test]
fn a_denied_microphone_is_told_apart_so_the_user_can_be_sent_to_settings() {
    let kind = kind_of(RecorderError::Capture(CaptureError::PermissionDenied));

    assert_eq!(kind, StartFailureKind::MicrophoneDenied);
    assert_eq!(kind.as_str(), "microphone_denied");
}

#[test]
fn a_missing_microphone_is_reported_as_no_microphone() {
    let kind = kind_of(RecorderError::Capture(CaptureError::DeviceUnavailable));

    assert_eq!(kind.as_str(), "no_microphone");
}

#[test]
fn a_speech_engine_that_cannot_start_is_reported_with_advice() {
    let kind = kind_of(TranscribeError::EngineUnavailable(
        "asset missing".to_owned(),
    ));

    assert_eq!(kind.as_str(), "speech_engine");
    assert!(!kind.advice().is_empty());
}

#[test]
fn an_unwritable_transcripts_folder_is_reported_as_a_folder_problem() {
    let kind = kind_of(RecorderError::Store(StoreError::DocumentsFolderUnavailable));

    assert_eq!(kind.as_str(), "transcripts_folder");
}

#[test]
fn starting_twice_is_reported_as_already_recording() {
    let kind = kind_of(RecorderError::AlreadyRecording);

    assert_eq!(kind.as_str(), "already_recording");
}

#[test]
fn the_advice_never_repeats_the_engine_message() {
    let kind = kind_of(TranscribeError::Engine("private detail".to_owned()));

    assert!(!kind.advice().contains("private detail"));
}
