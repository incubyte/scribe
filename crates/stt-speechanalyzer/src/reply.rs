use std::time::Duration;

use scribe_core::ports::TranscribeError;
use scribe_core::recording::{TranscriptEvent, Transcription, Utterance};
use serde::Deserialize;

use crate::bridge::SessionToken;

#[derive(Deserialize)]
pub(crate) struct Reply {
    ok: bool,
    error: Option<String>,
    #[serde(default)]
    events: Vec<EventPayload>,
    finalised_through_s: Option<f64>,
    session: Option<SessionToken>,
    status: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum EventPayload {
    Partial {
        text: String,
    },
    Final {
        text: String,
        start_s: f64,
        end_s: f64,
    },
}

impl Reply {
    pub(crate) fn parse(raw: &str) -> Result<Reply, TranscribeError> {
        let reply: Reply = serde_json::from_str(raw).map_err(|_| {
            TranscribeError::Engine("unreadable reply from the speech bridge".to_owned())
        })?;
        if reply.ok {
            Ok(reply)
        } else {
            Err(TranscribeError::Engine(reply.error.unwrap_or_default()))
        }
    }

    pub(crate) fn session(self) -> Result<SessionToken, TranscribeError> {
        self.session.ok_or_else(|| missing("session"))
    }

    pub(crate) fn status(self) -> Result<String, TranscribeError> {
        self.status.ok_or_else(|| missing("status"))
    }

    pub(crate) fn transcription(self) -> Result<Transcription, TranscribeError> {
        let finalised_through = self
            .finalised_through_s
            .map(seconds)
            .ok_or_else(|| missing("finalised_through_s"))?;
        Ok(Transcription {
            events: self.events(),
            finalised_through,
        })
    }

    pub(crate) fn events(self) -> Vec<TranscriptEvent> {
        self.events
            .into_iter()
            .map(EventPayload::into_event)
            .collect()
    }
}

impl EventPayload {
    fn into_event(self) -> TranscriptEvent {
        match self {
            EventPayload::Partial { text } => TranscriptEvent::Partial(text),
            EventPayload::Final {
                text,
                start_s,
                end_s,
            } => TranscriptEvent::Final(Utterance {
                start: seconds(start_s),
                end: seconds(end_s),
                text,
            }),
        }
    }
}

fn seconds(value: f64) -> Duration {
    Duration::try_from_secs_f64(value).unwrap_or_default()
}

fn missing(field: &str) -> TranscribeError {
    TranscribeError::Engine(format!("the speech bridge reply has no {field}"))
}
