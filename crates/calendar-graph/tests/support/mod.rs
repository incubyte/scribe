#![allow(dead_code, clippy::unnecessary_wraps)]

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use calendar_graph::{HttpReply, HttpTransport, TransportError};
use scribe_core::calendar::{Secret, VaultError};
use scribe_core::ports::CredentialVault;
use url::Url;

pub const DISPLAY_NAME: &str = "Sapan Parikh";
pub const EMAIL: &str = "sapan@incubyte.co";

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

pub struct FakeTransport {
    replies: Mutex<VecDeque<Result<HttpReply, TransportError>>>,
    requests: Mutex<Vec<FormRequest>>,
}

impl FakeTransport {
    pub fn replying(replies: Vec<Result<HttpReply, TransportError>>) -> Arc<FakeTransport> {
        Arc::new(FakeTransport {
            replies: Mutex::new(replies.into()),
            requests: Mutex::new(Vec::new()),
        })
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

    fn get_json(&self, _url: &str, _bearer: &Secret) -> Result<HttpReply, TransportError> {
        Err(TransportError(
            "the fake transport serves no JSON".to_owned(),
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
        "scope": "openid profile offline_access Calendars.ReadBasic",
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
