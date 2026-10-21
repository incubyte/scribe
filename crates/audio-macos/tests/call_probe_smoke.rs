#![cfg(target_os = "macos")]

use audio_macos::CoreAudioCallProbe;
use scribe_core::ports::CallProbe;

#[test]
#[ignore = "needs a live Microsoft Teams call"]
fn a_teams_call_in_progress_is_detected() {
    let probe = CoreAudioCallProbe;

    let detected = probe.call_app_holds_microphone();

    assert!(
        matches!(detected, Ok(true)),
        "bundle ids holding the microphone: {:?}",
        probe.bundle_ids_using_microphone()
    );
}
