#![allow(dead_code, clippy::unnecessary_wraps)]

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use calendar_graph::{HttpReply, HttpTransport, TransportError};
use scribe_core::calendar::{Secret, VaultError};
use scribe_core::ports::CredentialVault;
use url::Url;

pub const DISPLAY_NAME: &str = "Riya Shah";
const CONTENTION_WINDOW: Duration = Duration::from_millis(100);
pub const EMAIL: &str = "riya@example.com";

#[derive(Debug, Clone)]
pub struct FormRequest {
    pub url: String,
    pub form: Vec<(String, String)>,
}

impl FormRequest {
    pub fn field(&self, name: &str) -> Option<&str> {
        self.form
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetRequest {
    pub url: String,
    pub bearer: String,
}

pub struct FakeTransport {
    replies: Mutex<VecDeque<Result<HttpReply, TransportError>>>,
    requests: Mutex<Vec<FormRequest>>,
    json_replies: Mutex<VecDeque<Result<HttpReply, TransportError>>>,
    json_requests: Mutex<Vec<GetRequest>>,
}

impl FakeTransport {
    pub fn replying(replies: Vec<Result<HttpReply, TransportError>>) -> Arc<FakeTransport> {
        FakeTransport::scripted(replies, Vec::new())
    }

    pub fn scripted(
        replies: Vec<Result<HttpReply, TransportError>>,
        json_replies: Vec<Result<HttpReply, TransportError>>,
    ) -> Arc<FakeTransport> {
        Arc::new(FakeTransport {
            replies: Mutex::new(replies.into()),
            requests: Mutex::new(Vec::new()),
            json_replies: Mutex::new(json_replies.into()),
            json_requests: Mutex::new(Vec::new()),
        })
    }

    pub fn json_requests(&self) -> Vec<GetRequest> {
        self.json_requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn silent() -> Arc<FakeTransport> {
        FakeTransport::replying(Vec::new())
    }

    pub fn form_requests(&self) -> Vec<FormRequest> {
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl HttpTransport for FakeTransport {
    fn post_form(&self, url: &str, form: &[(&str, &str)]) -> Result<HttpReply, TransportError> {
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(FormRequest {
                url: url.to_owned(),
                form: form
                    .iter()
                    .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                    .collect(),
            });
        self.replies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .expect("the test scripted no reply for this request")
    }

    fn get_json(&self, url: &str, bearer: &Secret) -> Result<HttpReply, TransportError> {
        self.json_requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(GetRequest {
                url: url.to_owned(),
                bearer: bearer.expose().to_owned(),
            });
        self.json_replies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .expect("the test scripted no JSON reply for this request")
    }
}

pub struct HeldTransport {
    reply: Mutex<Option<Result<HttpReply, TransportError>>>,
    arrived: Mutex<Sender<()>>,
    released: Mutex<Receiver<()>>,
}

pub struct TransportGate {
    arrived: Receiver<()>,
    release: Sender<()>,
}

impl TransportGate {
    pub fn wait_for_request(&self) {
        self.arrived.recv().expect("the held request never arrived");
    }

    pub fn release(&self) {
        self.release
            .send(())
            .expect("the held request is no longer waiting");
    }
}

impl HeldTransport {
    pub fn holding(
        reply: Result<HttpReply, TransportError>,
    ) -> (Arc<HeldTransport>, TransportGate) {
        let (arrived_tx, arrived_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let transport = Arc::new(HeldTransport {
            reply: Mutex::new(Some(reply)),
            arrived: Mutex::new(arrived_tx),
            released: Mutex::new(release_rx),
        });
        let gate = TransportGate {
            arrived: arrived_rx,
            release: release_tx,
        };
        (transport, gate)
    }
}

impl HttpTransport for HeldTransport {
    fn post_form(&self, _url: &str, _form: &[(&str, &str)]) -> Result<HttpReply, TransportError> {
        let _ = self
            .arrived
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .send(());
        let _ = self
            .released
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .recv();
        self.reply
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .expect("the held transport answers one request")
    }

    fn get_json(&self, _url: &str, _bearer: &Secret) -> Result<HttpReply, TransportError> {
        Err(TransportError(
            "the held transport serves only tokens".to_owned(),
        ))
    }
}

pub struct FakeVault {
    stored: Mutex<Option<String>>,
    refusal: Option<VaultError>,
}

impl FakeVault {
    pub fn empty() -> Arc<FakeVault> {
        Arc::new(FakeVault {
            stored: Mutex::new(None),
            refusal: None,
        })
    }

    pub fn holding(token: &str) -> Arc<FakeVault> {
        Arc::new(FakeVault {
            stored: Mutex::new(Some(token.to_owned())),
            refusal: None,
        })
    }

    pub fn refusing_to_save(refusal: VaultError) -> Arc<FakeVault> {
        Arc::new(FakeVault {
            stored: Mutex::new(None),
            refusal: Some(refusal),
        })
    }

    pub fn stored(&self) -> Option<String> {
        self.stored
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl CredentialVault for FakeVault {
    fn load(&self) -> Result<Option<Secret>, VaultError> {
        Ok(self.stored().map(Secret::new))
    }

    fn save(&self, secret: &Secret) -> Result<(), VaultError> {
        if let Some(refusal) = &self.refusal {
            return Err(refusal.clone());
        }
        *self.stored.lock().unwrap_or_else(PoisonError::into_inner) =
            Some(secret.expose().to_owned());
        Ok(())
    }

    fn forget(&self) -> Result<(), VaultError> {
        *self.stored.lock().unwrap_or_else(PoisonError::into_inner) = None;
        Ok(())
    }
}

pub fn id_token_for(name: &str, email: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
    let claims = serde_json::json!({ "name": name, "preferred_username": email, "tid": "tenant" });
    let payload = URL_SAFE_NO_PAD.encode(claims.to_string());
    format!("{header}.{payload}.signature")
}

pub fn token_reply(
    access: &str,
    refresh: &str,
    expires_in: u64,
) -> Result<HttpReply, TransportError> {
    let body = serde_json::json!({
        "token_type": "Bearer",
        "scope": "openid profile offline_access Calendars.Read",
        "expires_in": expires_in,
        "access_token": access,
        "refresh_token": refresh,
        "id_token": id_token_for(DISPLAY_NAME, EMAIL),
    });
    Ok(HttpReply {
        status: 200,
        body: body.to_string(),
    })
}

pub fn error_reply(error: &str, description: &str) -> Result<HttpReply, TransportError> {
    let body = serde_json::json!({ "error": error, "error_description": description });
    Ok(HttpReply {
        status: 400,
        body: body.to_string(),
    })
}

pub fn unreachable() -> Result<HttpReply, TransportError> {
    Err(TransportError("dns lookup failed".to_owned()))
}

type RedirectQuery = Box<dyn Fn(&str) -> String + Send + Sync>;

pub struct ScriptedBrowser {
    redirect_query: RedirectQuery,
    opened: Mutex<Option<Url>>,
}

impl ScriptedBrowser {
    pub fn redirecting_with(
        redirect_query: impl Fn(&str) -> String + Send + Sync + 'static,
    ) -> ScriptedBrowser {
        ScriptedBrowser {
            redirect_query: Box::new(redirect_query),
            opened: Mutex::new(None),
        }
    }

    pub fn approving() -> ScriptedBrowser {
        ScriptedBrowser::redirecting_with(|state| format!("code=auth-code&state={state}"))
    }

    pub fn open(&self, authorize_url: &Url) {
        let state = query_value(authorize_url, "state");
        let redirect = Url::parse(&query_value(authorize_url, "redirect_uri")).unwrap();
        let request = format!(
            "GET /?{} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            (self.redirect_query)(&state)
        );
        *self.opened.lock().unwrap_or_else(PoisonError::into_inner) = Some(authorize_url.clone());
        thread::spawn(move || {
            let address = redirect.socket_addrs(|| None).unwrap()[0];
            let mut stream = TcpStream::connect(address).unwrap();
            stream.write_all(request.as_bytes()).unwrap();
            let mut page = String::new();
            let _ = stream.read_to_string(&mut page);
        });
    }

    pub fn opened_url(&self) -> Url {
        self.opened
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
            .expect("the browser was never opened")
    }
}

pub fn query_value(url: &Url, name: &str) -> String {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
        .unwrap_or_default()
}

pub fn started_on_thread<T: Send + 'static>(
    job: impl FnOnce() -> T + Send + 'static,
) -> JoinHandle<T> {
    let (started, has_started) = mpsc::channel();
    let handle = thread::spawn(move || {
        started
            .send(())
            .expect("the test stopped waiting for the thread");
        job()
    });
    has_started.recv().expect("the thread never started");
    handle
}

pub fn is_held_back<T>(running: &JoinHandle<T>) -> bool {
    thread::sleep(CONTENTION_WINDOW);
    !running.is_finished()
}
