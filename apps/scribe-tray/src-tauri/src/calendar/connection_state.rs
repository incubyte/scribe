use scribe_core::calendar::Connection;

use super::connection_payload::ConnectionPayload;

pub struct ConnectionView {
    pub connection: Connection,
    pub failure: Option<&'static str>,
}

impl ConnectionView {
    #[must_use]
    pub fn settled(connection: Connection) -> ConnectionView {
        ConnectionView {
            connection,
            failure: None,
        }
    }

    #[must_use]
    pub fn payload(&self) -> ConnectionPayload {
        ConnectionPayload::new(&self.connection, self.failure)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttemptTicket(u64);

pub struct ConnectionState {
    view: ConnectionView,
    disconnects: u64,
}

impl Default for ConnectionState {
    fn default() -> ConnectionState {
        ConnectionState::new()
    }
}

impl ConnectionState {
    #[must_use]
    pub fn new() -> ConnectionState {
        ConnectionState {
            view: ConnectionView::settled(Connection::Connecting),
            disconnects: 0,
        }
    }

    #[must_use]
    pub fn view(&self) -> &ConnectionView {
        &self.view
    }

    pub fn replace_view(&mut self, view: ConnectionView) {
        self.view = view;
    }

    #[must_use]
    pub fn ticket(&self) -> AttemptTicket {
        AttemptTicket(self.disconnects)
    }

    pub fn note_disconnect(&mut self) {
        self.disconnects += 1;
    }

    #[must_use]
    pub fn is_current(&self, ticket: AttemptTicket) -> bool {
        self.ticket() == ticket
    }

    #[must_use]
    pub fn is_connected_since(&self, ticket: AttemptTicket) -> bool {
        self.is_current(ticket) && matches!(self.view.connection, Connection::Connected(_))
    }
}
