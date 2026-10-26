use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use chrono::FixedOffset;
use scribe_core::calendar::{
    AccountIdentity, CalendarError, CalendarEvent, DayBounds, SIGN_IN_TIMEOUT, Secret, VaultError,
};
use scribe_core::ports::{CalendarSource, CredentialVault};
use url::Url;

use crate::authority::GRAPH_ORIGIN;
use crate::calendar_view::calendar_view_url;
use crate::graph_events::{GraphPage, events_from_page};
use crate::loopback::{LoopbackListener, RedirectParams};
use crate::pkce::SignInRequest;
use crate::sign_in_error::SignInError;
use crate::token_client::{CodeProof, TokenGrant, redeem_code, refresh};
use crate::transport::{HttpReply, HttpTransport};

const EARLY_REFRESH: Duration = Duration::from_secs(60);
const MAX_PAGES: usize = 5;
const UNAUTHORISED: u16 = 401;

struct CachedAccess {
    token: Secret,
    expires_at: Instant,
}

impl CachedAccess {
    fn granted(grant: &TokenGrant) -> CachedAccess {
        CachedAccess {
            token: grant.access.clone(),
            expires_at: Instant::now() + grant.expires_in,
        }
    }

    fn is_fresh(&self) -> bool {
        Instant::now() + EARLY_REFRESH < self.expires_at
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct DisconnectCount(u64);

#[derive(Default)]
struct AccountAccess {
    cached: Option<CachedAccess>,
    disconnects: DisconnectCount,
}

enum GrantFate {
    Kept,
    DroppedAfterDisconnect,
}

pub struct OutlookAccount {
    transport: Arc<dyn HttpTransport>,
    vault: Arc<dyn CredentialVault>,
    access: Mutex<AccountAccess>,
}

impl OutlookAccount {
    pub fn new(
        transport: Arc<dyn HttpTransport>,
        vault: Arc<dyn CredentialVault>,
    ) -> OutlookAccount {
        OutlookAccount {
            transport,
            vault,
            access: Mutex::new(AccountAccess::default()),
        }
    }

    pub fn connect(&self, open_browser: &dyn Fn(&Url)) -> Result<AccountIdentity, SignInError> {
        let since = self.disconnects();
        let request = SignInRequest::generate();
        let listener = LoopbackListener::bind().map_err(|_| SignInError::Listener)?;
        let redirect_uri = listener.redirect_uri();
        open_browser(&request.authorize_url(&redirect_uri));
        let code = code_from(listener.wait_for_redirect(SIGN_IN_TIMEOUT)?, &request)?;
        let proof = CodeProof {
            verifier: request.verifier(),
            redirect_uri: &redirect_uri,
        };
        let grant = redeem_code(self.transport.as_ref(), &code, &proof)?;
        match self.keep_unless_disconnected_since(since, &grant)? {
            GrantFate::Kept => Ok(grant.identity),
            GrantFate::DroppedAfterDisconnect => Err(SignInError::Disconnected),
        }
    }

    pub fn restore(&self) -> Result<Option<AccountIdentity>, CalendarError> {
        let since = self.disconnects();
        let Some(stored) = self.vault.load()? else {
            return Ok(None);
        };
        let grant = refresh(self.transport.as_ref(), &stored)?;
        match self.keep_unless_disconnected_since(since, &grant)? {
            GrantFate::Kept => Ok(Some(grant.identity)),
            GrantFate::DroppedAfterDisconnect => Ok(None),
        }
    }

    pub fn fresh_access_token(&self) -> Result<Secret, CalendarError> {
        let mut access = self.access();
        if let Some(cached) = access.cached.as_ref().filter(|cached| cached.is_fresh()) {
            return Ok(cached.token.clone());
        }
        let stored = self.vault.load()?.ok_or(CalendarError::NotConnected)?;
        let grant = refresh(self.transport.as_ref(), &stored)?;
        self.remember(&mut access, &grant)?;
        Ok(grant.access)
    }

    pub fn disconnect(&self) -> Result<(), VaultError> {
        let mut access = self.access();
        self.vault.forget()?;
        access.cached = None;
        access.disconnects.0 += 1;
        Ok(())
    }

    fn disconnects(&self) -> DisconnectCount {
        self.access().disconnects
    }

    fn keep_unless_disconnected_since(
        &self,
        since: DisconnectCount,
        grant: &TokenGrant,
    ) -> Result<GrantFate, VaultError> {
        let mut access = self.access();
        if access.disconnects != since {
            return Ok(GrantFate::DroppedAfterDisconnect);
        }
        self.remember(&mut access, grant)?;
        Ok(GrantFate::Kept)
    }

    fn remember(&self, access: &mut AccountAccess, grant: &TokenGrant) -> Result<(), VaultError> {
        self.vault.save(&grant.refresh)?;
        access.cached = Some(CachedAccess::granted(grant));
        Ok(())
    }

    fn access(&self) -> MutexGuard<'_, AccountAccess> {
        self.access.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl CalendarSource for OutlookAccount {
    fn events_between(&self, day: &DayBounds) -> Result<Vec<CalendarEvent>, CalendarError> {
        let mut events = Vec::new();
        let mut next = Some(calendar_view_url(day).to_string());
        for _ in 0..MAX_PAGES {
            let Some(url) = next else { break };
            let page = self.page_at(&url, day.start.timezone())?;
            events.extend(page.events);
            next = page.next_link;
        }
        Ok(events)
    }
}

impl OutlookAccount {
    fn page_at(&self, url: &str, offset: FixedOffset) -> Result<GraphPage, CalendarError> {
        if !url.starts_with(GRAPH_ORIGIN) {
            return Err(CalendarError::Malformed);
        }
        let reply = self.get_authorised(url)?;
        if !reply.is_success() {
            return Err(CalendarError::Unreachable);
        }
        events_from_page(&reply.body, offset)
    }

    fn get_authorised(&self, url: &str) -> Result<HttpReply, CalendarError> {
        let reply = self.get_with_current_token(url)?;
        if reply.status != UNAUTHORISED {
            return Ok(reply);
        }
        self.access().cached = None;
        self.get_with_current_token(url)
    }

    fn get_with_current_token(&self, url: &str) -> Result<HttpReply, CalendarError> {
        let token = self.fresh_access_token()?;
        self.transport
            .get_json(url, &token)
            .map_err(|_| CalendarError::Unreachable)
    }
}

fn code_from(params: RedirectParams, request: &SignInRequest) -> Result<Secret, SignInError> {
    match params {
        RedirectParams::Code { code, state } if state == request.state() => Ok(code),
        RedirectParams::Code { .. } => Err(SignInError::StateMismatch),
        RedirectParams::Refused { error, description } => {
            Err(SignInError::from_refusal(&error, &description))
        }
    }
}
