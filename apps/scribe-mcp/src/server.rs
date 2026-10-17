use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, tool, tool_router};
use scribe_core::library::{LibraryError, SearchQuery, TranscriptLibrary};

use crate::requests::{
    GetTranscriptRequest, ListTranscriptsRequest, SearchResults, SearchTranscriptsRequest,
    TranscriptList,
};

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

    #[tool(
        description = "Search the user's saved call transcripts for a phrase, matched literally and ignoring case. Returns the newest calls that have a matching line, each with its matching lines' timestamp, speaker and text. Use get_transcript to read a whole call. Calls still being recorded are not searched."
    )]
    async fn search_transcripts(
        &self,
        Parameters(request): Parameters<SearchTranscriptsRequest>,
    ) -> Result<Json<SearchResults>, String> {
        let query = SearchQuery::parse(&request.query).map_err(|error| report(&error))?;
        let hits = self
            .library
            .search(&query, request.limit())
            .map_err(|error| report(&error))?;
        Ok(Json(SearchResults::from(hits)))
    }
}

fn report(error: &LibraryError) -> String {
    tracing::warn!(%error, "a transcript request failed");
    error.to_string()
}
