use std::sync::Arc;

use rmcp::ServiceExt;
use scribe_core::library::TranscriptLibrary;
use scribe_mcp::server::ScribeServer;
use scribe_mcp::transport::bounded_stdio;
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
    let folder = TranscriptsFolder::default_location()?;
    let library = TranscriptLibrary::new(Arc::new(MarkdownTranscriptStore::new(folder)));
    ScribeServer::new(library)
        .serve(bounded_stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}
