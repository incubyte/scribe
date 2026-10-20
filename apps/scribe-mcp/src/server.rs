use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{Json, ServerHandler, tool, tool_handler, tool_router};
use scribe_core::library::{LibraryError, SearchQuery, TranscriptLibrary};
use scribe_core::ports::StoreError;
use scribe_core::recording::SessionId;

use crate::requests::{
    GetScreenshotRequest, GetTranscriptRequest, ListTranscriptsRequest, SearchResults,
    SearchTranscriptsRequest, TranscriptList, TranscriptsFolderDto,
};
use crate::screenshot_listing::{MAX_SERVED_EDGE_PX, screenshots_section, served_label};
use crate::transcripts_location::TranscriptsLocation;

#[derive(Clone)]
pub struct ScribeServer {
    library: TranscriptLibrary,
    location: TranscriptsLocation,
}

impl ScribeServer {
    #[must_use]
    pub fn new(library: TranscriptLibrary, location: TranscriptsLocation) -> ScribeServer {
        ScribeServer { library, location }
    }
}

#[tool_router]
impl ScribeServer {
    #[tool(
        description = "List the call transcripts Scribe has recorded and saved on this computer, newest first. Returns id, title, start time, duration and status for each call. Calls still being recorded are not included."
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
        description = "Read one call transcript recorded by Scribe, by id, as markdown, including its header (title, start and end time, status) and every spoken line with its timestamp and speaker. When the call has screenshots, the markdown ends with a Screenshots section naming each one; pass a name to get_screenshot to see the image."
    )]
    async fn get_transcript(
        &self,
        Parameters(request): Parameters<GetTranscriptRequest>,
    ) -> Result<String, String> {
        let document = self
            .library
            .get(&request.id)
            .map_err(|error| report(&error))?;
        let mut markdown = document.raw;
        if let Some(section) = self.screenshots_section_for(&document.header.id) {
            markdown.push_str(&section);
        }
        Ok(markdown)
    }

    #[tool(
        description = "See one screenshot Scribe saved during a call, by the name listed at the end of get_transcript (written <session id>/<HH-MM-SS>.png). Returns a one-line label and the image, scaled down to at most 1568 px on its long edge."
    )]
    async fn get_screenshot(
        &self,
        Parameters(request): Parameters<GetScreenshotRequest>,
    ) -> Result<CallToolResult, String> {
        let served = self
            .library
            .screenshot(&request.name, MAX_SERVED_EDGE_PX)
            .map_err(|error| report(&error))?;
        let label = served_label(&served.reference, &served.image);
        let png = STANDARD.encode(&served.image.png);
        Ok(CallToolResult::success(vec![
            ContentBlock::text(label),
            ContentBlock::image(png, "image/png"),
        ]))
    }

    #[tool(
        description = "Search the call transcripts Scribe has recorded on this computer for a phrase, matched literally and ignoring case. Returns the newest calls that have a matching line, each with its matching lines' timestamp, speaker and text. Use get_transcript to read a whole call. Calls still being recorded are not searched."
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

    #[tool(
        description = "Return the folder on this computer where Scribe saves call transcripts, as an absolute path. Useful only to an AI app that can read local files itself, such as Claude Code; to read a call, use get_transcript instead. The user can move this folder in Scribe's settings, so ask again rather than remembering it."
    )]
    async fn get_transcripts_folder(&self) -> Result<Json<TranscriptsFolderDto>, String> {
        let folder = self
            .location
            .current()
            .map_err(|error| report_location(&error))?;
        Ok(Json(TranscriptsFolderDto::from(&folder)))
    }
}

impl ScribeServer {
    fn screenshots_section_for(&self, id: &SessionId) -> Option<String> {
        match self.library.screenshots(id) {
            Ok(references) => screenshots_section(&references),
            Err(error) => {
                tracing::warn!(%error, "could not list the screenshots of a transcript");
                None
            }
        }
    }
}

fn report(error: &LibraryError) -> String {
    tracing::warn!(%error, "a transcript request failed");
    error.to_string()
}

fn report_location(error: &StoreError) -> String {
    tracing::warn!(%error, "the transcripts folder could not be located");
    error.to_string()
}

#[tool_handler(
    name = "scribe",
    instructions = "Scribe records the user's calls and meetings on this computer and saves each one as a transcript, with the user's lines marked Me and the other side marked Them. Use these tools when the user asks about their Scribe calls, their meetings, or what someone said on a call. Transcripts stay on this computer until you read one with these tools."
)]
impl ServerHandler for ScribeServer {}
