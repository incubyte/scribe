use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, FixedOffset, TimeDelta};
use scribe_core::library::{Limit, SearchQuery, TranscriptLibrary};
use scribe_core::ports::TranscriptStore;
use scribe_core::recording::{Segment, SessionHeader, Speaker};
use scribe_core::testing::{InMemoryScreenshotStore, InMemoryTranscriptStore};

const CALL_COUNT: usize = 500;
const SEGMENTS_PER_HOUR: u64 = 720;
const SECONDS_PER_SEGMENT: u64 = 5;
const MATCHING_CALL: usize = 250;
const MATCHING_LINE: u64 = 360;
const SEARCH_BUDGET: Duration = Duration::from_secs(1);

fn base_moment() -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339("2026-10-01T09:00:00+00:00").unwrap()
}

fn spoken_line(call: usize, line: u64) -> Segment {
    let text = if call == MATCHING_CALL && line == MATCHING_LINE {
        "we agreed on the pricing decision".to_owned()
    } else {
        format!("talking through the roadmap item {line}")
    };
    Segment {
        speaker: Speaker::Them,
        start: Duration::from_secs(line * SECONDS_PER_SEGMENT),
        end: Duration::from_secs(line * SECONDS_PER_SEGMENT),
        text,
    }
}

fn save_one_hour_call(store: &InMemoryTranscriptStore, call: usize) {
    let started_at = base_moment() + TimeDelta::minutes(i64::try_from(call).unwrap());
    let header = SessionHeader::recording(started_at, "speechanalyzer", "en-US");
    let id = store.create(&header).unwrap();
    for line in 0..SEGMENTS_PER_HOUR {
        store.append(&id, &spoken_line(call, line)).unwrap();
    }
    store
        .finalise(&id, started_at + TimeDelta::hours(1))
        .unwrap();
}

#[test]
#[ignore = "timing budget: run by hand in release mode"]
fn transcript_search_performance_stays_under_one_second() {
    let store = Arc::new(InMemoryTranscriptStore::default());
    for call in 0..CALL_COUNT {
        save_one_hour_call(&store, call);
    }
    let library = TranscriptLibrary::new(store, Arc::new(InMemoryScreenshotStore::default()));
    let query = SearchQuery::parse("pricing decision").unwrap();

    let started = Instant::now();
    let hits = library.search(&query, Limit::clamped(100)).unwrap();
    let elapsed = started.elapsed();

    eprintln!("searched {CALL_COUNT} one-hour transcripts in {elapsed:?}");
    assert_eq!(hits.len(), 1);
    assert!(elapsed < SEARCH_BUDGET, "search took {elapsed:?}");
}
