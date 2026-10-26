mod support;

use calendar_graph::{HttpReply, OutlookAccount, TransportError, calendar_view_url};
use chrono::{DateTime, FixedOffset};
use scribe_core::calendar::{CalendarError, CalendarEvent, DayBounds, Response, ShowAs};
use scribe_core::ports::CalendarSource;
use serde_json::{Value, json};
use support::{FakeTransport, FakeVault, query_value, token_reply, unreachable};

const AN_HOUR: u64 = 3600;

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn india_today() -> DayBounds {
    DayBounds {
        start: moment("2026-10-26T00:00:00+05:30"),
        end: moment("2026-10-27T00:00:00+05:30"),
    }
}

fn graph_event(id: &str, start: &str, end: &str) -> Value {
    json!({
        "id": id,
        "subject": format!("Subject {id}"),
        "start": { "dateTime": start, "timeZone": "UTC" },
        "end": { "dateTime": end, "timeZone": "UTC" },
        "organizer": { "emailAddress": { "name": "Asha Rao", "address": "asha@example.com" } },
        "attendees": [
            { "type": "required", "emailAddress": { "name": "Riya Shah", "address": "riya@example.com" } }
        ],
        "isAllDay": false,
        "isCancelled": false,
        "onlineMeetingProvider": "teamsForBusiness",
        "type": "occurrence",
        "responseStatus": { "response": "accepted" },
        "showAs": "busy"
    })
}

fn standup() -> Value {
    graph_event(
        "standup",
        "2026-10-26T03:30:00.0000000",
        "2026-10-26T03:45:00.0000000",
    )
}

#[allow(clippy::unnecessary_wraps)]
fn page(events: &[Value], next_link: Option<&str>) -> Result<HttpReply, TransportError> {
    let mut body = json!({ "value": events });
    if let Some(link) = next_link {
        body["@odata.nextLink"] = json!(link);
    }
    Ok(HttpReply {
        status: 200,
        body: body.to_string(),
    })
}

#[allow(clippy::unnecessary_wraps)]
fn status(code: u16) -> Result<HttpReply, TransportError> {
    Ok(HttpReply {
        status: code,
        body: String::new(),
    })
}

fn account_with(
    forms: Vec<Result<HttpReply, TransportError>>,
    pages: Vec<Result<HttpReply, TransportError>>,
) -> (OutlookAccount, std::sync::Arc<FakeTransport>) {
    let transport = FakeTransport::scripted(forms, pages);
    let account = OutlookAccount::new(transport.clone(), FakeVault::holding("refresh-1"));
    (account, transport)
}

fn events_of(pages: Vec<Result<HttpReply, TransportError>>) -> Vec<CalendarEvent> {
    let (account, _) = account_with(vec![token_reply("access-1", "refresh-1", AN_HOUR)], pages);
    account.events_between(&india_today()).unwrap()
}

#[test]
fn the_request_selects_only_the_meeting_fields_and_never_the_body() {
    let url = calendar_view_url(&india_today());

    assert_eq!(
        query_value(&url, "$select"),
        "id,subject,start,end,organizer,attendees,isAllDay,isCancelled,onlineMeetingProvider,type,responseStatus,showAs"
    );
}

#[test]
fn the_request_asks_for_local_midnight_to_midnight_expressed_in_utc() {
    let url = calendar_view_url(&india_today());

    assert_eq!(query_value(&url, "startDateTime"), "2026-10-25T18:30:00Z");
    assert_eq!(query_value(&url, "endDateTime"), "2026-10-26T18:30:00Z");
    assert_eq!(url.host_str(), Some("graph.microsoft.com"));
}

#[test]
fn a_graph_event_becomes_a_meeting_in_the_local_offset() {
    let events = events_of(vec![page(&[standup()], None)]);

    let meeting = &events[0].meeting;
    assert_eq!(meeting.event_id.as_str(), "standup");
    assert_eq!(meeting.subject, "Subject standup");
    assert_eq!(meeting.starts_at, moment("2026-10-26T09:00:00+05:30"));
    assert_eq!(meeting.ends_at, moment("2026-10-26T09:15:00+05:30"));
}

#[test]
fn a_teams_series_occurrence_the_user_accepted_carries_those_flags() {
    let events = events_of(vec![page(&[standup()], None)]);

    let meeting = &events[0].meeting;
    assert!(meeting.is_teams);
    assert!(meeting.is_recurring);
    assert_eq!(meeting.response, Response::Accepted);
    assert_eq!(events[0].show_as, ShowAs::Busy);
}

#[test]
fn the_organizer_and_attendees_keep_their_names_and_emails() {
    let events = events_of(vec![page(&[standup()], None)]);

    let meeting = &events[0].meeting;
    assert_eq!(meeting.organizer.email, "asha@example.com");
    assert_eq!(meeting.attendees[0].name, "Riya Shah");
}

#[test]
fn a_plain_single_event_is_neither_teams_nor_recurring() {
    let mut plain = standup();
    plain["onlineMeetingProvider"] = json!("unknown");
    plain["type"] = json!("singleInstance");

    let events = events_of(vec![page(&[plain], None)]);

    assert!(!events[0].meeting.is_teams);
    assert!(!events[0].meeting.is_recurring);
}

