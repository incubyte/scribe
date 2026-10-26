use chrono::{DateTime, FixedOffset, SecondsFormat};
use scribe_core::calendar::{
    AgendaLayout, Meeting, OverflowChip, PlacedMeeting, TodaysAgenda, lay_out,
};
use serde::Serialize;

use super::agenda_snapshot::{AgendaSnapshot, FetchedAgenda};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgendaView {
    status: &'static str,
    stale: bool,
    day_start: Option<String>,
    fetched_at: Option<String>,
    blocks: Vec<BlockView>,
    overflow: Vec<ChipView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlockView {
    event_id: String,
    subject: String,
    starts_at: String,
    ends_at: String,
    is_teams: bool,
    is_recurring: bool,
    top_minutes: u32,
    height_minutes: u32,
    column: u8,
    columns: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChipView {
    top_minutes: u32,
    height_minutes: u32,
    hidden: Vec<HiddenMeetingView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HiddenMeetingView {
    event_id: String,
    subject: String,
    starts_at: String,
    ends_at: String,
}

impl AgendaView {
    #[must_use]
    pub fn of(snapshot: &AgendaSnapshot) -> AgendaView {
        match &snapshot.fetched {
            Some(fetched) => ready(fetched, snapshot.refresh_failed),
            None => without_meetings(snapshot.refresh_failed),
        }
    }
}

fn without_meetings(refresh_failed: bool) -> AgendaView {
    AgendaView {
        status: if refresh_failed { "failed" } else { "loading" },
        stale: false,
        day_start: None,
        fetched_at: None,
        blocks: Vec::new(),
        overflow: Vec::new(),
    }
}

fn ready(fetched: &FetchedAgenda, refresh_failed: bool) -> AgendaView {
    let layout = lay_out(&fetched.agenda, &fetched.day);
    AgendaView {
        status: "ready",
        stale: refresh_failed,
        day_start: Some(clock_text(fetched.day.start)),
        fetched_at: Some(clock_text(fetched.at)),
        blocks: layout.placed.iter().map(BlockView::from).collect(),
        overflow: chips(&layout, &fetched.agenda),
    }
}

fn chips(layout: &AgendaLayout, agenda: &TodaysAgenda) -> Vec<ChipView> {
    layout
        .overflow
        .iter()
        .map(|chip| ChipView::of(chip, agenda))
        .collect()
}

impl From<&PlacedMeeting> for BlockView {
    fn from(placed: &PlacedMeeting) -> BlockView {
        let meeting = &placed.meeting;
        BlockView {
            event_id: meeting.event_id.as_str().to_owned(),
            subject: meeting.subject.clone(),
            starts_at: clock_text(meeting.starts_at),
            ends_at: clock_text(meeting.ends_at),
            is_teams: meeting.is_teams,
            is_recurring: meeting.is_recurring,
            top_minutes: placed.top_minutes,
            height_minutes: placed.height_minutes,
            column: placed.column,
            columns: placed.columns,
        }
    }
}

impl ChipView {
    fn of(chip: &OverflowChip, agenda: &TodaysAgenda) -> ChipView {
        ChipView {
            top_minutes: chip.top_minutes,
            height_minutes: chip.height_minutes,
            hidden: agenda
                .meetings()
                .iter()
                .filter(|meeting| chip.hidden.contains(&meeting.event_id))
                .map(HiddenMeetingView::from)
                .collect(),
        }
    }
}

impl From<&Meeting> for HiddenMeetingView {
    fn from(meeting: &Meeting) -> HiddenMeetingView {
        HiddenMeetingView {
            event_id: meeting.event_id.as_str().to_owned(),
            subject: meeting.subject.clone(),
            starts_at: clock_text(meeting.starts_at),
            ends_at: clock_text(meeting.ends_at),
        }
    }
}

fn clock_text(moment: DateTime<FixedOffset>) -> String {
    moment.to_rfc3339_opts(SecondsFormat::Secs, false)
}
