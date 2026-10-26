use std::collections::HashSet;

use chrono::{DateTime, FixedOffset};

use super::agenda::TodaysAgenda;
use super::armed::ArmedMeetings;
use super::meeting::{EventId, Meeting};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchPrompt {
    pub from: Meeting,
    pub to: Meeting,
}

#[derive(Debug, Clone, Copy)]
pub struct SwitchInputs<'a> {
    pub linked: &'a Meeting,
    pub agenda: &'a TodaysAgenda,
    pub armed: &'a ArmedMeetings,
    pub titled: Option<&'a Meeting>,
    pub now: DateTime<FixedOffset>,
}

#[derive(Debug, Default)]
pub struct SwitchPrompts {
    offered: HashSet<EventId>,
}

impl SwitchPrompts {
    #[must_use]
    pub fn new() -> SwitchPrompts {
        SwitchPrompts::default()
    }

    pub fn due(&mut self, inputs: &SwitchInputs) -> Option<SwitchPrompt> {
        let next = inputs
            .agenda
            .meetings()
            .iter()
            .filter(|meeting| meeting.event_id != inputs.linked.event_id)
            .filter(|meeting| !self.offered.contains(&meeting.event_id))
            .find(|meeting| is_next(meeting, inputs))?;
        self.offered.insert(next.event_id.clone());
        Some(SwitchPrompt {
            from: inputs.linked.clone(),
            to: next.clone(),
        })
    }
}

fn is_next(meeting: &Meeting, inputs: &SwitchInputs) -> bool {
    is_named_in_titles(meeting, inputs) || is_armed_and_underway(meeting, inputs)
}

fn is_named_in_titles(meeting: &Meeting, inputs: &SwitchInputs) -> bool {
    inputs
        .titled
        .is_some_and(|titled| titled.event_id == meeting.event_id)
}

fn is_armed_and_underway(meeting: &Meeting, inputs: &SwitchInputs) -> bool {
    inputs.armed.is_armed(&meeting.event_id)
        && meeting.starts_at <= inputs.now
        && inputs.now < meeting.ends_at
}
