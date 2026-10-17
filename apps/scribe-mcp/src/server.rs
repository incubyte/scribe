use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, tool, tool_router};
use scribe_core::library::{LibraryError, TranscriptLibrary};

use crate::requests::{GetTranscriptRequest, ListTranscriptsRequest, TranscriptList};

#[derive(Clone)]
pub struct ScribeServer {
    library: TranscriptLibrary,
}

impl ScribeServer {
    #[must_use]
    pub fn new(library: TranscriptLibrary) -> ScribeServer {
        ScribeServer { library }
    }
}

#[tool_router(server_handler)]
impl ScribeServer {
    #[tool(
        description = "List the user's saved call transcripts, newest first. Returns id, title, start time, duration and status for each call. Calls still being recorded are not included."
    )]
    async fn list_transcripts(
        &self,
        Parameters(request): Parameters<ListTranscriptsRequest>,
    ) -> Result<Json<TranscriptList>, String> {
        let query = request.into_query()?;
        let summaries = self.library.list(query).map_err(|error| report(&error))?;
        Ok(Json(TranscriptList::from(summaries)))
    }

    #[tool(
        description = "Read one saved call transcript by id as markdown, including its header (title, start and end time, status) and every spoken line with its timestamp and speaker."
    )]
    async fn get_transcript(
        &self,
        Parameters(request): Parameters<GetTranscriptRequest>,
    ) -> Result<String, String> {
        let document = self
            .library
            .get(&request.id)
            .map_err(|error| report(&error))?;
        Ok(document.raw)
    }
}

fn report(error: &LibraryError) -> String {
    tracing::warn!(%error, "a transcript request failed");
    error.to_string()
}
