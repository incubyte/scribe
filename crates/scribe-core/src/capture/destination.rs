use std::time::Duration;

use crate::recording::{LiveSession, SessionId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMoment {
    pub session: SessionId,
    pub offset: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    IntoSession(SessionMoment),
    ClipboardOnly,
}

impl Destination {
    #[must_use]
    pub fn at_press(live: Option<LiveSession>) -> Destination {
        live.map_or(Destination::ClipboardOnly, |session| {
            Destination::IntoSession(SessionMoment {
                offset: session.offset(),
                session: session.id().clone(),
            })
        })
    }
}
