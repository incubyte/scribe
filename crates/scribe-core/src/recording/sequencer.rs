use std::collections::BTreeMap;
use std::time::Duration;

use crate::recording::{Segment, Speaker};

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
            .partition_point(|held| held.start <= segment.start);
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
            .take_while(|segment| self.other_lanes_have_passed(segment))
            .count();
        self.pending.drain(..ready).collect()
    }

    pub fn drain(&mut self) -> Vec<Segment> {
        std::mem::take(&mut self.pending)
    }

    fn other_lanes_have_passed(&self, segment: &Segment) -> bool {
        self.watermarks
            .iter()
            .filter(|(lane, _)| **lane != segment.speaker)
            .all(|(_, watermark)| *watermark >= segment.start)
    }
}
