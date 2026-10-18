use std::fs::{self, File};
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::Path;

use scribe_core::ports::StoreError;
use scribe_core::recording::{
    Segment, SessionDocument, SessionHeader, SessionId, SessionSummary, Speaker, offset_from_clock,
};

use crate::format::{parse_header, split_front_matter};
use crate::store_error::{file_name, io_error, malformed};

const DELIMITER: &str = "---";
const FRONT_MATTER_DELIMITERS: usize = 2;

pub(crate) fn list_all_summaries<'a>(
    paths: impl IntoIterator<Item = &'a Path>,
) -> Vec<SessionSummary> {
    list_summaries(paths, usize::MAX)
}

pub(crate) fn list_summaries<'a>(
    paths: impl IntoIterator<Item = &'a Path>,
    limit: usize,
) -> Vec<SessionSummary> {
    let mut summaries: Vec<SessionSummary> =
        paths.into_iter().filter_map(summary_or_skip).collect();
    summaries.sort_by_key(|summary| std::cmp::Reverse(summary.started_at));
    summaries.truncate(limit);
    summaries
}

pub(crate) fn read_header(path: &Path) -> Result<SessionHeader, StoreError> {
    let front_matter = read_front_matter(path)?;
    parse_header(&front_matter).map_err(|_| malformed(path))
}

pub(crate) fn read_document(path: &Path, id: &SessionId) -> Result<SessionDocument, StoreError> {
    let raw = fs::read_to_string(path).map_err(|source| match source.kind() {
        ErrorKind::NotFound => StoreError::NotFound { id: id.clone() },
        _ => io_error(path, source),
    })?;
    let header = parse_header(&raw).map_err(|_| malformed(path))?;
    let (_, body) = split_front_matter(&raw).map_err(|_| malformed(path))?;
    let segments = body.lines().filter_map(parse_segment_line).collect();
    Ok(SessionDocument {
        header,
        segments,
        raw,
    })
}

fn parse_segment_line(line: &str) -> Option<Segment> {
    let (clock, rest) = line.strip_prefix('[')?.split_once("] **")?;
    let (label, text) = rest.split_once(":** ")?;
    let start = offset_from_clock(clock)?;
    Some(Segment {
        speaker: Speaker::from_label(label)?,
        start,
        end: start,
        text: text.to_owned(),
    })
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
