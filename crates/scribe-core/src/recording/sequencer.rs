use std::collections::BTreeMap;
use std::time::Duration;

use crate::recording::{Segment, Speaker};

pub const MAX_HOLD: Duration = Duration::from_millis(1_500);

pub struct SegmentSequencer {
    watermarks: BTreeMap<Speaker, Duration>,
    pending: Vec<Segment>,
}

impl SegmentSequencer {
    #[must_use]
    pub fn new(lanes: &[Speaker]) -> SegmentSequencer {
        SegmentSequencer {
            watermarks: lanes.iter().map(|lane| (*lane, Duration::ZERO)).collect(),
            pending: Vec::new(),
        }
    }

    pub fn accept(&mut self, segment: Segment) {
        let position = self
            .pending
            .partition_point(|held| release_order(held) <= release_order(&segment));
        self.pending.insert(position, segment);
    }

    pub fn advance(&mut self, speaker: Speaker, watermark: Duration) {
        let current = self.watermarks.entry(speaker).or_insert(Duration::ZERO);
        *current = (*current).max(watermark);
    }

    pub fn release_ready(&mut self) -> Vec<Segment> {
        let ready = self
            .pending
            .iter()
            .take_while(|segment| self.is_ready(segment))
            .count();
        self.pending.drain(..ready).collect()
    }

    pub fn drain(&mut self) -> Vec<Segment> {
        std::mem::take(&mut self.pending)
    }

    fn is_ready(&self, segment: &Segment) -> bool {
        self.other_lanes_have_passed(segment) || self.has_been_held_too_long(segment)
    }

    fn has_been_held_too_long(&self, segment: &Segment) -> bool {
        self.watermarks
            .get(&segment.speaker)
            .is_some_and(|watermark| *watermark >= segment.end.saturating_add(MAX_HOLD))
    }

    fn other_lanes_have_passed(&self, segment: &Segment) -> bool {
        self.watermarks
            .iter()
            .filter(|(lane, _)| **lane != segment.speaker)
            .all(|(_, watermark)| *watermark >= segment.start)
    }
}

fn release_order(segment: &Segment) -> (Duration, Speaker) {
    (segment.start, segment.speaker)
}
