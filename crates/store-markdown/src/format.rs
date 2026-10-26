use chrono::{DateTime, FixedOffset, SecondsFormat};
use scribe_core::calendar::MeetingLink;
use scribe_core::recording::{Segment, SessionHeader, SessionId, SessionStatus, offset_as_clock};
use serde::Deserialize;
use std::fmt::Write as _;

use crate::meeting_block::{parse_meeting, render_meeting};

const DELIMITER_LINE: &str = "---\n";
const HEADER_END: &str = "\n---\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a Scribe transcript header")]
pub struct MalformedHeader;

#[must_use]
pub fn render_header(header: &SessionHeader) -> String {
    let mut text = String::from(DELIMITER_LINE);
    let _ = writeln!(text, "id: {}", header.id);
    let _ = writeln!(text, "title: {}", quoted(&header.title));
    let _ = writeln!(text, "started_at: {}", timestamp(header.started_at));
    let _ = writeln!(text, "ended_at:{}", optional_timestamp(header.ended_at));
    let _ = writeln!(text, "status: {}", header.status.as_str());
    let _ = writeln!(text, "engine: {}", header.engine);
    let _ = writeln!(text, "language: {}", header.language);
    if let Some(link) = &header.meeting {
        render_meeting(&mut text, link);
    }
    text.push_str(DELIMITER_LINE);
    text
}

#[must_use]
pub fn render_segment(segment: &Segment) -> String {
    let words: Vec<&str> = segment.text.split_whitespace().collect();
    format!(
        "[{}] **{}:** {}\n",
        offset_as_clock(segment.start),
        segment.speaker.label(),
        words.join(" ")
    )
}

pub fn parse_header(raw: &str) -> Result<SessionHeader, MalformedHeader> {
    let (front_matter, _) = split_front_matter(raw)?;
    let fields: HeaderFields = serde_saphyr::from_str(front_matter).map_err(|_| MalformedHeader)?;
    fields.into_header(parse_meeting(front_matter))
}

pub(crate) fn split_front_matter(raw: &str) -> Result<(&str, &str), MalformedHeader> {
    let rest = raw.strip_prefix(DELIMITER_LINE).ok_or(MalformedHeader)?;
    let end = rest.find(HEADER_END).ok_or(MalformedHeader)?;
    Ok((&rest[..end], &rest[end + HEADER_END.len()..]))
}

#[derive(Deserialize)]
struct HeaderFields {
    id: String,
    title: String,
    started_at: String,
    ended_at: Option<String>,
    status: String,
    engine: String,
    language: String,
}

impl HeaderFields {
    fn into_header(self, meeting: Option<MeetingLink>) -> Result<SessionHeader, MalformedHeader> {
        Ok(SessionHeader {
            id: SessionId::parse(&self.id).map_err(|_| MalformedHeader)?,
            title: self.title,
            started_at: parse_timestamp(&self.started_at)?,
            ended_at: self.ended_at.as_deref().map(parse_timestamp).transpose()?,
            status: self
                .status
                .parse::<SessionStatus>()
                .map_err(|_| MalformedHeader)?,
            engine: self.engine,
            language: self.language,
            meeting,
        })
    }
}

pub(crate) fn parse_timestamp(raw: &str) -> Result<DateTime<FixedOffset>, MalformedHeader> {
    DateTime::parse_from_rfc3339(raw).map_err(|_| MalformedHeader)
}

pub(crate) fn timestamp(moment: DateTime<FixedOffset>) -> String {
    moment.to_rfc3339_opts(SecondsFormat::Secs, false)
}

fn optional_timestamp(moment: Option<DateTime<FixedOffset>>) -> String {
    moment
        .map(|value| format!(" {}", timestamp(value)))
        .unwrap_or_default()
}

pub(crate) fn quoted(text: &str) -> String {
    let escaped: String = text.chars().map(escape_char).collect();
    format!("\"{escaped}\"")
}

fn escape_char(character: char) -> String {
    match character {
        '"' => "\\\"".to_owned(),
        '\\' => "\\\\".to_owned(),
        '\n' => "\\n".to_owned(),
        '\r' => "\\r".to_owned(),
        '\t' => "\\t".to_owned(),
        other => other.to_string(),
    }
}
