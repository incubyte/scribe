#![cfg(windows)]

use audio_windows::WasapiCallProbe;
use scribe_core::ports::CallProbe;

#[test]
#[ignore = "needs a live Microsoft Teams call on Windows"]
fn a_teams_call_in_progress_is_detected() {
    let detected = WasapiCallProbe.call_app_holds_microphone();

    assert!(matches!(detected, Ok(true)), "probe result: {detected:?}");
}
