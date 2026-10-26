use std::sync::{Mutex, MutexGuard, PoisonError};

use scribe_core::calendar::{
    EventId, MatchAnchor, MatchInputs, MatchOutcome, Meeting, TodaysAgenda, match_meeting,
    meeting_candidates,
};
use scribe_core::recording::SessionId;

const UNMATCHED: &str = "unmatched";
const MATCHED: &str = "matched";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoStart {
    Yes,
    No,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedChoice {
    pub meeting: Meeting,
    pub start: AutoStart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeetingChoice {
    Meeting(Meeting),
    NotAMeeting,
}

impl MeetingChoice {
    fn of(meeting: Option<Meeting>) -> MeetingChoice {
        meeting.map_or(MeetingChoice::NotAMeeting, MeetingChoice::Meeting)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChooseError {
    NotACandidate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallMatch {
    state: &'static str,
    candidates: Vec<Meeting>,
    choice: MeetingChoice,
}

impl CallMatch {
    #[must_use]
    pub fn settled(meeting: Option<Meeting>) -> CallMatch {
        CallMatch {
            state: UNMATCHED,
            candidates: Vec::new(),
            choice: MeetingChoice::of(meeting),
        }
    }

    #[must_use]
    pub fn decided(meeting: Meeting) -> CallMatch {
        CallMatch {
            state: MATCHED,
            candidates: vec![meeting.clone()],
            choice: MeetingChoice::Meeting(meeting),
        }
    }

    #[must_use]
    pub fn state(&self) -> &'static str {
        self.state
    }

    #[must_use]
    pub fn candidates(&self) -> &[Meeting] {
        &self.candidates
    }

    #[must_use]
    pub fn choice(&self) -> &MeetingChoice {
        &self.choice
    }

    #[must_use]
    pub fn selected(&self) -> Option<&Meeting> {
        match &self.choice {
            MeetingChoice::Meeting(meeting) => Some(meeting),
            MeetingChoice::NotAMeeting => None,
        }
    }

    pub fn choose(&mut self, id: Option<&EventId>) -> Result<(), ChooseError> {
        self.choice = match id {
            None => MeetingChoice::NotAMeeting,
            Some(id) => MeetingChoice::Meeting(self.candidate(id)?.clone()),
        };
        Ok(())
    }

    fn candidate(&self, id: &EventId) -> Result<&Meeting, ChooseError> {
        self.candidates
            .iter()
            .find(|candidate| &candidate.event_id == id)
            .ok_or(ChooseError::NotACandidate)
    }
}

#[must_use]
pub fn call_match(inputs: Option<MatchInputs>) -> CallMatch {
    let Some(inputs) = inputs else {
        return CallMatch::settled(None);
    };
    let outcome = match_meeting(&inputs);
    tracing::info!(r#match = outcome.kind(), "matched a call to the calendar");
    settle(outcome, inputs.agenda, &inputs.anchor)
}

fn settle(outcome: MatchOutcome, agenda: &TodaysAgenda, anchor: &MatchAnchor) -> CallMatch {
    let state = outcome.kind();
    match outcome {
        MatchOutcome::Matched(meeting) => CallMatch {
            state,
            candidates: candidates_with_first(agenda, anchor, &meeting),
            choice: MeetingChoice::Meeting(meeting),
        },
        MatchOutcome::Ambiguous(ranked) => CallMatch {
            state,
            choice: MeetingChoice::of(ranked.first().cloned()),
            candidates: ranked,
        },
        MatchOutcome::Unmatched => CallMatch::settled(None),
    }
}

fn candidates_with_first(
    agenda: &TodaysAgenda,
    anchor: &MatchAnchor,
    first: &Meeting,
) -> Vec<Meeting> {
    let mut candidates: Vec<Meeting> = meeting_candidates(agenda, anchor)
        .into_iter()
        .cloned()
        .collect();
    candidates.sort_by_key(|candidate| candidate.event_id != first.event_id);
    candidates
}

#[must_use]
pub fn adopt(pending: Option<CallMatch>, fallback: impl FnOnce() -> Option<Meeting>) -> CallMatch {
    pending.unwrap_or_else(|| CallMatch::settled(fallback()))
}

struct ActiveRecording {
    session: SessionId,
    found: CallMatch,
    start: AutoStart,
}

#[derive(Default)]
struct Selection {
    pending_call: Option<CallMatch>,
    prepared: Option<(CallMatch, AutoStart)>,
    recording: Option<ActiveRecording>,
}

#[derive(Default)]
pub struct MeetingSelection {
    selection: Mutex<Selection>,
}

impl MeetingSelection {
    #[must_use]
    pub fn new() -> MeetingSelection {
        MeetingSelection::default()
    }

    pub fn begin(&self, session: SessionId, meeting: Option<Meeting>) {
        self.selection().recording = Some(ActiveRecording {
            session,
            found: CallMatch::settled(meeting),
            start: AutoStart::No,
        });
    }

    pub fn prepare(&self, meeting: Meeting, start: AutoStart) {
        self.selection().prepared = Some((CallMatch::decided(meeting), start));
    }

    pub fn abandon_prepared(&self) {
        self.selection().prepared = None;
    }

    pub fn begin_from_call(&self, session: SessionId, fallback: impl FnOnce() -> Option<Meeting>) {
        let mut selection = self.selection();
        let (found, start) = match selection.prepared.take() {
            Some(prepared) => prepared,
            None => (
                adopt(selection.pending_call.clone(), fallback),
                AutoStart::No,
            ),
        };
        selection.recording = Some(ActiveRecording {
            session,
            found,
            start,
        });
    }

    pub fn take_for(&self, session: &SessionId) -> Option<FinishedChoice> {
        let mut selection = self.selection();
        let is_for_session = selection
            .recording
            .as_ref()
            .is_some_and(|recording| &recording.session == session);
        if !is_for_session {
            return None;
        }
        let finished = selection.recording.take()?;
        Some(FinishedChoice {
            meeting: finished.found.selected()?.clone(),
            start: finished.start,
        })
    }

    #[must_use]
    pub fn auto_started_session(&self) -> Option<SessionId> {
        self.selection()
            .recording
            .as_ref()
            .filter(|recording| recording.start == AutoStart::Yes)
            .map(|recording| recording.session.clone())
    }

    #[must_use]
    pub fn active_meeting(&self) -> Option<Meeting> {
        self.selection()
            .recording
            .as_ref()
            .and_then(|recording| recording.found.selected().cloned())
    }

    pub fn note_call_started(&self, found: CallMatch) {
        self.selection().pending_call = Some(found);
    }

    pub fn note_call_ended(&self) {
        self.selection().pending_call = None;
    }

    #[must_use]
    pub fn current(&self) -> Option<CallMatch> {
        let selection = self.selection();
        match &selection.recording {
            Some(recording) => Some(recording.found.clone()),
            None => selection.pending_call.clone(),
        }
    }

    pub fn choose(&self, id: Option<&EventId>) -> Result<(), ChooseError> {
        let mut selection = self.selection();
        let target = match selection.recording.as_mut() {
            Some(recording) => Some(&mut recording.found),
            None => selection.pending_call.as_mut(),
        };
        target.ok_or(ChooseError::NotACandidate)?.choose(id)
    }

    fn selection(&self) -> MutexGuard<'_, Selection> {
        self.selection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

#[must_use]
pub fn chosen_meeting(inputs: Option<MatchInputs>) -> Option<Meeting> {
    let outcome = inputs.map_or(MatchOutcome::Unmatched, |inputs| match_meeting(&inputs));
    tracing::info!(
        r#match = outcome.kind(),
        "matched a recording to the calendar"
    );
    match outcome {
        MatchOutcome::Matched(meeting) => Some(meeting),
        MatchOutcome::Ambiguous(_) | MatchOutcome::Unmatched => None,
    }
}
