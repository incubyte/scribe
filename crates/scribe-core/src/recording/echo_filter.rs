use std::time::Duration;

use strsim::normalized_levenshtein;

use crate::recording::Segment;

pub const ECHO_OVERLAP_TOLERANCE: Duration = Duration::from_secs(1);
pub const ECHO_SIMILARITY_THRESHOLD: f64 = 0.8;
const MIN_SHARED_RUN_WORDS: usize = 3;

pub struct EchoFilter {
    overlap_tolerance: Duration,
    similarity_threshold: f64,
}

impl EchoFilter {
    #[must_use]
    pub fn new(overlap_tolerance: Duration, similarity_threshold: f64) -> EchoFilter {
        EchoFilter {
            overlap_tolerance,
            similarity_threshold,
        }
    }

    #[must_use]
    pub fn overlap_tolerance(&self) -> Duration {
        self.overlap_tolerance
    }

    #[must_use]
    pub fn is_echo(&self, mine: &Segment, theirs: &[Segment]) -> bool {
        let my_text = normalise(&mine.text);
        !my_text.is_empty()
            && theirs
                .iter()
                .filter(|other| self.overlaps(mine, other))
                .map(|other| normalise(&other.text))
                .any(|their_text| self.repeats(&my_text, &their_text))
    }

    fn overlaps(&self, a: &Segment, b: &Segment) -> bool {
        a.start <= b.end.saturating_add(self.overlap_tolerance)
            && b.start <= a.end.saturating_add(self.overlap_tolerance)
    }

    fn repeats(&self, mine: &str, theirs: &str) -> bool {
        self.is_similar(mine, theirs) || shares_contiguous_run(mine, theirs)
    }

    fn is_similar(&self, a: &str, b: &str) -> bool {
        normalized_levenshtein(a, b) >= self.similarity_threshold
    }
}

fn normalise(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn shares_contiguous_run(mine: &str, theirs: &str) -> bool {
    let my_words: Vec<&str> = mine.split(' ').collect();
    let their_words: Vec<&str> = theirs.split(' ').collect();
    their_words.windows(MIN_SHARED_RUN_WORDS).any(|run| {
        my_words
            .windows(MIN_SHARED_RUN_WORDS)
            .any(|mine| mine == run)
    })
}
