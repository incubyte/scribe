use scribe_core::calendar::Meeting;
use serde::Serialize;

use super::agenda_view::clock_text;
use super::selection::{CallMatch, MeetingChoice};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CallMeetingView {
    state: &'static str,
    selected_event_id: Option<String>,
    not_a_meeting: bool,
    candidates: Vec<CandidateView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CandidateView {
    event_id: String,
    subject: String,
    starts_at: String,
    ends_at: String,
}

impl CallMeetingView {
    #[must_use]
    pub fn of(found: Option<&CallMatch>) -> CallMeetingView {
        found.map_or_else(CallMeetingView::nothing_to_choose, CallMeetingView::from)
    }

    fn nothing_to_choose() -> CallMeetingView {
        CallMeetingView::from(&CallMatch::settled(None))
    }
}

impl From<&CallMatch> for CallMeetingView {
    fn from(found: &CallMatch) -> CallMeetingView {
        CallMeetingView {
            state: found.state(),
            selected_event_id: found
                .selected()
                .map(|meeting| meeting.event_id.as_str().to_owned()),
            not_a_meeting: matches!(found.choice(), MeetingChoice::NotAMeeting),
            candidates: found.candidates().iter().map(CandidateView::from).collect(),
        }
    }
}

impl From<&Meeting> for CandidateView {
    fn from(meeting: &Meeting) -> CandidateView {
        CandidateView {
            event_id: meeting.event_id.as_str().to_owned(),
            subject: meeting.subject.clone(),
            starts_at: clock_text(meeting.starts_at),
            ends_at: clock_text(meeting.ends_at),
        }
    }
}
