use std::collections::HashMap;

use chrono::{DateTime, FixedOffset, NaiveDate};

use super::agenda::TodaysAgenda;
use super::meeting::{EventId, Meeting};
use super::{CANDIDATE_LEAD, MISSED_AFTER};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmedEntry {
    pub event_id: EventId,
    pub date: NaiveDate,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArmedMeetings {
    entries: Vec<ArmedEntry>,
}

impl ArmedMeetings {
    #[must_use]
    pub const fn new() -> ArmedMeetings {
        ArmedMeetings {
            entries: Vec::new(),
        }
    }

    #[must_use]
    pub fn for_day(entries: Vec<ArmedEntry>, today: NaiveDate) -> ArmedMeetings {
        ArmedMeetings {
            entries: entries
                .into_iter()
                .filter(|entry| entry.date >= today)
                .collect(),
        }
    }

    #[must_use]
    pub fn is_armed(&self, id: &EventId) -> bool {
        self.entries.iter().any(|entry| &entry.event_id == id)
    }

    pub fn arm(&mut self, id: EventId, today: NaiveDate) {
        if !self.is_armed(&id) {
            self.entries.push(ArmedEntry {
                event_id: id,
                date: today,
            });
        }
    }

    pub fn disarm(&mut self, id: &EventId) {
        self.entries.retain(|entry| &entry.event_id != id);
    }

    #[must_use]
    pub fn entries(&self) -> &[ArmedEntry] {
        &self.entries
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmedState {
    Armed,
    Recording,
    Recorded,
    Missed(MissedReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissedReason {
    NoCallInTime,
    StartFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArmedAction {
    StartRecording(Meeting),
    ReportMissed(Meeting),
}

#[derive(Debug, Clone, Copy)]
pub struct WatchInputs<'a> {
    pub agenda: &'a TodaysAgenda,
    pub armed: &'a ArmedMeetings,
    pub matched: Option<&'a Meeting>,
    pub in_call: bool,
    pub recording: bool,
    pub now: DateTime<FixedOffset>,
}

impl ArmedState {
    fn can_start(self) -> bool {
        matches!(
            self,
            ArmedState::Armed | ArmedState::Missed(MissedReason::NoCallInTime)
        )
    }
}

#[derive(Debug, Default)]
pub struct ArmedWatch {
    states: HashMap<EventId, ArmedState>,
    last_call_seen: Option<DateTime<FixedOffset>>,
}

impl ArmedWatch {
    #[must_use]
    pub fn new() -> ArmedWatch {
        ArmedWatch::default()
    }

    #[must_use]
    pub fn state_of(&self, id: &EventId) -> ArmedState {
        self.states.get(id).copied().unwrap_or(ArmedState::Armed)
    }

    pub fn take_due_actions(&mut self, inputs: &WatchInputs) -> Vec<ArmedAction> {
        if inputs.in_call {
            self.last_call_seen = Some(inputs.now);
        }
        let due: Vec<(ArmedAction, ArmedState)> = inputs
            .agenda
            .meetings()
            .iter()
            .filter(|meeting| inputs.armed.is_armed(&meeting.event_id))
            .filter(|meeting| inputs.now < meeting.ends_at)
            .filter_map(|meeting| self.due_action(meeting, inputs))
            .collect();
        due.into_iter()
            .map(|(action, next)| self.settle(action, next))
            .collect()
    }

    pub fn note_recording(&mut self, id: &EventId) {
        self.states.insert(id.clone(), ArmedState::Recording);
    }

    pub fn note_recorded(&mut self, id: &EventId) {
        if self.state_of(id) == ArmedState::Recording {
            self.states.insert(id.clone(), ArmedState::Recorded);
        }
    }

    pub fn note_start_failed(&mut self, id: &EventId) {
        self.states
            .insert(id.clone(), ArmedState::Missed(MissedReason::StartFailed));
    }

    pub fn forget(&mut self, id: &EventId) {
        self.states.remove(id);
    }

    fn due_action(
        &self,
        meeting: &Meeting,
        inputs: &WatchInputs,
    ) -> Option<(ArmedAction, ArmedState)> {
        let state = self.state_of(&meeting.event_id);
        if state.can_start() && is_start_due(meeting, inputs) {
            return Some((
                ArmedAction::StartRecording(meeting.clone()),
                ArmedState::Recording,
            ));
        }
        (state == ArmedState::Armed && self.is_missed(meeting, inputs)).then(|| {
            (
                ArmedAction::ReportMissed(meeting.clone()),
                ArmedState::Missed(MissedReason::NoCallInTime),
            )
        })
    }

    fn is_missed(&self, meeting: &Meeting, inputs: &WatchInputs) -> bool {
        let call_seen_since_start = self
            .last_call_seen
            .is_some_and(|seen| seen >= meeting.starts_at);
        !call_seen_since_start && inputs.now >= meeting.starts_at + as_chrono(MISSED_AFTER)
    }

    fn settle(&mut self, action: ArmedAction, next: ArmedState) -> ArmedAction {
        let (ArmedAction::StartRecording(meeting) | ArmedAction::ReportMissed(meeting)) = &action;
        self.states.insert(meeting.event_id.clone(), next);
        action
    }
}

fn is_start_due(meeting: &Meeting, inputs: &WatchInputs) -> bool {
    inputs.in_call
        && !inputs.recording
        && is_matched(meeting, inputs)
        && inputs.now >= meeting.starts_at - as_chrono(CANDIDATE_LEAD)
}

fn is_matched(meeting: &Meeting, inputs: &WatchInputs) -> bool {
    inputs
        .matched
        .is_some_and(|matched| matched.event_id == meeting.event_id)
}

fn as_chrono(duration: std::time::Duration) -> chrono::Duration {
    chrono::Duration::from_std(duration).unwrap_or_default()
}
