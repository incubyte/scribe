use std::cmp::Reverse;

use chrono::{DateTime, FixedOffset};

use super::CANDIDATE_LEAD;
use super::agenda::TodaysAgenda;
use super::meeting::{Meeting, Response};
use super::titles::meeting_named_in_titles;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchAnchor(DateTime<FixedOffset>);

impl MatchAnchor {
    #[must_use]
    pub fn new(moment: DateTime<FixedOffset>) -> MatchAnchor {
        MatchAnchor(moment)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MatchInputs<'a> {
    pub agenda: &'a TodaysAgenda,
    pub anchor: MatchAnchor,
    pub titles: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchOutcome {
    Matched(Meeting),
    Ambiguous(Vec<Meeting>),
    Unmatched,
}

impl MatchOutcome {
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            MatchOutcome::Matched(_) => "matched",
            MatchOutcome::Ambiguous(_) => "ambiguous",
            MatchOutcome::Unmatched => "unmatched",
        }
    }
}

#[must_use]
pub fn meeting_candidates<'a>(agenda: &'a TodaysAgenda, anchor: &MatchAnchor) -> Vec<&'a Meeting> {
    agenda
        .meetings()
        .iter()
        .filter(|meeting| is_candidate(meeting, anchor))
        .collect()
}

#[must_use]
pub fn match_meeting(inputs: &MatchInputs) -> MatchOutcome {
    let candidates = meeting_candidates(inputs.agenda, &inputs.anchor);
    match meeting_named_in_titles(&candidates, inputs.titles) {
        Some(named) => MatchOutcome::Matched(named.clone()),
        None => rank_candidates(candidates, &inputs.anchor),
    }
}

fn rank_candidates(candidates: Vec<&Meeting>, anchor: &MatchAnchor) -> MatchOutcome {
    let mut ranked: Vec<Meeting> = candidates.into_iter().cloned().collect();
    ranked.sort_by_key(|meeting| rank(meeting, anchor));
    match ranked.as_slice() {
        [] => MatchOutcome::Unmatched,
        [only] => MatchOutcome::Matched(only.clone()),
        [best, runner_up, ..] if outranks(best, runner_up) => MatchOutcome::Matched(best.clone()),
        _ => MatchOutcome::Ambiguous(ranked),
    }
}

fn is_candidate(meeting: &Meeting, anchor: &MatchAnchor) -> bool {
    let lead = chrono::Duration::from_std(CANDIDATE_LEAD).unwrap_or_default();
    meeting.starts_at - lead <= anchor.0 && anchor.0 < meeting.ends_at
}

fn outranks(better: &Meeting, other: &Meeting) -> bool {
    (better.is_teams, answer_strength(better.response))
        > (other.is_teams, answer_strength(other.response))
}

fn rank(meeting: &Meeting, anchor: &MatchAnchor) -> (Reverse<bool>, Reverse<u8>, i64) {
    let gap = (meeting.starts_at - anchor.0).abs().num_seconds();
    (
        Reverse(meeting.is_teams),
        Reverse(answer_strength(meeting.response)),
        gap,
    )
}

fn answer_strength(response: Response) -> u8 {
    match response {
        Response::Organizer | Response::Accepted => 2,
        Response::Tentative => 1,
        Response::None | Response::Declined => 0,
    }
}
