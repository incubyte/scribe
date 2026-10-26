use chrono::{DateTime, FixedOffset};

use super::agenda::TodaysAgenda;
use super::meeting::{DayBounds, EventId, Meeting};

pub const MIN_BLOCK_MINUTES: u32 = 15;
pub const MAX_COLUMNS: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedMeeting {
    pub meeting: Meeting,
    pub top_minutes: u32,
    pub height_minutes: u32,
    pub column: u8,
    pub columns: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverflowChip {
    pub top_minutes: u32,
    pub height_minutes: u32,
    pub hidden: Vec<EventId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgendaLayout {
    pub placed: Vec<PlacedMeeting>,
    pub overflow: Vec<OverflowChip>,
}

struct Slot {
    meeting: Meeting,
    top: u32,
    bottom: u32,
    column: u8,
}

impl Slot {
    fn of(meeting: &Meeting, day: &DayBounds) -> Slot {
        let top = minutes_since(day.start, meeting.starts_at.max(day.start));
        let end = minutes_since(day.start, meeting.ends_at.min(day.end));
        Slot {
            meeting: meeting.clone(),
            top,
            bottom: end.max(top + MIN_BLOCK_MINUTES),
            column: 0,
        }
    }

    fn is_hidden(&self) -> bool {
        self.column >= MAX_COLUMNS
    }
}

#[must_use]
pub fn lay_out(agenda: &TodaysAgenda, day: &DayBounds) -> AgendaLayout {
    let slots = agenda
        .meetings()
        .iter()
        .map(|meeting| Slot::of(meeting, day))
        .collect();
    let mut layout = AgendaLayout::default();
    for cluster in clusters(slots) {
        arrange(cluster, &mut layout);
    }
    layout
}

fn minutes_since(start: DateTime<FixedOffset>, moment: DateTime<FixedOffset>) -> u32 {
    u32::try_from(moment.signed_duration_since(start).num_minutes()).unwrap_or(0)
}

fn clusters(slots: Vec<Slot>) -> Vec<Vec<Slot>> {
    let mut clusters: Vec<Vec<Slot>> = Vec::new();
    for slot in slots {
        match clusters.last_mut() {
            Some(cluster) if slot.top < cluster_bottom(cluster) => cluster.push(slot),
            _ => clusters.push(vec![slot]),
        }
    }
    clusters
}

fn cluster_bottom(cluster: &[Slot]) -> u32 {
    cluster.iter().map(|slot| slot.bottom).max().unwrap_or(0)
}

fn arrange(mut cluster: Vec<Slot>, layout: &mut AgendaLayout) {
    assign_columns(&mut cluster);
    let columns = visible_columns(&cluster);
    let (hidden, shown): (Vec<Slot>, Vec<Slot>) = cluster.into_iter().partition(Slot::is_hidden);
    layout
        .placed
        .extend(shown.into_iter().map(|slot| placed(slot, columns)));
    layout.overflow.extend(chip_for(&hidden));
}

fn assign_columns(cluster: &mut [Slot]) {
    let mut column_bottoms: Vec<u32> = Vec::new();
    for slot in cluster {
        let column = column_bottoms
            .iter()
            .position(|bottom| *bottom <= slot.top)
            .unwrap_or(column_bottoms.len());
        if column == column_bottoms.len() {
            column_bottoms.push(slot.bottom);
        } else {
            column_bottoms[column] = slot.bottom;
        }
        slot.column = u8::try_from(column).unwrap_or(u8::MAX);
    }
}

fn visible_columns(cluster: &[Slot]) -> u8 {
    let used = cluster
        .iter()
        .map(|slot| slot.column)
        .max()
        .map_or(1, |last| last.saturating_add(1));
    used.min(MAX_COLUMNS)
}

fn placed(slot: Slot, columns: u8) -> PlacedMeeting {
    PlacedMeeting {
        top_minutes: slot.top,
        height_minutes: slot.bottom - slot.top,
        column: slot.column,
        columns,
        meeting: slot.meeting,
    }
}

fn chip_for(hidden: &[Slot]) -> Option<OverflowChip> {
    let top = hidden.iter().map(|slot| slot.top).min()?;
    let bottom = hidden.iter().map(|slot| slot.bottom).max()?;
    Some(OverflowChip {
        top_minutes: top,
        height_minutes: bottom - top,
        hidden: hidden
            .iter()
            .map(|slot| slot.meeting.event_id.clone())
            .collect(),
    })
}
