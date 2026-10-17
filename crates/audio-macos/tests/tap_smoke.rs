#![cfg(target_os = "macos")]

use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use audio_macos::SystemOutputSource;
use scribe_core::ports::AudioSource;
use scribe_core::recording::SessionClock;
use scribe_core::testing::RecordingListenerSpy;

const TONE: &str = "/System/Library/Sounds/Glass.aiff";
const PATIENCE: Duration = Duration::from_secs(2);

#[test]
#[ignore = "needs real audio hardware and the System Audio Recording permission"]
fn sound_played_by_another_app_reaches_the_other_side_lane() {
    let mut source = SystemOutputSource::new(Arc::new(RecordingListenerSpy::default()));
    let chunks = source.start(SessionClock::start()).unwrap();
    let mut player = Command::new("afplay").arg(TONE).spawn().unwrap();

    let deadline = Instant::now() + PATIENCE;
    let heard_sound = std::iter::from_fn(|| chunks.recv_timeout(PATIENCE).ok())
        .take_while(|_| Instant::now() < deadline)
        .any(|chunk| !chunk.is_exact_silence());
    source.stop();
    let _ = player.wait();

    assert!(heard_sound);
}
