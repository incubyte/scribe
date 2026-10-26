use scribe_core::calendar::{AccountIdentity, Connection};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectionPayload {
    state: &'static str,
    display_name: Option<String>,
    email: Option<String>,
    error: Option<&'static str>,
}

impl ConnectionPayload {
    #[must_use]
    pub fn new(connection: &Connection, failure: Option<&'static str>) -> ConnectionPayload {
        let identity = connected_identity(connection);
        ConnectionPayload {
            state: state_name(connection),
            display_name: identity.map(|identity| identity.display_name.clone()),
            email: identity.map(|identity| identity.email.clone()),
            error: failure,
        }
    }
}

fn connected_identity(connection: &Connection) -> Option<&AccountIdentity> {
    match connection {
        Connection::Connected(identity) => Some(identity),
        _ => None,
    }
}

fn state_name(connection: &Connection) -> &'static str {
    match connection {
        Connection::Disconnected => "disconnected",
        Connection::Connecting => "connecting",
        Connection::Connected(_) => "connected",
        Connection::ReconnectNeeded => "reconnect_needed",
    }
}