#[test]
fn cancelled_and_all_day_flags_and_free_time_are_carried_through() {
    let mut event = standup();
    event["isAllDay"] = json!(true);
    event["isCancelled"] = json!(true);
    event["showAs"] = json!("free");
    event["responseStatus"] = json!({ "response": "declined" });

    let events = events_of(vec![page(&[event], None)]);

    assert!(events[0].is_all_day);
    assert!(events[0].is_cancelled);
    assert_eq!(events[0].show_as, ShowAs::Free);
    assert_eq!(events[0].meeting.response, Response::Declined);
}

#[test]
fn a_malformed_event_is_skipped_and_the_rest_survive() {
    let broken = json!({ "id": "broken", "subject": "No times" });
    let bad_time = graph_event("bad-time", "yesterday", "tomorrow");

    let events = events_of(vec![page(&[broken, standup(), bad_time], None)]);

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].meeting.event_id.as_str(), "standup");
}

#[test]
fn a_reply_that_is_not_a_page_of_events_is_malformed() {
    let (account, _) = account_with(
        vec![token_reply("access-1", "refresh-1", AN_HOUR)],
        vec![Ok(HttpReply {
            status: 200,
            body: "not json".to_owned(),
        })],
    );

    let outcome = account.events_between(&india_today());

    assert_eq!(outcome, Err(CalendarError::Malformed));
}

#[test]
fn the_next_link_is_followed_until_the_last_page() {
    let next = "https://graph.microsoft.com/v1.0/me/calendarView?$skiptoken=abc";
    let review = graph_event(
        "review",
        "2026-10-26T10:00:00.0000000",
        "2026-10-26T11:00:00.0000000",
    );
    let (account, transport) = account_with(
        vec![token_reply("access-1", "refresh-1", AN_HOUR)],
        vec![page(&[standup()], Some(next)), page(&[review], None)],
    );

    let events = account.events_between(&india_today()).unwrap();

    let ids: Vec<&str> = events
        .iter()
        .map(|event| event.meeting.event_id.as_str())
        .collect();
    assert_eq!(ids, ["standup", "review"]);
    assert_eq!(transport.json_requests()[1].url, next);
}

#[test]
fn a_next_link_pointing_away_from_graph_is_refused_and_no_token_is_sent() {
    let (account, transport) = account_with(
        vec![token_reply("access-1", "refresh-1", AN_HOUR)],
        vec![page(&[standup()], Some("https://evil.example/steal"))],
    );

    let outcome = account.events_between(&india_today());

    assert_eq!(outcome, Err(CalendarError::Malformed));
    assert_eq!(transport.json_requests().len(), 1);
}

#[test]
fn paging_stops_after_five_pages() {
    let next = "https://graph.microsoft.com/v1.0/me/calendarView?$skiptoken=again";
    let endless: Vec<_> = (0..6).map(|_| page(&[], Some(next))).collect();
    let (account, transport) =
        account_with(vec![token_reply("access-1", "refresh-1", AN_HOUR)], endless);

    account.events_between(&india_today()).unwrap();

    assert_eq!(transport.json_requests().len(), 5);
}

#[test]
fn the_request_carries_the_access_token_as_bearer() {
    let (account, transport) = account_with(
        vec![token_reply("access-1", "refresh-1", AN_HOUR)],
        vec![page(&[], None)],
    );

    account.events_between(&india_today()).unwrap();

    assert_eq!(transport.json_requests()[0].bearer, "access-1");
}

#[test]
fn an_unauthorised_reply_triggers_one_token_refresh_and_a_retry() {
    let (account, transport) = account_with(
        vec![
            token_reply("access-1", "refresh-1", AN_HOUR),
            token_reply("access-2", "refresh-2", AN_HOUR),
        ],
        vec![status(401), page(&[standup()], None)],
    );

    let events = account.events_between(&india_today()).unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(transport.form_requests().len(), 2);
    let requests = transport.json_requests();
    assert_eq!(requests[0].bearer, "access-1");
    assert_eq!(requests[1].bearer, "access-2");
}

#[test]
fn a_second_unauthorised_reply_is_a_failure_not_another_retry() {
    let (account, transport) = account_with(
        vec![
            token_reply("access-1", "refresh-1", AN_HOUR),
            token_reply("access-2", "refresh-2", AN_HOUR),
        ],
        vec![status(401), status(401)],
    );

    let outcome = account.events_between(&india_today());

    assert_eq!(outcome, Err(CalendarError::Unreachable));
    assert_eq!(transport.json_requests().len(), 2);
}

#[test]
fn a_server_error_means_the_calendar_could_not_be_reached() {
    let (account, _) = account_with(
        vec![token_reply("access-1", "refresh-1", AN_HOUR)],
        vec![status(503)],
    );

    assert_eq!(
        account.events_between(&india_today()),
        Err(CalendarError::Unreachable)
    );
}

#[test]
fn a_network_failure_means_the_calendar_could_not_be_reached() {
    let (account, _) = account_with(
        vec![token_reply("access-1", "refresh-1", AN_HOUR)],
        vec![unreachable()],
    );

    assert_eq!(
        account.events_between(&india_today()),
        Err(CalendarError::Unreachable)
    );
}

#[test]
fn asking_without_a_stored_account_says_not_connected() {
    let transport = FakeTransport::silent();
    let account = OutlookAccount::new(transport, FakeVault::empty());

    assert_eq!(
        account.events_between(&india_today()),
        Err(CalendarError::NotConnected)
    );
}
