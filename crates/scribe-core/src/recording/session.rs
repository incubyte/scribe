use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, FixedOffset};

const STAMP_FORMAT: &str = "%Y-%m-%d-%H%M%S";
const TITLE_FORMAT: &str = "Call %Y-%m-%d %H:%M";
const STAMP_PART_LENGTHS: [usize; 4] = [4, 2, 2, 6];
const MAX_SUFFIX_DIGITS: usize = 3;
const MAX_COLLISION_SUFFIX: u16 = 99;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a valid session id")]
pub struct InvalidSessionId;

impl SessionId {
    pub fn parse(raw: &str) -> Result<SessionId, InvalidSessionId> {
        let parts: Vec<&str> = raw.split('-').collect();
        let (stamp, suffix) = parts.split_at(parts.len().min(STAMP_PART_LENGTHS.len()));
        if is_stamp(stamp) && is_suffix(suffix) {
            Ok(SessionId(raw.to_owned()))
        } else {
            Err(InvalidSessionId)
        }
    }

    #[must_use]
    pub fn from_start(started_at: DateTime<FixedOffset>) -> SessionId {
        SessionId(started_at.format(STAMP_FORMAT).to_string())
    }

    #[must_use]
    pub fn with_collision_suffix(&self, suffix: u16) -> SessionId {
        SessionId(format!("{}-{suffix}", self.0))
    }

    pub fn collision_candidates(&self) -> impl Iterator<Item = SessionId> + '_ {
        std::iter::once(self.clone())
            .chain((2..=MAX_COLLISION_SUFFIX).map(|suffix| self.with_collision_suffix(suffix)))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn is_stamp(parts: &[&str]) -> bool {
    parts.len() == STAMP_PART_LENGTHS.len()
        && parts
            .iter()
            .zip(STAMP_PART_LENGTHS)
            .all(|(part, length)| part.len() == length && is_digits(part))
}

fn is_suffix(parts: &[&str]) -> bool {
    match parts {
        [] => true,
        [suffix] => (1..=MAX_SUFFIX_DIGITS).contains(&suffix.len()) && is_digits(suffix),
        _ => false,
    }
}

fn is_digits(text: &str) -> bool {
    text.bytes().all(|byte| byte.is_ascii_digit())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Recording,
    Complete,
    Incomplete,
}

const ALL_STATUSES: [SessionStatus; 3] = [
    SessionStatus::Recording,
    SessionStatus::Complete,
    SessionStatus::Incomplete,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a known session status")]
pub struct UnknownSessionStatus;

impl SessionStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SessionStatus::Recording => "recording",
            SessionStatus::Complete => "complete",
            SessionStatus::Incomplete => "incomplete",
        }
    }
}

impl FromStr for SessionStatus {
    type Err = UnknownSessionStatus;

    fn from_str(raw: &str) -> Result<SessionStatus, UnknownSessionStatus> {
        ALL_STATUSES
            .into_iter()
            .find(|status| status.as_str() == raw)
            .ok_or(UnknownSessionStatus)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionHeader {
    pub id: SessionId,
    pub title: String,
    pub started_at: DateTime<FixedOffset>,
    pub ended_at: Option<DateTime<FixedOffset>>,
    pub status: SessionStatus,
    pub engine: String,
    pub language: String,
}

impl SessionHeader {
    #[must_use]
    pub fn recording(
        started_at: DateTime<FixedOffset>,
        engine: &str,
        language: &str,
    ) -> SessionHeader {
        SessionHeader {
            id: SessionId::from_start(started_at),
            title: default_title(started_at),
            started_at,
            ended_at: None,
            status: SessionStatus::Recording,
            engine: engine.to_owned(),
            language: language.to_owned(),
        }
    }

    #[must_use]
    pub fn with_id(self, id: SessionId) -> SessionHeader {
        SessionHeader { id, ..self }
    }

    #[must_use]
    pub fn finalised(self, ended_at: DateTime<FixedOffset>) -> SessionHeader {
        SessionHeader {
            ended_at: Some(ended_at),
            status: SessionStatus::Complete,
            ..self
        }
    }

    #[must_use]
    pub fn abandoned(self) -> SessionHeader {
        SessionHeader {
            status: SessionStatus::Incomplete,
            ..self
        }
    }
}

#[must_use]
pub fn default_title(started_at: DateTime<FixedOffset>) -> String {
    started_at.format(TITLE_FORMAT).to_string()
}
