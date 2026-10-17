use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use scribe_core::ports::StoreError;
use scribe_core::recording::{SessionHeader, SessionSummary};

use crate::format::parse_header;
use crate::store_error::{file_name, io_error, malformed};

const DELIMITER: &str = "---";
const FRONT_MATTER_DELIMITERS: usize = 2;

pub(crate) fn list_summaries<'a>(
    paths: impl IntoIterator<Item = &'a Path>,
    limit: usize,
) -> Vec<SessionSummary> {
    let mut summaries: Vec<SessionSummary> =
        paths.into_iter().filter_map(summary_or_skip).collect();
    summaries.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    summaries.truncate(limit);
    summaries
}

pub(crate) fn read_header(path: &Path) -> Result<SessionHeader, StoreError> {
    let front_matter = read_front_matter(path)?;
    parse_header(&front_matter).map_err(|_| malformed(path))
}

fn summary_or_skip(path: &Path) -> Option<SessionSummary> {
    match read_header(path) {
        Ok(header) => Some(header.summary()),
        Err(error) => {
            tracing::warn!(file = %file_name(path), %error, "skipped a file while listing sessions");
            None
        }
    }
}

fn read_front_matter(path: &Path) -> Result<String, StoreError> {
    let file = File::open(path).map_err(|source| io_error(path, source))?;
    let mut text = String::new();
    let mut delimiters_seen = 0;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|source| io_error(path, source))?;
        delimiters_seen += usize::from(line == DELIMITER);
        text.push_str(&line);
        text.push('\n');
        if delimiters_seen == FRONT_MATTER_DELIMITERS {
            break;
        }
    }
    Ok(text)
}
