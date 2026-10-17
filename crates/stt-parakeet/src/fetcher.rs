use std::io::{self, Read};
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use reqwest::header::{CONTENT_RANGE, HeaderMap, RANGE};

use crate::download_error::DownloadError;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const WHOLE_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);
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
            .timeout(WHOLE_DOWNLOAD_TIMEOUT)
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
    match response.status() {
        StatusCode::PARTIAL_CONTENT => {
            require_range_to_start_at(response.headers(), offset)?;
            Ok(FetchOutcome::Resumed)
        }
        StatusCode::OK => Ok(FetchOutcome::Restarted),
        status => Err(DownloadError::Network(format!(
            "the server answered {status}"
        ))),
    }
}

fn require_range_to_start_at(headers: &HeaderMap, offset: u64) -> Result<(), DownloadError> {
    let start = headers
        .get(CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(range_start);
    if start == Some(offset) {
        return Ok(());
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
