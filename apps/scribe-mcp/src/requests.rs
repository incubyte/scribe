use chrono::NaiveDate;
use rmcp::schemars;
use schemars::JsonSchema;
use scribe_core::library::{
    Limit, ListQuery, MeetingSummaryView, PersonView, SearchHit, SegmentMatch, SessionSummaryView,
};
use scribe_core::recording::SessionSummary;
use serde::{Deserialize, Serialize};
use store_markdown::TranscriptsFolder;

const DATE_FORMAT: &str = "%Y-%m-%d";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTranscriptsRequest {
    #[schemars(description = "Maximum number of calls to return, from 1 to 100. Defaults to 20.")]
    pub limit: Option<u32>,
    #[schemars(
        description = "Only return calls that started on or after this date, written YYYY-MM-DD."
    )]
    pub since: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTranscriptRequest {
    #[schemars(description = "The transcript id as returned by list_transcripts.")]
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetScreenshotRequest {
    #[schemars(
        description = "A screenshot name as listed at the end of get_transcript, written <session id>/<HH-MM-SS>.png, for example 2026-10-20-143005/00-12-34.png."
    )]
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchTranscriptsRequest {
    #[schemars(
        description = "Text to find in what was said. Matched literally, ignoring case; it is never treated as a pattern."
    )]
    pub query: String,
    #[schemars(description = "Maximum number of calls to return, from 1 to 100. Defaults to 20.")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TranscriptsFolderDto {
    pub path: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TranscriptSummaryDto {
    pub id: String,
    pub title: String,
    pub started_at: String,
    pub duration_seconds: Option<u64>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting: Option<MeetingDto>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct PersonDto {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MeetingDto {
    pub subject: String,
    pub organizer: PersonDto,
    pub attendees: Vec<PersonDto>,
    pub attendee_count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TranscriptList {
    pub transcripts: Vec<TranscriptSummaryDto>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SegmentMatchDto {
    pub at: String,
    pub speaker: String,
    pub text: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SearchHitDto {
    pub id: String,
    pub title: String,
    pub started_at: String,
    pub matches: Vec<SegmentMatchDto>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SearchResults {
    pub transcripts: Vec<SearchHitDto>,
}

impl ListTranscriptsRequest {
    pub fn into_query(self) -> Result<ListQuery, String> {
        let since = self.since.as_deref().map(parse_since).transpose()?;
        Ok(ListQuery {
            limit: clamped_limit(self.limit),
            since,
        })
    }
}

impl SearchTranscriptsRequest {
    #[must_use]
    pub fn limit(&self) -> Limit {
        clamped_limit(self.limit)
    }
}

fn clamped_limit(requested: Option<u32>) -> Limit {
    requested.map_or_else(Limit::default, |count| Limit::clamped(count as usize))
}

fn parse_since(raw: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(raw, DATE_FORMAT)
        .map_err(|_| format!("Invalid since date: {raw} (expected YYYY-MM-DD)"))
}

impl From<&TranscriptsFolder> for TranscriptsFolderDto {
    fn from(folder: &TranscriptsFolder) -> TranscriptsFolderDto {
        TranscriptsFolderDto {
            path: folder.path().to_string_lossy().into_owned(),
        }
    }
}

impl From<SessionSummaryView> for TranscriptSummaryDto {
    fn from(view: SessionSummaryView) -> TranscriptSummaryDto {
        TranscriptSummaryDto {
            id: view.id,
            title: view.title,
            started_at: view.started_at,
            duration_seconds: view.duration_seconds,
            status: view.status.to_owned(),
            meeting: view.meeting.map(MeetingDto::from),
        }
    }
}

impl From<MeetingSummaryView> for MeetingDto {
    fn from(view: MeetingSummaryView) -> MeetingDto {
        MeetingDto {
            subject: view.subject,
            organizer: view.organizer.into(),
            attendees: view.attendees.into_iter().map(PersonDto::from).collect(),
            attendee_count: view.attendee_count,
        }
    }
}

impl From<PersonView> for PersonDto {
    fn from(view: PersonView) -> PersonDto {
        PersonDto {
            name: view.name,
            email: view.email,
        }
    }
}

impl From<Vec<SessionSummary>> for TranscriptList {
    fn from(summaries: Vec<SessionSummary>) -> TranscriptList {
        TranscriptList {
            transcripts: summaries
                .into_iter()
                .map(|summary| SessionSummaryView::from(summary).into())
                .collect(),
        }
    }
}

impl From<SegmentMatch> for SegmentMatchDto {
    fn from(segment_match: SegmentMatch) -> SegmentMatchDto {
        SegmentMatchDto {
            at: segment_match.at,
            speaker: segment_match.speaker.label().to_owned(),
            text: segment_match.text,
        }
    }
}

impl From<SearchHit> for SearchHitDto {
    fn from(hit: SearchHit) -> SearchHitDto {
        SearchHitDto {
            id: hit.id.to_string(),
            title: hit.title,
            started_at: hit.started_at.to_rfc3339(),
            matches: hit.matches.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Vec<SearchHit>> for SearchResults {
    fn from(hits: Vec<SearchHit>) -> SearchResults {
        SearchResults {
            transcripts: hits.into_iter().map(Into::into).collect(),
        }
    }
}
