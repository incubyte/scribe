use chrono::{DateTime, FixedOffset};
use scribe_core::calendar::{DayBounds, TodaysAgenda};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedAgenda {
    pub agenda: TodaysAgenda,
    pub day: DayBounds,
    pub at: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgendaSnapshot {
    pub fetched: Option<FetchedAgenda>,
    pub refresh_failed: bool,
}

impl AgendaSnapshot {
    #[must_use]
    pub fn after_fetch(fetched: FetchedAgenda) -> AgendaSnapshot {
        AgendaSnapshot {
            fetched: Some(fetched),
            refresh_failed: false,
        }
    }

    #[must_use]
    pub fn after_failure(self) -> AgendaSnapshot {
        AgendaSnapshot {
            refresh_failed: true,
            ..self
        }
    }
}
