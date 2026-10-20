use std::sync::Arc;

use rmcp::ServiceExt;
use scribe_core::library::TranscriptLibrary;
use scribe_mcp::following_store::SettingsFollowingStore;
use scribe_mcp::server::ScribeServer;
use scribe_mcp::transcripts_location::TranscriptsLocation;
use scribe_mcp::transport::bounded_stdio;
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder, default_settings_path};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
    let location = locate_transcripts()?;
    ScribeServer::new(build_library(&location)?, location)
        .serve(bounded_stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

fn locate_transcripts() -> anyhow::Result<TranscriptsLocation> {
    Ok(match default_settings_path() {
        Some(settings_path) => TranscriptsLocation::FollowingSettings(settings_path),
        None => TranscriptsLocation::Fixed(TranscriptsFolder::configured_by(None)?),
    })
}

fn build_library(location: &TranscriptsLocation) -> anyhow::Result<TranscriptLibrary> {
    Ok(match location {
        TranscriptsLocation::FollowingSettings(settings_path) => {
            let store = Arc::new(SettingsFollowingStore::new(settings_path.clone())?);
            TranscriptLibrary::new(store.clone(), store)
        }
        TranscriptsLocation::Fixed(folder) => {
            let store = Arc::new(MarkdownTranscriptStore::new(folder.clone()));
            TranscriptLibrary::new(store.clone(), store)
        }
    })
}
