use scribe_core::calendar::{AccountIdentity, Connection};
use scribe_tray::calendar::connection_payload::ConnectionPayload;
use serde_json::{Value, json};

fn sent_to_the_webview(connection: &Connection, failure: Option<&'static str>) -> Value {
    serde_json::to_value(ConnectionPayload::new(connection, failure)).unwrap()
}

#[test]
fn a_connected_account_shows_its_name_and_email() {
    let connection = Connection::Connected(AccountIdentity {
        display_name: "Sapan Parikh".to_owned(),
        email: "sapan@incubyte.co".to_owned(),
    });

    let payload = sent_to_the_webview(&connection, None);

    assert_eq!(
        payload,
        json!({
            "state": "connected",
            "display_name": "Sapan Parikh",
            "email": "sapan@incubyte.co",
            "error": null,
        })
    );
}

#[test]
fn a_failed_sign_in_explains_why_while_signed_out() {
    let payload = sent_to_the_webview(
        &Connection::Disconnected,
        Some("the sign-in was cancelled or consent was declined"),
    );

    assert_eq!(
        payload,
        json!({
            "state": "disconnected",
            "display_name": null,
            "email": null,
            "error": "the sign-in was cancelled or consent was declined",
        })
    );
}

#[test]
fn a_connection_in_progress_and_one_needing_a_new_sign_in_are_named() {
    let connecting = sent_to_the_webview(&Connection::Connecting, None);
    let reconnect = sent_to_the_webview(&Connection::ReconnectNeeded, None);

    assert_eq!(connecting["state"], "connecting");
    assert_eq!(reconnect["state"], "reconnect_needed");
}
