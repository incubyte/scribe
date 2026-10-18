use chrono::DateTime;
use scribe_core::library::SessionSummaryView;
use scribe_core::recording::{SessionId, SessionStatus, SessionSummary};

fn summary(duration_seconds: Option<u64>, status: SessionStatus) -> SessionSummary {
    SessionSummary {
        id: SessionId::parse("2026-10-17-143005").unwrap(),
        title: "Call 2026-10-17 14:30".to_owned(),
        started_at: DateTime::parse_from_rfc3339("2026-10-17T14:30:05+05:30").unwrap(),
        duration_seconds,
        status,
    }
}

#[test]
fn a_finished_session_is_shown_with_its_start_time_length_and_status_word() {
    let view = SessionSummaryView::from(summary(Some(1_961), SessionStatus::Complete));

    assert_eq!(
        view,
        SessionSummaryView {
            id: "2026-10-17-143005".to_owned(),
            title: "Call 2026-10-17 14:30".to_owned(),
            started_at: "2026-10-17T14:30:05+05:30".to_owned(),
            duration_seconds: Some(1_961),
            status: "complete",
        }
    );
}

#[test]
fn a_session_still_recording_is_shown_without_a_length() {
    let view = SessionSummaryView::from(summary(None, SessionStatus::Recording));

    assert_eq!(view.duration_seconds, None);
    assert_eq!(view.status, "recording");
}
