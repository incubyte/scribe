use std::time::Duration;

use scribe_core::recording::{Segment, SegmentSequencer, Speaker};

fn line(speaker: Speaker, (from_ms, to_ms): (u64, u64), text: &str) -> Segment {
    Segment {
        speaker,
        start: Duration::from_millis(from_ms),
        end: Duration::from_millis(to_ms),
        text: text.to_owned(),
    }
}

fn headphones_conversation() -> Vec<Segment> {
    vec![
        line(Speaker::Them, (0, 2_000), "Thanks for joining everyone"),
        line(Speaker::Me, (2_200, 3_800), "Happy to be here"),
        line(
            Speaker::Them,
            (4_000, 7_000),
            "Let's start with the roadmap for next quarter",
        ),
        line(Speaker::Me, (6_500, 8_000), "Before that, a quick question"),
        line(Speaker::Them, (8_200, 10_000), "Of course, go ahead"),
        line(
            Speaker::Me,
            (10_200, 12_500),
            "Is the launch date still the fifteenth",
        ),
    ]
}

#[test]
fn a_headphones_only_conversation_loses_no_lines() {
    let conversation = headphones_conversation();
    let mut sequencer = SegmentSequencer::new(&[Speaker::Me, Speaker::Them]);
    for segment in &conversation {
        sequencer.accept(segment.clone());
    }
    sequencer.advance(Speaker::Me, Duration::from_secs(20));
    sequencer.advance(Speaker::Them, Duration::from_secs(20));

    let mut kept = sequencer.release_ready();
    kept.extend(sequencer.drain());

    assert_eq!(kept.len(), conversation.len());
}
