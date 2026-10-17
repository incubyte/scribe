use std::future::ready;

use futures::{Sink, Stream, StreamExt};
use rmcp::RoleServer;
use rmcp::service::{RxJsonRpcMessage, TxJsonRpcMessage};
use rmcp::transport::async_rw::{JsonRpcMessageCodec, JsonRpcMessageCodecError};
use tokio::io::{AsyncRead, AsyncWrite, stdin, stdout};
use tokio_util::codec::{FramedRead, FramedWrite};

pub const MAX_MESSAGE_BYTES: usize = 1_048_576;

type Incoming = RxJsonRpcMessage<RoleServer>;
type Outgoing = TxJsonRpcMessage<RoleServer>;

pub fn bounded_stdio() -> (
    impl Sink<Outgoing, Error = JsonRpcMessageCodecError>,
    impl Stream<Item = Incoming> + Unpin,
) {
    bounded_transport(stdin(), stdout())
}

pub fn bounded_transport<R, W>(
    reader: R,
    writer: W,
) -> (
    impl Sink<Outgoing, Error = JsonRpcMessageCodecError>,
    impl Stream<Item = Incoming> + Unpin,
)
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    let codec = JsonRpcMessageCodec::<Incoming>::new_with_max_length(MAX_MESSAGE_BYTES);
    let incoming = FramedRead::new(reader, codec)
        .take_while(|frame| ready(keeps_reading(frame)))
        .filter_map(|frame| ready(frame.ok()));
    let outgoing = FramedWrite::new(writer, JsonRpcMessageCodec::<Outgoing>::default());
    (outgoing, incoming)
}

fn keeps_reading(frame: &Result<Incoming, JsonRpcMessageCodecError>) -> bool {
    match frame {
        Ok(_) => true,
        Err(error) => {
            tracing::error!(%error, "closing the connection after an unreadable incoming message");
            false
        }
    }
}
