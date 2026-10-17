use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use scribe_core::recording::{RecorderState, Segment, Speaker};
use scribe_tray::live_feed::LiveFeed;
use serde_json::json;

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn line(speaker: Speaker, start_secs: u64, text: &str) -> Segment {
    Segment {
        speaker,
        start: Duration::from_secs(start_secs),
        end: Duration::from_secs(start_secs + 1),
        text: text.to_owned(),
    }
}

#[test]
fn a_feed_that_has_not_started_has_no_lines_and_no_start_time() {
    let snapshot = LiveFeed::default().snapshot(RecorderState::Idle);

    assert_eq!(
        serde_json::to_value(snapshot).unwrap(),
        json!({ "state": "idle", "started_at": null, "lines": [] })
    );
}

#[test]
fn finalised_lines_are_kept_in_the_order_they_arrived() {
    let feed = LiveFeed::default();
    feed.started(moment("2026-10-17T09:00:00+00:00"));

    feed.finalised(&line(Speaker::Them, 1, "hello"));
    feed.finalised(&line(Speaker::Me, 3, "hi there"));

    assert_eq!(
        serde_json::to_value(feed.snapshot(RecorderState::Recording)).unwrap(),
        json!({
            "state": "recording",
            "started_at": "2026-10-17T09:00:00+00:00",
            "lines": [
                { "speaker": "Them", "start_ms": 1000, "text": "hello" },
                { "speaker": "Me", "start_ms": 3000, "text": "hi there" }
            ]
        })
    );
}

#[test]
fn a_new_recording_begins_with_an_empty_transcript() {
    let feed = LiveFeed::default();
    feed.started(moment("2026-10-17T09:00:00+00:00"));
    feed.finalised(&line(Speaker::Me, 1, "left over"));

    feed.started(moment("2026-10-17T10:00:00+00:00"));

    let snapshot = serde_json::to_value(feed.snapshot(RecorderState::Recording)).unwrap();
    assert_eq!(snapshot["lines"], json!([]));
}

#[test]
fn stopping_clears_the_transcript_and_the_start_time() {
    let feed = LiveFeed::default();
    feed.started(moment("2026-10-17T09:00:00+00:00"));
    feed.finalised(&line(Speaker::Me, 1, "bye"));

    feed.stopped();

    assert_eq!(
        serde_json::to_value(feed.snapshot(RecorderState::Idle)).unwrap(),
        json!({ "state": "idle", "started_at": null, "lines": [] })
    );
}

#[test]
fn a_locked_recorder_is_reported_as_locked() {
    let snapshot = LiveFeed::default().snapshot(RecorderState::Locked);

    assert_eq!(serde_json::to_value(snapshot).unwrap()["state"], "locked");
}
