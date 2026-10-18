use crate::recording::SessionSummary;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SessionSummaryView {
    pub id: String,
    pub title: String,
    pub started_at: String,
    pub duration_seconds: Option<u64>,
    pub status: &'static str,
}

impl From<SessionSummary> for SessionSummaryView {
    fn from(summary: SessionSummary) -> SessionSummaryView {
        SessionSummaryView {
            id: summary.id.to_string(),
            title: summary.title,
            started_at: summary.started_at.to_rfc3339(),
            duration_seconds: summary.duration_seconds,
            status: summary.status.as_str(),
        }
    }
}
