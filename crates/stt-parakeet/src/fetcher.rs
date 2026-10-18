use std::io::{self, Read};
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use reqwest::header::{CONTENT_RANGE, RANGE};

use crate::download_error::DownloadError;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const STALL_TIMEOUT: Duration = Duration::from_secs(60);
const READ_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchOutcome {
    Resumed,
    Restarted,
}

pub trait FetchSink {
    fn begin(&mut self, outcome: FetchOutcome) -> io::Result<()>;
    fn write(&mut self, chunk: &[u8]) -> io::Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeRequest<'a> {
    pub url: &'a str,
    pub offset: u64,
}

pub trait RangeFetcher: Send {
    fn fetch_from(
        &self,
        request: &RangeRequest,
        sink: &mut dyn FetchSink,
    ) -> Result<(), DownloadError>;
}

pub struct HttpsFetcher {
    client: Client,
}

impl HttpsFetcher {
    pub fn new() -> Result<HttpsFetcher, DownloadError> {
        let client = Client::builder()
            .https_only(true)
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(STALL_TIMEOUT)
            .build()
            .map_err(network_error)?;
        Ok(HttpsFetcher { client })
    }
}

impl RangeFetcher for HttpsFetcher {
    fn fetch_from(
        &self,
        request: &RangeRequest,
        sink: &mut dyn FetchSink,
    ) -> Result<(), DownloadError> {
        let mut response = self
            .client
            .get(request.url)
            .header(RANGE, format!("bytes={}-", request.offset))
            .send()
            .map_err(network_error)?;
        sink.begin(outcome_of(&response, request.offset)?)?;
        stream_into(&mut response, sink)
    }
}

fn outcome_of(response: &Response, offset: u64) -> Result<FetchOutcome, DownloadError> {
    let content_range = response
        .headers()
        .get(CONTENT_RANGE)
        .and_then(|value| value.to_str().ok());
    classify_response(response.status(), content_range, offset)
}

fn classify_response(
    status: StatusCode,
    content_range: Option<&str>,
    requested_offset: u64,
) -> Result<FetchOutcome, DownloadError> {
    match status {
        StatusCode::PARTIAL_CONTENT => require_range_to_start_at(content_range, requested_offset),
        StatusCode::OK => Ok(FetchOutcome::Restarted),
        status => Err(DownloadError::Network(format!(
            "the server answered {status}"
        ))),
    }
}

fn require_range_to_start_at(
    content_range: Option<&str>,
    offset: u64,
) -> Result<FetchOutcome, DownloadError> {
    if content_range.and_then(range_start) == Some(offset) {
        return Ok(FetchOutcome::Resumed);
    }
    Err(DownloadError::Network(
        "the server resumed from a different position than requested".to_owned(),
    ))
}

fn range_start(content_range: &str) -> Option<u64> {
    let (unit, rest) = content_range.split_once(' ')?;
    let (start, _) = rest.split_once('-')?;
    (unit == "bytes").then(|| start.parse().ok())?
}

fn stream_into(response: &mut Response, sink: &mut dyn FetchSink) -> Result<(), DownloadError> {
    let mut buffer = vec![0_u8; READ_BUFFER_BYTES];
    loop {
        let read = response.read(&mut buffer).map_err(network_error)?;
        if read == 0 {
            return Ok(());
        }
        sink.write(&buffer[..read])?;
    }
}

fn network_error(error: impl std::error::Error) -> DownloadError {
    DownloadError::Network(error.to_string())
}

#[cfg(test)]
mod response_classification {
    use super::*;

    const RESUME_FROM: u64 = 100;

    fn is_network_failure(outcome: &Result<FetchOutcome, DownloadError>) -> bool {
        matches!(outcome, Err(DownloadError::Network(_)))
    }

    #[test]
    fn a_partial_answer_starting_where_we_asked_resumes_the_file() {
        let outcome = classify_response(
            StatusCode::PARTIAL_CONTENT,
            Some("bytes 100-199/200"),
            RESUME_FROM,
        );

        assert_eq!(outcome.unwrap(), FetchOutcome::Resumed);
    }

    #[test]
    fn a_partial_answer_starting_elsewhere_is_refused() {
        let outcome = classify_response(
            StatusCode::PARTIAL_CONTENT,
            Some("bytes 0-199/200"),
            RESUME_FROM,
        );

        assert!(is_network_failure(&outcome));
    }

    #[test]
    fn a_partial_answer_without_a_content_range_is_refused() {
        let outcome = classify_response(StatusCode::PARTIAL_CONTENT, None, RESUME_FROM);

        assert!(is_network_failure(&outcome));
    }

    #[test]
    fn a_partial_answer_in_a_unit_other_than_bytes_is_refused() {
        let outcome = classify_response(
            StatusCode::PARTIAL_CONTENT,
            Some("items 100-199/200"),
            RESUME_FROM,
        );

        assert!(is_network_failure(&outcome));
    }

    #[test]
    fn a_partial_answer_with_an_unreadable_range_is_refused() {
        let outcome = classify_response(
            StatusCode::PARTIAL_CONTENT,
            Some("bytes */200"),
            RESUME_FROM,
        );

        assert!(is_network_failure(&outcome));
    }

    #[test]
    fn a_whole_file_answer_to_a_resume_request_restarts_the_file() {
        let outcome = classify_response(StatusCode::OK, None, RESUME_FROM);

        assert_eq!(outcome.unwrap(), FetchOutcome::Restarted);
    }

    #[test]
    fn a_whole_file_answer_to_a_fresh_request_starts_the_file() {
        let outcome = classify_response(StatusCode::OK, None, 0);

        assert_eq!(outcome.unwrap(), FetchOutcome::Restarted);
    }

    #[test]
    fn a_missing_file_is_a_network_failure() {
        let outcome = classify_response(StatusCode::NOT_FOUND, None, 0);

        assert!(is_network_failure(&outcome));
    }

    #[test]
    fn an_unsatisfiable_range_is_a_network_failure() {
        let outcome = classify_response(
            StatusCode::RANGE_NOT_SATISFIABLE,
            Some("bytes */200"),
            RESUME_FROM,
        );

        assert!(is_network_failure(&outcome));
    }

    #[test]
    fn an_unfollowed_redirect_is_a_network_failure() {
        let outcome = classify_response(StatusCode::FOUND, None, 0);

        assert!(is_network_failure(&outcome));
    }
}
