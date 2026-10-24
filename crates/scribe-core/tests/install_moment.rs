use scribe_core::recording::RecorderState;
use scribe_core::update::{InstallConditions, InstallMoment, WaitReason, install_moment};

const IDLE_WITH_UPDATE_WAITING: InstallConditions = InstallConditions {
    update_waiting: true,
    recorder: RecorderState::Idle,
    in_call: false,
    popover_open: false,
};

#[test]
fn installs_when_an_update_waits_and_nothing_is_happening() {
    assert_eq!(
        install_moment(&IDLE_WITH_UPDATE_WAITING),
        InstallMoment::Now
    );
}

#[test]
fn waits_when_no_update_is_waiting() {
    let conditions = InstallConditions {
        update_waiting: false,
        ..IDLE_WITH_UPDATE_WAITING
    };

    assert_eq!(
        install_moment(&conditions),
        InstallMoment::Wait(WaitReason::NothingReady)
    );
}

#[test]
fn waits_while_recording() {
    let conditions = InstallConditions {
        recorder: RecorderState::Recording,
        ..IDLE_WITH_UPDATE_WAITING
    };

    assert_eq!(
        install_moment(&conditions),
        InstallMoment::Wait(WaitReason::Recording)
    );
}

#[test]
fn waits_during_a_call_even_when_not_recording() {
    let conditions = InstallConditions {
        in_call: true,
        ..IDLE_WITH_UPDATE_WAITING
    };

    assert_eq!(
        install_moment(&conditions),
        InstallMoment::Wait(WaitReason::InCall)
    );
}

#[test]
fn waits_while_the_popover_is_open() {
    let conditions = InstallConditions {
        popover_open: true,
        ..IDLE_WITH_UPDATE_WAITING
    };

    assert_eq!(
        install_moment(&conditions),
        InstallMoment::Wait(WaitReason::PopoverOpen)
    );
}

#[test]
fn a_locked_recorder_does_not_hold_back_an_update() {
    let conditions = InstallConditions {
        recorder: RecorderState::Locked,
        ..IDLE_WITH_UPDATE_WAITING
    };

    assert_eq!(install_moment(&conditions), InstallMoment::Now);
}

#[test]
fn recording_takes_precedence_over_a_call_and_an_open_popover() {
    let conditions = InstallConditions {
        recorder: RecorderState::Recording,
        in_call: true,
        popover_open: true,
        ..IDLE_WITH_UPDATE_WAITING
    };

    assert_eq!(
        install_moment(&conditions),
        InstallMoment::Wait(WaitReason::Recording)
    );
}
