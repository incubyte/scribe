use std::time::Duration;

use scribe_core::recording::{AudioChunk, SAMPLE_RATE_HZ, SilenceWatchdog, WatchdogVerdict};

const FIVE_SECONDS: Duration = Duration::from_secs(5);
const FAINT: f32 = 0.000_01;
const EXACT_ZERO: f32 = 0.0;

fn one_second_of(sample: f32, at_second: u64) -> AudioChunk {
    AudioChunk {
        samples: vec![sample; SAMPLE_RATE_HZ as usize],
        captured_at: Duration::from_secs(at_second),
    }
}

fn observe_seconds(
    watchdog: &mut SilenceWatchdog,
    sample: f32,
    seconds: std::ops::Range<u64>,
) -> Vec<WatchdogVerdict> {
    seconds
        .map(|second| watchdog.observe(&one_second_of(sample, second)))
        .collect()
}

#[test]
fn a_quiet_room_with_faint_sound_is_never_reported_as_silenced() {
    let mut watchdog = SilenceWatchdog::new(FIVE_SECONDS);

    let verdicts = observe_seconds(&mut watchdog, FAINT, 0..7);

    assert_eq!(verdicts, vec![WatchdogVerdict::Healthy; 7]);
}

#[test]
fn five_seconds_of_exact_zeros_are_reported_once() {
    let mut watchdog = SilenceWatchdog::new(FIVE_SECONDS);

    let verdicts = observe_seconds(&mut watchdog, EXACT_ZERO, 0..7);

    assert_eq!(
        verdicts,
        vec![
            WatchdogVerdict::Healthy,
            WatchdogVerdict::Healthy,
            WatchdogVerdict::Healthy,
            WatchdogVerdict::Healthy,
            WatchdogVerdict::SilencedFor(FIVE_SECONDS),
            WatchdogVerdict::AlreadyReported,
            WatchdogVerdict::AlreadyReported,
        ]
    );
}

#[test]
fn a_single_audible_sample_breaks_the_silence() {
    let mut watchdog = SilenceWatchdog::new(FIVE_SECONDS);
    let mut nearly_silent = one_second_of(EXACT_ZERO, 4);
    nearly_silent.samples[100] = FAINT;
    observe_seconds(&mut watchdog, EXACT_ZERO, 0..4);

    let verdict = watchdog.observe(&nearly_silent);

    assert_eq!(verdict, WatchdogVerdict::Healthy);
}

#[test]
fn sound_returning_after_a_reported_silence_allows_the_next_silence_to_be_reported() {
    let mut watchdog = SilenceWatchdog::new(FIVE_SECONDS);
    observe_seconds(&mut watchdog, EXACT_ZERO, 0..5);
    observe_seconds(&mut watchdog, FAINT, 5..6);

    let verdicts = observe_seconds(&mut watchdog, EXACT_ZERO, 6..11);

    assert_eq!(
        verdicts.last(),
        Some(&WatchdogVerdict::SilencedFor(FIVE_SECONDS))
    );
}
