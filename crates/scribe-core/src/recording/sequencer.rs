use std::collections::BTreeMap;
use std::time::Duration;

use crate::recording::{
    ECHO_OVERLAP_TOLERANCE, ECHO_SIMILARITY_THRESHOLD, EchoFilter, Segment, Speaker,
};

pub const MAX_HOLD: Duration = Duration::from_millis(1_500);
const RECENT_THEM_WINDOW: Duration = Duration::from_secs(3);

pub struct SegmentSequencer {
    finalised_through: BTreeMap<Speaker, Duration>,
    heard_through: BTreeMap<Speaker, Duration>,
    final_results_waits: BTreeMap<Speaker, Duration>,
    pending: Vec<Segment>,
    recent_them: Vec<Segment>,
    echo_filter: EchoFilter,
}

impl SegmentSequencer {
    #[must_use]
    pub fn new(lanes: &[Speaker]) -> SegmentSequencer {
        let starting_points: BTreeMap<Speaker, Duration> =
            lanes.iter().map(|lane| (*lane, Duration::ZERO)).collect();
        SegmentSequencer {
            finalised_through: starting_points.clone(),
            heard_through: starting_points,
            final_results_waits: BTreeMap::new(),
            pending: Vec::new(),
            recent_them: Vec::new(),
            echo_filter: EchoFilter::new(ECHO_OVERLAP_TOLERANCE, ECHO_SIMILARITY_THRESHOLD),
        }
    }

    pub fn accept(&mut self, segment: Segment) {
        let position = self
            .pending
            .partition_point(|held| release_order(held) <= release_order(&segment));
        self.pending.insert(position, segment);
    }

    pub fn advance(&mut self, speaker: Speaker, finalised_through: Duration) {
        self.finalised_through.insert(speaker, finalised_through);
        self.hear(speaker, finalised_through);
    }

    pub fn limit_wait_for_finals(&mut self, speaker: Speaker, wait: Duration) {
        self.final_results_waits.insert(speaker, wait);
    }

    pub fn hear(&mut self, speaker: Speaker, heard_through: Duration) {
        raise(&mut self.heard_through, speaker, heard_through);
    }

    pub fn close(&mut self, speaker: Speaker) {
        self.finalised_through.insert(speaker, Duration::MAX);
    }

    pub fn release_ready(&mut self) -> Vec<Segment> {
        let mut released = Vec::new();
        while self
            .pending
            .first()
            .is_some_and(|front| self.is_ready(front))
        {
            released.extend(self.release_front());
        }
        released
    }

    pub fn drain(&mut self) -> Vec<Segment> {
        let mut released = Vec::new();
        while !self.pending.is_empty() {
            released.extend(self.release_front());
        }
        released
    }

    fn release_front(&mut self) -> Option<Segment> {
        let segment = self.pending.remove(0);
        match segment.speaker {
            Speaker::Me => self.screen_for_echo(segment),
            Speaker::Them => Some(self.remember_recent(segment)),
        }
    }

    fn screen_for_echo(&self, mine: Segment) -> Option<Segment> {
        let theirs: Vec<Segment> = self
            .recent_them
            .iter()
            .chain(
                self.pending
                    .iter()
                    .filter(|held| held.speaker == Speaker::Them),
            )
            .cloned()
            .collect();
        (!self.echo_filter.is_echo(&mine, &theirs)).then_some(mine)
    }

    fn remember_recent(&mut self, theirs: Segment) -> Segment {
        self.recent_them
            .retain(|recent| recent.end.saturating_add(RECENT_THEM_WINDOW) >= theirs.start);
        self.recent_them.push(theirs.clone());
        theirs
    }

    fn is_ready(&self, segment: &Segment) -> bool {
        let needed = self.watermark_needed_to_release(segment);
        self.finalised_through
            .iter()
            .filter(|(lane, _)| **lane != segment.speaker)
            .all(|(lane, finalised)| {
                *finalised >= needed
                    || self.has_stopped_waiting_for(*lane, segment)
                    || self.has_waited_long_enough_for_finals(*lane, needed)
            })
    }

    fn has_waited_long_enough_for_finals(&self, lane: Speaker, needed: Duration) -> bool {
        self.final_results_waits
            .get(&lane)
            .is_some_and(|wait| self.heard(lane) >= needed.saturating_add(*wait))
    }

    fn has_stopped_waiting_for(&self, lane: Speaker, segment: &Segment) -> bool {
        let own = self.heard(segment.speaker);
        let held_too_long = own >= segment.end.saturating_add(MAX_HOLD);
        let lane_has_stalled = self.heard(lane).saturating_add(MAX_HOLD) <= own;
        held_too_long && lane_has_stalled
    }

    fn heard(&self, lane: Speaker) -> Duration {
        self.heard_through.get(&lane).copied().unwrap_or_default()
    }

    fn watermark_needed_to_release(&self, segment: &Segment) -> Duration {
        match segment.speaker {
            Speaker::Me => segment
                .end
                .saturating_add(self.echo_filter.overlap_tolerance()),
            Speaker::Them => segment.start,
        }
    }
}

fn release_order(segment: &Segment) -> (Duration, Speaker) {
    (segment.start, segment.speaker)
}

fn raise(progress: &mut BTreeMap<Speaker, Duration>, speaker: Speaker, to: Duration) {
    let current = progress.entry(speaker).or_insert(Duration::ZERO);
    *current = (*current).max(to);
}
