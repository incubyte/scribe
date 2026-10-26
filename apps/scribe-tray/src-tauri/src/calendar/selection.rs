use std::sync::{Mutex, MutexGuard, PoisonError};

use scribe_core::calendar::{
    MatchAnchor, MatchInputs, MatchOutcome, Meeting, TodaysAgenda, match_meeting,
};
use scribe_core::recording::SessionId;

#[derive(Default)]
pub struct MeetingSelection {
    chosen: Mutex<Option<(SessionId, Meeting)>>,
}

impl MeetingSelection {
    #[must_use]
    pub fn new() -> MeetingSelection {
        MeetingSelection::default()
    }

    pub fn begin(&self, session: SessionId, meeting: Option<Meeting>) {
        *self.chosen() = meeting.map(|meeting| (session, meeting));
    }

    pub fn take_for(&self, session: &SessionId) -> Option<Meeting> {
        let mut chosen = self.chosen();
        let is_for_session = chosen
            .as_ref()
            .is_some_and(|(chosen_for, _)| chosen_for == session);
        if is_for_session {
            chosen.take().map(|(_, meeting)| meeting)
        } else {
            None
        }
    }

    fn chosen(&self) -> MutexGuard<'_, Option<(SessionId, Meeting)>> {
        self.chosen.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[must_use]
pub fn chosen_meeting(
    agenda: Option<&TodaysAgenda>,
    anchor: &MatchAnchor,
    titles: &[String],
) -> Option<Meeting> {
    let outcome = agenda.map_or(MatchOutcome::Unmatched, |agenda| {
        match_meeting(&MatchInputs {
            agenda,
            anchor: *anchor,
            titles,
        })
    });
    tracing::info!(
        r#match = outcome.kind(),
        "matched a recording to the calendar"
    );
    match outcome {
        MatchOutcome::Matched(meeting) => Some(meeting),
        MatchOutcome::Ambiguous(_) | MatchOutcome::Unmatched => None,
    }
}
