use std::time::Duration;

use futures::StreamExt;
use scribe_mcp::transport::{MAX_MESSAGE_BYTES, bounded_transport};
use tokio::io::{AsyncWriteExt, duplex};
use tokio::time::timeout;

const DUPLEX_BUFFER_BYTES: usize = 65_536;
const WAIT: Duration = Duration::from_secs(5);

fn initialize_request(id: u32) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"test","version":"0"}}}}}}"#
    )
}

#[tokio::test]
async fn a_message_over_one_mebibyte_ends_the_connection_without_being_processed() {
    let (client_to_server, server_input) = duplex(DUPLEX_BUFFER_BYTES);
    let (server_output, _client_from_server) = duplex(DUPLEX_BUFFER_BYTES);
    let (_sink, mut incoming) = bounded_transport(server_input, server_output);
    let mut client = client_to_server;
    let oversize = "x".repeat(MAX_MESSAGE_BYTES + 1);
    let writer = tokio::spawn(async move {
        let _ = client.write_all(oversize.as_bytes()).await;
        let _ = client.write_all(b"\n").await;
        let _ = client.write_all(initialize_request(1).as_bytes()).await;
        let _ = client.write_all(b"\n").await;
        client
    });

    let next = timeout(WAIT, incoming.next()).await.unwrap();

    assert!(next.is_none());
    writer.abort();
}

#[tokio::test]
async fn a_message_within_the_limit_is_delivered() {
    let (mut client_to_server, server_input) = duplex(DUPLEX_BUFFER_BYTES);
    let (server_output, _client_from_server) = duplex(DUPLEX_BUFFER_BYTES);
    let (_sink, mut incoming) = bounded_transport(server_input, server_output);
    client_to_server
        .write_all(format!("{}\n", initialize_request(7)).as_bytes())
        .await
        .unwrap();

    let next = timeout(WAIT, incoming.next()).await.unwrap();

    assert!(next.is_some());
}
