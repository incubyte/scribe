use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use scribe_core::calendar::Secret;
use url::Url;

use crate::sign_in_error::SignInError;

const ACCEPT_POLL: Duration = Duration::from_millis(200);
const SHORTEST_READ_WAIT: Duration = Duration::from_millis(1);
const LONGEST_READ_WAIT: Duration = Duration::from_secs(2);
const MAX_REQUEST_BYTES: u64 = 16 * 1024;
const CLOSE_TAB_PAGE: &str = "<!doctype html><meta charset=\"utf-8\"><title>Scribe</title>\
<p>You can close this tab and return to Scribe.</p>";

#[derive(Debug, PartialEq, Eq)]
pub enum RedirectParams {
    Code { code: Secret, state: String },
    Refused { error: String, description: String },
}

pub struct LoopbackListener {
    listener: TcpListener,
    redirect_uri: Url,
}

impl LoopbackListener {
    pub fn bind() -> io::Result<LoopbackListener> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        let redirect_uri =
            Url::parse(&format!("http://127.0.0.1:{port}/")).map_err(io::Error::other)?;
        Ok(LoopbackListener {
            listener,
            redirect_uri,
        })
    }

    #[must_use]
    pub fn redirect_uri(&self) -> Url {
        self.redirect_uri.clone()
    }

    pub fn wait_for_redirect(self, deadline: Duration) -> Result<RedirectParams, SignInError> {
        let give_up_at = Instant::now() + deadline;
        loop {
            let stream = self.accept_before(give_up_at)?;
            if let Some(params) = self.redirect_on(stream) {
                return Ok(params);
            }
        }
    }

    fn redirect_on(&self, mut stream: TcpStream) -> Option<RedirectParams> {
        let target = read_request_target(&stream).ok()?;
        let params = self.redirect_params(&target)?;
        answer_close_tab(&mut stream);
        Some(params)
    }

    fn accept_before(&self, give_up_at: Instant) -> Result<TcpStream, SignInError> {
        self.listener
            .set_nonblocking(true)
            .map_err(|_| SignInError::Listener)?;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => return prepared(stream, give_up_at),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    pause_until_next_poll(give_up_at)?;
                }
                Err(_) => return Err(SignInError::Listener),
            }
        }
    }

    fn redirect_params(&self, target: &str) -> Option<RedirectParams> {
        params_from(&self.redirect_uri.join(target).ok()?)
    }
}

fn pause_until_next_poll(give_up_at: Instant) -> Result<(), SignInError> {
    let remaining = give_up_at.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(SignInError::TimedOut);
    }
    thread::sleep(ACCEPT_POLL.min(remaining));
    Ok(())
}

fn prepared(stream: TcpStream, give_up_at: Instant) -> Result<TcpStream, SignInError> {
    let read_wait = give_up_at
        .saturating_duration_since(Instant::now())
        .min(LONGEST_READ_WAIT)
        .max(SHORTEST_READ_WAIT);
    stream
        .set_nonblocking(false)
        .and_then(|()| stream.set_read_timeout(Some(read_wait)))
        .map_err(|_| SignInError::Listener)?;
    Ok(stream)
}

fn read_request_target(stream: &TcpStream) -> Result<String, SignInError> {
    let mut reader = BufReader::new(Read::take(stream, MAX_REQUEST_BYTES));
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|error| read_failed(&error))?;
    skip_headers(&mut reader)?;
    request_line
        .split_whitespace()
        .nth(1)
        .map(str::to_owned)
        .ok_or(SignInError::Malformed)
}

fn skip_headers(reader: &mut impl BufRead) -> Result<(), SignInError> {
    let mut header = String::new();
    loop {
        header.clear();
        let read = reader
            .read_line(&mut header)
            .map_err(|error| read_failed(&error))?;
        if read == 0 || header.trim().is_empty() {
            return Ok(());
        }
    }
}

fn read_failed(error: &io::Error) -> SignInError {
    match error.kind() {
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => SignInError::TimedOut,
        _ => SignInError::Malformed,
    }
}

fn answer_close_tab(stream: &mut TcpStream) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{CLOSE_TAB_PAGE}",
        CLOSE_TAB_PAGE.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn params_from(url: &Url) -> Option<RedirectParams> {
    let value = |name: &str| {
        url.query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };
    let state = value("state")?;
    if let Some(code) = value("code") {
        return Some(RedirectParams::Code {
            code: Secret::new(code),
            state,
        });
    }
    Some(RedirectParams::Refused {
        error: value("error")?,
        description: value("error_description").unwrap_or_default(),
    })
}
