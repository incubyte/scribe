use std::sync::Arc;

use rmcp::ServiceExt;
use scribe_core::library::TranscriptLibrary;
use scribe_core::ports::TranscriptStore;
use scribe_mcp::following_store::SettingsFollowingStore;
use scribe_mcp::server::ScribeServer;
use scribe_mcp::transport::bounded_stdio;
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder, default_settings_path};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
    let library = TranscriptLibrary::new(build_store()?);
    ScribeServer::new(library)
        .serve(bounded_stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

fn build_store() -> anyhow::Result<Arc<dyn TranscriptStore>> {
    Ok(match default_settings_path() {
        Some(settings_path) => Arc::new(SettingsFollowingStore::new(settings_path)?),
        None => Arc::new(MarkdownTranscriptStore::new(
            TranscriptsFolder::configured_by(None)?,
        )),
    })
}
