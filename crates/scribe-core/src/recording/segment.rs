use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Speaker {
    Me,
    Them,
}

impl Speaker {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Speaker::Me => "Me",
            Speaker::Them => "Them",
        }
    }

    #[must_use]
    pub fn from_label(label: &str) -> Option<Speaker> {
        [Speaker::Me, Speaker::Them]
            .into_iter()
            .find(|speaker| speaker.label() == label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utterance {
    pub start: Duration,
    pub end: Duration,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptEvent {
    Partial(String),
    Final(Utterance),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcription {
    pub events: Vec<TranscriptEvent>,
    pub finalised_through: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub speaker: Speaker,
    pub start: Duration,
    pub end: Duration,
    pub text: String,
}
