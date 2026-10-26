use std::time::Duration;

use reqwest::blocking::{Client, Response};
use reqwest::header::CONTENT_TYPE;
use scribe_core::calendar::Secret;
use url::form_urlencoded;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const FORM_CONTENT_TYPE: &str = "application/x-www-form-urlencoded";
const PREFER_HEADER: &str = "Prefer";
const TIMES_IN_UTC: &str = "outlook.timezone=\"UTC\"";

pub trait HttpTransport: Send + Sync {
    fn post_form(&self, url: &str, form: &[(&str, &str)]) -> Result<HttpReply, TransportError>;
    fn get_json(&self, url: &str, bearer: &Secret) -> Result<HttpReply, TransportError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpReply {
    pub status: u16,
    pub body: String,
}

impl HttpReply {
    #[must_use]
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the request did not complete: {0}")]
pub struct TransportError(pub String);

pub struct ReqwestTransport {
    client: Client,
}

impl ReqwestTransport {
    pub fn new() -> Result<ReqwestTransport, TransportError> {
        let client = Client::builder()
            .https_only(true)
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(transport_error)?;
        Ok(ReqwestTransport { client })
    }
}

impl HttpTransport for ReqwestTransport {
    fn post_form(&self, url: &str, form: &[(&str, &str)]) -> Result<HttpReply, TransportError> {
        let body = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(form)
            .finish();
        let response = self
            .client
            .post(url)
            .header(CONTENT_TYPE, FORM_CONTENT_TYPE)
            .body(body)
            .send()
            .map_err(transport_error)?;
        reply_from(response)
    }

    fn get_json(&self, url: &str, bearer: &Secret) -> Result<HttpReply, TransportError> {
        let response = self
            .client
            .get(url)
            .bearer_auth(bearer.expose())
            .header(PREFER_HEADER, TIMES_IN_UTC)
            .send()
            .map_err(transport_error)?;
        reply_from(response)
    }
}

fn reply_from(response: Response) -> Result<HttpReply, TransportError> {
    let status = response.status().as_u16();
    let body = response.text().map_err(transport_error)?;
    Ok(HttpReply { status, body })
}

fn transport_error(error: reqwest::Error) -> TransportError {
    TransportError(error.without_url().to_string())
}
