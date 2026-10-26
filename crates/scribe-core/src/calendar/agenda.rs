use super::meeting::{CalendarEvent, DayBounds, Meeting, Response, ShowAs};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TodaysAgenda(Vec<Meeting>);

impl TodaysAgenda {
    #[must_use]
    pub fn meetings(&self) -> &[Meeting] {
        &self.0
    }
}

#[must_use]
pub fn todays_agenda(events: Vec<CalendarEvent>, day: &DayBounds) -> TodaysAgenda {
    let mut meetings: Vec<Meeting> = events
        .into_iter()
        .filter(is_worth_showing)
        .map(|event| event.meeting)
        .filter(|meeting| overlaps(meeting, day))
        .collect();
    meetings.sort_by_key(|meeting| (meeting.starts_at, meeting.ends_at));
    TodaysAgenda(meetings)
}

fn is_worth_showing(event: &CalendarEvent) -> bool {
    !event.is_all_day
        && !event.is_cancelled
        && event.show_as != ShowAs::Free
        && event.meeting.response != Response::Declined
}

fn overlaps(meeting: &Meeting, day: &DayBounds) -> bool {
    meeting.starts_at < day.end && meeting.ends_at > day.start
}
