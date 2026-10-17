use chrono::NaiveDate;

use crate::recording::SessionSummary;

const MIN_LIMIT: usize = 1;
const MAX_LIMIT: usize = 100;
const DEFAULT_LIMIT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limit(usize);

impl Limit {
    #[must_use]
    pub fn clamped(requested: usize) -> Limit {
        Limit(requested.clamp(MIN_LIMIT, MAX_LIMIT))
    }

    #[must_use]
    pub fn get(self) -> usize {
        self.0
    }
}

impl Default for Limit {
    fn default() -> Limit {
        Limit(DEFAULT_LIMIT)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ListQuery {
    pub limit: Limit,
    pub since: Option<NaiveDate>,
}

impl ListQuery {
    pub(crate) fn includes(&self, summary: &SessionSummary) -> bool {
        self.since
            .is_none_or(|since| summary.started_at.date_naive() >= since)
    }
}
