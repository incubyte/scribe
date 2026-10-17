use chrono::NaiveDate;
use rmcp::schemars;
use schemars::JsonSchema;
use scribe_core::library::{Limit, ListQuery};
use scribe_core::recording::SessionSummary;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, JsonSchema)]
pub struct TranscriptSummaryDto {
    pub id: String,
    pub title: String,
    pub started_at: String,
    pub duration_seconds: Option<u64>,
    pub status: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TranscriptList {
    pub transcripts: Vec<TranscriptSummaryDto>,
}

impl ListTranscriptsRequest {
    pub fn into_query(self) -> Result<ListQuery, String> {
        let since = self.since.as_deref().map(parse_since).transpose()?;
        let limit = self.limit.map_or_else(Limit::default, |requested| {
            Limit::clamped(requested as usize)
        });
        Ok(ListQuery { limit, since })
    }
}

fn parse_since(raw: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(raw, DATE_FORMAT)
        .map_err(|_| format!("Invalid since date: {raw} (expected YYYY-MM-DD)"))
}

impl From<SessionSummary> for TranscriptSummaryDto {
    fn from(summary: SessionSummary) -> TranscriptSummaryDto {
        TranscriptSummaryDto {
            id: summary.id.to_string(),
            title: summary.title,
            started_at: summary.started_at.to_rfc3339(),
            duration_seconds: summary.duration_seconds,
            status: summary.status.as_str().to_owned(),
        }
    }
}

impl From<Vec<SessionSummary>> for TranscriptList {
    fn from(summaries: Vec<SessionSummary>) -> TranscriptList {
        TranscriptList {
            transcripts: summaries.into_iter().map(Into::into).collect(),
        }
    }
}
