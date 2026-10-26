use chrono::{DateTime, FixedOffset};
use scribe_core::calendar::{
    AgendaLayout, CalendarEvent, DayBounds, EventId, MIN_BLOCK_MINUTES, Meeting, Person, Response,
    ShowAs, lay_out, todays_agenda,
};

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn today() -> DayBounds {
    DayBounds {
        start: moment("2026-10-26T00:00:00+05:30"),
        end: moment("2026-10-27T00:00:00+05:30"),
    }
}

fn event(id: &str, starts_at: &str, ends_at: &str) -> CalendarEvent {
    CalendarEvent {
        meeting: Meeting {
            event_id: EventId::new(id),
            subject: format!("Meeting {id}"),
            starts_at: moment(starts_at),
            ends_at: moment(ends_at),
            organizer: Person {
                name: "Asha".to_owned(),
                email: "asha@incubyte.co".to_owned(),
            },
            attendees: Vec::new(),
            is_teams: false,
            is_recurring: false,
            response: Response::Accepted,
        },
        is_all_day: false,
        is_cancelled: false,
        show_as: ShowAs::Busy,
    }
}

fn between(id: &str, from: &str, to: &str) -> CalendarEvent {
    event(
        id,
        &format!("2026-10-26T{from}:00+05:30"),
        &format!("2026-10-26T{to}:00+05:30"),
    )
}

fn laid_out(events: Vec<CalendarEvent>) -> AgendaLayout {
    lay_out(&todays_agenda(events, &today()), &today())
}

fn placement(layout: &AgendaLayout, id: &str) -> (u8, u8) {
    let placed = layout
        .placed
        .iter()
        .find(|placed| placed.meeting.event_id.as_str() == id)
        .unwrap();
    (placed.column, placed.columns)
}

#[test]
fn a_meeting_sits_at_its_start_in_minutes_since_midnight() {
    let layout = laid_out(vec![between("review", "09:30", "10:30")]);

    assert_eq!(layout.placed[0].top_minutes, 9 * 60 + 30);
    assert_eq!(layout.placed[0].height_minutes, 60);
}

#[test]
fn a_meeting_shorter_than_the_minimum_is_drawn_at_the_minimum_height() {
    let layout = laid_out(vec![between("ping", "09:00", "09:10")]);

    assert_eq!(layout.placed[0].height_minutes, MIN_BLOCK_MINUTES);
}

#[test]
fn a_meeting_running_past_midnight_stops_at_the_end_of_the_day() {
    let layout = laid_out(vec![event(
        "deploy",
        "2026-10-26T23:00:00+05:30",
        "2026-10-27T02:00:00+05:30",
    )]);

    assert_eq!(layout.placed[0].height_minutes, 60);
}

#[test]
fn a_meeting_that_started_yesterday_is_drawn_from_midnight() {
    let layout = laid_out(vec![event(
        "overnight",
        "2026-10-25T22:00:00+05:30",
        "2026-10-26T01:00:00+05:30",
    )]);

    assert_eq!(layout.placed[0].top_minutes, 0);
    assert_eq!(layout.placed[0].height_minutes, 60);
}

#[test]
fn meetings_that_do_not_overlap_each_take_the_full_width() {
    let layout = laid_out(vec![
        between("first", "09:00", "10:00"),
        between("second", "10:00", "11:00"),
    ]);

    assert_eq!(placement(&layout, "first"), (0, 1));
    assert_eq!(placement(&layout, "second"), (0, 1));
}

#[test]
fn two_overlapping_meetings_share_the_width_in_two_columns() {
    let layout = laid_out(vec![
        between("first", "09:00", "10:00"),
        between("second", "09:30", "10:30"),
    ]);

    assert_eq!(placement(&layout, "first"), (0, 2));
    assert_eq!(placement(&layout, "second"), (1, 2));
}

fn three_meetings_at_once() -> AgendaLayout {
    laid_out(vec![
        between("first", "09:00", "10:00"),
        between("second", "09:00", "10:00"),
        between("third", "09:15", "09:45"),
    ])
}

#[test]
fn only_two_overlapping_meetings_keep_their_own_block() {
    let layout = three_meetings_at_once();

    assert_eq!(layout.placed.len(), 2);
    assert_eq!(layout.overflow.len(), 1);
}

#[test]
fn a_third_overlapping_meeting_collapses_into_a_chip_where_it_would_have_been() {
    let chip = &three_meetings_at_once().overflow[0];

    assert_eq!(chip.hidden, [EventId::new("third")]);
    assert_eq!(chip.top_minutes, 9 * 60 + 15);
    assert_eq!(chip.height_minutes, 30);
}

#[test]
fn every_meeting_beyond_the_second_is_listed_in_one_chip() {
    let layout = laid_out(vec![
        between("first", "09:00", "10:00"),
        between("second", "09:00", "10:00"),
        between("third", "09:00", "10:00"),
        between("fourth", "09:00", "10:00"),
    ]);

    assert_eq!(
        layout.overflow[0].hidden,
        [EventId::new("third"), EventId::new("fourth")]
    );
}

#[test]
fn a_short_meeting_drawn_at_the_minimum_height_counts_as_overlapping_the_next() {
    let layout = laid_out(vec![
        between("ping", "09:00", "09:05"),
        between("next", "09:10", "09:40"),
    ]);

    assert_eq!(placement(&layout, "ping"), (0, 2));
    assert_eq!(placement(&layout, "next"), (1, 2));
}

#[test]
fn separate_clusters_are_laid_out_independently() {
    let layout = laid_out(vec![
        between("a", "09:00", "10:00"),
        between("b", "09:30", "10:30"),
        between("c", "14:00", "15:00"),
    ]);

    assert_eq!(placement(&layout, "c"), (0, 1));
}

#[test]
fn an_empty_agenda_has_nothing_to_draw() {
    let layout = laid_out(Vec::new());

    assert!(layout.placed.is_empty());
    assert!(layout.overflow.is_empty());
}
