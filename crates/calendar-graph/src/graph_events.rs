use chrono::{DateTime, FixedOffset, NaiveDateTime};
use scribe_core::calendar::{
    CalendarError, CalendarEvent, EventId, Meeting, Person, Response, ShowAs,
};
use serde::Deserialize;
use serde_json::Value;

const GRAPH_TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.f";
const TEAMS_PROVIDER: &str = "teamsForBusiness";
const SERIES_MEMBER_TYPES: [&str; 2] = ["occurrence", "exception"];
const NO_SUBJECT: &str = "(No subject)";

pub struct GraphPage {
    pub events: Vec<CalendarEvent>,
    pub next_link: Option<String>,
}

#[derive(Deserialize)]
struct PageDto {
    value: Vec<Value>,
    #[serde(rename = "@odata.nextLink")]
    next_link: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventDto {
    id: String,
    subject: Option<String>,
    start: MomentDto,
    end: MomentDto,
    organizer: Option<RecipientDto>,
    #[serde(default)]
    attendees: Vec<RecipientDto>,
    #[serde(default)]
    is_all_day: bool,
    #[serde(default)]
    is_cancelled: bool,
    online_meeting_provider: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    response_status: Option<ResponseStatusDto>,
    show_as: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MomentDto {
    date_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecipientDto {
    email_address: AddressDto,
}

#[derive(Deserialize)]
struct AddressDto {
    name: Option<String>,
    address: Option<String>,
}

#[derive(Deserialize)]
struct ResponseStatusDto {
    response: Option<String>,
}

pub fn events_from_page(body: &str, offset: FixedOffset) -> Result<GraphPage, CalendarError> {
    let page: PageDto = serde_json::from_str(body).map_err(|_| CalendarError::Malformed)?;
    let events = page
        .value
        .into_iter()
        .filter_map(|value| readable_event(value, offset))
        .collect();
    Ok(GraphPage {
        events,
        next_link: page.next_link,
    })
}

fn readable_event(value: Value, offset: FixedOffset) -> Option<CalendarEvent> {
    let event = serde_json::from_value::<EventDto>(value)
        .ok()
        .and_then(|dto| dto.into_event(offset));
    if event.is_none() {
        tracing::warn!(
            kind = "malformed_event",
            "skipped an unreadable calendar event"
        );
    }
    event
}

impl EventDto {
    fn into_event(self, offset: FixedOffset) -> Option<CalendarEvent> {
        let meeting = Meeting {
            starts_at: local_moment(&self.start, offset)?,
            ends_at: local_moment(&self.end, offset)?,
            event_id: EventId::new(self.id),
            subject: self.subject.unwrap_or_else(|| NO_SUBJECT.to_owned()),
            organizer: self.organizer.map(Person::from).unwrap_or_default(),
            attendees: self.attendees.into_iter().map(Person::from).collect(),
            is_teams: self.online_meeting_provider.as_deref() == Some(TEAMS_PROVIDER),
            is_recurring: self
                .kind
                .as_deref()
                .is_some_and(|kind| SERIES_MEMBER_TYPES.contains(&kind)),
            response: response_from(self.response_status),
        };
        Some(CalendarEvent {
            meeting,
            is_all_day: self.is_all_day,
            is_cancelled: self.is_cancelled,
            show_as: show_as_from(self.show_as.as_deref()),
        })
    }
}

impl From<RecipientDto> for Person {
    fn from(recipient: RecipientDto) -> Person {
        Person {
            name: recipient.email_address.name.unwrap_or_default(),
            email: recipient.email_address.address.unwrap_or_default(),
        }
    }
}

fn local_moment(moment: &MomentDto, offset: FixedOffset) -> Option<DateTime<FixedOffset>> {
    NaiveDateTime::parse_from_str(&moment.date_time, GRAPH_TIME_FORMAT)
        .ok()
        .map(|utc| utc.and_utc().with_timezone(&offset))
}

fn response_from(status: Option<ResponseStatusDto>) -> Response {
    match status.and_then(|status| status.response).as_deref() {
        Some("organizer") => Response::Organizer,
        Some("accepted") => Response::Accepted,
        Some("tentativelyAccepted") => Response::Tentative,
        Some("declined") => Response::Declined,
        _ => Response::None,
    }
}

fn show_as_from(show_as: Option<&str>) -> ShowAs {
    match show_as {
        Some("free") => ShowAs::Free,
        Some("tentative") => ShowAs::Tentative,
        Some("busy") => ShowAs::Busy,
        Some("oof") => ShowAs::Away,
        Some("workingElsewhere") => ShowAs::WorkingElsewhere,
        _ => ShowAs::Unknown,
    }
}
