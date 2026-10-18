#![cfg(windows)]

use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use audio_windows::LoopbackSource;
use scribe_core::ports::AudioSource;
use scribe_core::recording::SessionClock;
use scribe_core::testing::RecordingListenerSpy;

const PLAY_CHIME: &str =
    "(New-Object Media.SoundPlayer 'C:\\Windows\\Media\\chimes.wav').PlaySync()";
const PATIENCE: Duration = Duration::from_secs(3);

#[test]
#[ignore = "needs real audio hardware with a default output device"]
fn sound_played_by_another_app_reaches_the_other_side_lane() {
    let mut source = LoopbackSource::new(Arc::new(RecordingListenerSpy::default()));
    let chunks = source.start(SessionClock::start()).unwrap();
    let mut player = Command::new("powershell")
        .args(["-NoProfile", "-Command", PLAY_CHIME])
        .spawn()
        .unwrap();

    let deadline = Instant::now() + PATIENCE;
    let heard_sound = std::iter::from_fn(|| chunks.recv_timeout(PATIENCE).ok())
        .take_while(|_| Instant::now() < deadline)
        .any(|chunk| !chunk.is_exact_silence());
    source.stop();
    let _ = player.wait();

    assert!(heard_sound);
}
