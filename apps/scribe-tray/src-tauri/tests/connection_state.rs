use scribe_core::calendar::{AccountIdentity, Connection};
use scribe_tray::calendar::connection_state::{ConnectionState, ConnectionView};

fn connected() -> ConnectionView {
    ConnectionView::settled(Connection::Connected(AccountIdentity {
        display_name: "Riya Shah".to_owned(),
        email: "riya@example.com".to_owned(),
    }))
}

#[test]
fn an_outcome_begun_before_a_disconnect_no_longer_lands() {
    let mut state = ConnectionState::new();
    let ticket = state.ticket();

    state.note_disconnect();

    assert!(!state.is_current(ticket));
}

#[test]
fn an_outcome_begun_after_the_last_disconnect_still_lands() {
    let mut state = ConnectionState::new();
    state.note_disconnect();

    let ticket = state.ticket();

    assert!(state.is_current(ticket));
}

#[test]
fn a_fetch_lands_while_the_connection_it_began_on_is_still_connected() {
    let mut state = ConnectionState::new();
    state.replace_view(connected());

    let ticket = state.ticket();

    assert!(state.is_connected_since(ticket));
}

#[test]
fn a_fetch_that_returns_after_a_disconnect_is_dropped() {
    let mut state = ConnectionState::new();
    state.replace_view(connected());
    let ticket = state.ticket();

    state.note_disconnect();
    state.replace_view(ConnectionView::settled(Connection::Disconnected));

    assert!(!state.is_connected_since(ticket));
}

#[test]
fn a_fetch_that_returns_once_a_reconnect_is_needed_is_dropped() {
    let mut state = ConnectionState::new();
    state.replace_view(connected());
    let ticket = state.ticket();

    state.replace_view(ConnectionView::settled(Connection::ReconnectNeeded));

    assert!(!state.is_connected_since(ticket));
}
