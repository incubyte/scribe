use scribe_core::recording::{Readiness, RecorderState, recorder_state};

#[test]
fn recorder_is_locked_until_privacy_is_acknowledged() {
    let readiness = Readiness {
        privacy_acknowledged: false,
        model_ready: true,
    };
    assert_eq!(recorder_state(&readiness, false), RecorderState::Locked);
}

#[test]
fn recorder_is_locked_while_the_speech_model_is_missing() {
    let readiness = Readiness {
        privacy_acknowledged: true,
        model_ready: false,
    };
    assert_eq!(recorder_state(&readiness, false), RecorderState::Locked);
}

#[test]
fn recorder_is_locked_when_nothing_is_ready() {
    let readiness = Readiness {
        privacy_acknowledged: false,
        model_ready: false,
    };
    assert_eq!(recorder_state(&readiness, false), RecorderState::Locked);
}

#[test]
fn recorder_is_idle_when_privacy_and_model_are_ready() {
    let readiness = Readiness {
        privacy_acknowledged: true,
        model_ready: true,
    };
    assert_eq!(recorder_state(&readiness, false), RecorderState::Idle);
}

#[test]
fn recorder_is_recording_while_a_session_is_active() {
    let readiness = Readiness {
        privacy_acknowledged: true,
        model_ready: true,
    };
    assert_eq!(recorder_state(&readiness, true), RecorderState::Recording);
}
