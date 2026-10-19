use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::{RoleClient, ServiceExt};
use scribe_core::library::TranscriptLibrary;
use scribe_core::ports::TranscriptStore;
use scribe_core::recording::{Segment, SessionHeader, SessionId, Speaker};
use scribe_mcp::following_store::SettingsFollowingStore;
use scribe_mcp::server::ScribeServer;
use serde_json::{Value, json};
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder};

const DUPLEX_BUFFER_BYTES: usize = 65_536;

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn store_in(folder: &Path) -> MarkdownTranscriptStore {
    MarkdownTranscriptStore::new(TranscriptsFolder::at(folder.to_path_buf()))
}

fn start_call(store: &MarkdownTranscriptStore, started_at: &str) -> SessionId {
    let header = SessionHeader::recording(moment(started_at), "speechanalyzer", "en-US");
    store.create(&header).unwrap()
}

fn save_call(store: &MarkdownTranscriptStore, started_at: &str) -> SessionId {
    let id = start_call(store, started_at);
    store.finalise(&id, moment(started_at)).unwrap();
    id
}

fn line(speaker: Speaker, text: &str) -> Segment {
    Segment {
        speaker,
        start: Duration::from_secs(4),
        end: Duration::from_secs(6),
        text: text.to_owned(),
    }
}

async fn connect_to(folder: &Path) -> RunningService<RoleClient, ()> {
    let library = TranscriptLibrary::new(Arc::new(store_in(folder)));
    let (server_side, client_side) = tokio::io::duplex(DUPLEX_BUFFER_BYTES);
    tokio::spawn(async move {
        ScribeServer::new(library)
            .serve(server_side)
            .await?
            .waiting()
            .await?;
        anyhow::Ok(())
    });
    ().serve(client_side).await.unwrap()
}

async fn connect_following(settings_path: &Path) -> RunningService<RoleClient, ()> {
    let store = SettingsFollowingStore::new(settings_path.to_path_buf()).unwrap();
    let library = TranscriptLibrary::new(Arc::new(store));
    let (server_side, client_side) = tokio::io::duplex(DUPLEX_BUFFER_BYTES);
    tokio::spawn(async move {
        ScribeServer::new(library)
            .serve(server_side)
            .await?
            .waiting()
            .await?;
        anyhow::Ok(())
    });
    ().serve(client_side).await.unwrap()
}

fn point_settings_at(settings_path: &Path, folder: &Path) {
    fs::write(
        settings_path,
        json!({ "transcripts_folder": folder.to_str().unwrap() }).to_string(),
    )
    .unwrap();
}

async fn call(
    client: &RunningService<RoleClient, ()>,
    tool: &'static str,
    arguments: Value,
) -> Value {
    let request =
        CallToolRequestParams::new(tool).with_arguments(arguments.as_object().unwrap().clone());
    serde_json::to_value(client.call_tool(request).await.unwrap()).unwrap()
}

fn listed_ids(result: &Value) -> Vec<&str> {
    result["structuredContent"]["transcripts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|transcript| transcript["id"].as_str().unwrap())
        .collect()
}

fn text_of(result: &Value) -> &str {
    result["content"][0]["text"].as_str().unwrap()
}

fn is_tool_error(result: &Value) -> bool {
    result["isError"] == json!(true)
}

fn folder_contents(folder: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    collect_files(folder, folder, &mut files);
    files.sort();
    files
}

fn collect_files(root: &Path, folder: &Path, files: &mut Vec<(String, Vec<u8>)>) {
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_files(root, &path, files);
        } else {
            let relative = path.strip_prefix(root).unwrap();
            files.push((
                relative.to_string_lossy().into_owned(),
                fs::read(&path).unwrap(),
            ));
        }
    }
}

#[tokio::test]
async fn an_ai_app_discovers_the_list_and_read_tools_with_descriptions_and_input_schemas() {
    let folder = tempfile::tempdir().unwrap();
    let client = connect_to(folder.path()).await;

    let mut tools = client.list_all_tools().await.unwrap();
    tools.sort_by(|a, b| a.name.cmp(&b.name));

    let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
    assert_eq!(
        names,
        ["get_transcript", "list_transcripts", "search_transcripts"]
    );
    assert!(tools.iter().all(|tool| tool.description.is_some()));
    assert!(tools[0].input_schema["properties"]["id"].is_object());
    assert!(tools[1].input_schema["properties"]["limit"].is_object());
    assert!(tools[1].input_schema["properties"]["since"].is_object());
    assert!(tools[2].input_schema["properties"]["query"].is_object());
    assert!(tools[2].input_schema["properties"]["limit"].is_object());
}

#[tokio::test]
async fn every_tool_says_it_reads_scribe_transcripts_so_ai_apps_can_tell_it_apart() {
    let folder = tempfile::tempdir().unwrap();
    let client = connect_to(folder.path()).await;

    let tools = client.list_all_tools().await.unwrap();

    assert!(tools.iter().all(|tool| {
        tool.description
            .as_deref()
            .is_some_and(|text| text.contains("Scribe"))
    }));
}

#[tokio::test]
async fn the_server_introduces_itself_as_scribe_with_instructions_for_the_ai_app() {
    let folder = tempfile::tempdir().unwrap();
    let client = connect_to(folder.path()).await;

    let info = client.peer_info().unwrap();

    assert_eq!(
        info.server_info.as_ref().map(|server| server.name.as_str()),
        Some("scribe")
    );
    assert!(
        info.instructions
            .as_deref()
            .is_some_and(|text| text.contains("Scribe"))
    );
}

#[tokio::test]
async fn an_ai_app_sees_saved_calls_newest_first() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    save_call(&store, "2026-10-15T09:00:00+00:00");
    save_call(&store, "2026-10-17T09:00:00+00:00");
    save_call(&store, "2026-10-16T09:00:00+00:00");
    let client = connect_to(folder.path()).await;

    let result = call(&client, "list_transcripts", json!({})).await;

    assert_eq!(
        listed_ids(&result),
        [
            "2026-10-17-090000",
            "2026-10-16-090000",
            "2026-10-15-090000"
        ]
    );
}

#[tokio::test]
async fn each_listed_call_carries_its_title_and_status() {
    let folder = tempfile::tempdir().unwrap();
    save_call(&store_in(folder.path()), "2026-10-17T09:00:00+00:00");
    let client = connect_to(folder.path()).await;

    let result = call(&client, "list_transcripts", json!({})).await;

    let listed = &result["structuredContent"]["transcripts"][0];
    assert_eq!(listed["title"], "Call 2026-10-17 09:00");
    assert_eq!(listed["status"], "complete");
}

#[tokio::test]
async fn an_ai_app_can_limit_how_many_calls_it_sees() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    save_call(&store, "2026-10-15T09:00:00+00:00");
    save_call(&store, "2026-10-16T09:00:00+00:00");
    save_call(&store, "2026-10-17T09:00:00+00:00");
    let client = connect_to(folder.path()).await;

    let result = call(&client, "list_transcripts", json!({ "limit": 2 })).await;

    assert_eq!(
        listed_ids(&result),
        ["2026-10-17-090000", "2026-10-16-090000"]
    );
}

#[tokio::test]
async fn an_ai_app_can_list_only_calls_since_a_date() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    save_call(&store, "2026-10-14T09:00:00+00:00");
    save_call(&store, "2026-10-16T09:00:00+00:00");
    let client = connect_to(folder.path()).await;

    let result = call(
        &client,
        "list_transcripts",
        json!({ "since": "2026-10-15" }),
    )
    .await;

    assert_eq!(listed_ids(&result), ["2026-10-16-090000"]);
}

#[tokio::test]
async fn a_since_value_that_is_not_a_date_is_reported_as_a_tool_error() {
    let folder = tempfile::tempdir().unwrap();
    let client = connect_to(folder.path()).await;

    let result = call(
        &client,
        "list_transcripts",
        json!({ "since": "last tuesday" }),
    )
    .await;

    assert!(is_tool_error(&result));
    assert_eq!(
        text_of(&result),
        "Invalid since date: last tuesday (expected YYYY-MM-DD)"
    );
}

#[tokio::test]
async fn a_call_still_being_recorded_is_not_listed() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    save_call(&store, "2026-10-15T09:00:00+00:00");
    start_call(&store, "2026-10-16T09:00:00+00:00");
    let client = connect_to(folder.path()).await;

    let result = call(&client, "list_transcripts", json!({})).await;

    assert_eq!(listed_ids(&result), ["2026-10-15-090000"]);
}

#[tokio::test]
async fn an_ai_app_reads_a_full_transcript_including_its_header() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    let id = start_call(&store, "2026-10-15T09:00:00+00:00");
    store
        .append(&id, &line(Speaker::Them, "Welcome to the review"))
        .unwrap();
    store
        .finalise(&id, moment("2026-10-15T09:30:00+00:00"))
        .unwrap();
    let client = connect_to(folder.path()).await;

    let result = call(&client, "get_transcript", json!({ "id": id.as_str() })).await;

    assert!(!is_tool_error(&result));
    assert_eq!(
        text_of(&result),
        fs::read_to_string(store.path_for(&id)).unwrap()
    );
    assert!(text_of(&result).contains("status: complete"));
    assert!(text_of(&result).contains("[00:00:04] **Them:** Welcome to the review"));
}

#[tokio::test]
async fn an_unknown_transcript_returns_not_found() {
    let folder = tempfile::tempdir().unwrap();
    let client = connect_to(folder.path()).await;

    let result = call(
        &client,
        "get_transcript",
        json!({ "id": "2026-10-15-090000" }),
    )
    .await;

    assert!(is_tool_error(&result));
    assert_eq!(text_of(&result), "Transcript not found: 2026-10-15-090000");
}

#[tokio::test]
async fn a_call_still_being_recorded_cannot_be_read() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    let id = start_call(&store, "2026-10-15T09:00:00+00:00");
    let client = connect_to(folder.path()).await;

    let result = call(&client, "get_transcript", json!({ "id": id.as_str() })).await;

    assert!(is_tool_error(&result));
    assert_eq!(text_of(&result), "Transcript not found: 2026-10-15-090000");
}

#[tokio::test]
async fn an_id_that_tries_to_leave_the_transcripts_folder_is_rejected() {
    let parent = tempfile::tempdir().unwrap();
    fs::write(parent.path().join("secret.md"), "top secret").unwrap();
    let folder = parent.path().join("Scribe");
    fs::create_dir(&folder).unwrap();
    let client = connect_to(&folder).await;

    let result = call(&client, "get_transcript", json!({ "id": "../secret" })).await;

    assert!(is_tool_error(&result));
    assert_eq!(text_of(&result), "Invalid transcript id: ../secret");
}

#[tokio::test]
async fn a_corrupt_transcript_file_is_skipped_rather_than_crashing_the_server() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    let good = save_call(&store, "2026-10-15T09:00:00+00:00");
    fs::write(folder.path().join("2026-10-16-090000.md"), "garbage").unwrap();
    let client = connect_to(folder.path()).await;

    let listing = call(&client, "list_transcripts", json!({})).await;
    let corrupt_read = call(
        &client,
        "get_transcript",
        json!({ "id": "2026-10-16-090000" }),
    )
    .await;
    let good_read = call(&client, "get_transcript", json!({ "id": good.as_str() })).await;

    assert_eq!(listed_ids(&listing), ["2026-10-15-090000"]);
    assert!(is_tool_error(&corrupt_read));
    assert!(!is_tool_error(&good_read));
}

fn save_call_saying(
    store: &MarkdownTranscriptStore,
    started_at: &str,
    lines: &[Segment],
) -> SessionId {
    let id = start_call(store, started_at);
    for spoken in lines {
        store.append(&id, spoken).unwrap();
    }
    store.finalise(&id, moment(started_at)).unwrap();
    id
}

fn matched_texts(result: &Value) -> Vec<&str> {
    result["structuredContent"]["transcripts"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|hit| hit["matches"].as_array().unwrap())
        .map(|hit_match| hit_match["text"].as_str().unwrap())
        .collect()
}

fn save_quarterly_review(store: &MarkdownTranscriptStore) -> SessionId {
    save_call_saying(
        store,
        "2026-10-15T09:00:00+00:00",
        &[
            line(Speaker::Them, "Let's review the QUARTERLY numbers"),
            line(Speaker::Me, "Sounds good"),
        ],
    )
}

#[tokio::test]
async fn an_ai_app_finds_the_lines_that_mention_a_phrase_regardless_of_case() {
    let folder = tempfile::tempdir().unwrap();
    save_quarterly_review(&store_in(folder.path()));
    let client = connect_to(folder.path()).await;

    let result = call(
        &client,
        "search_transcripts",
        json!({ "query": "quarterly" }),
    )
    .await;

    assert!(!is_tool_error(&result));
    assert_eq!(listed_ids(&result), ["2026-10-15-090000"]);
    assert_eq!(
        result["structuredContent"]["transcripts"][0]["matches"],
        json!([{ "at": "00:00:04", "speaker": "Them", "text": "Let's review the QUARTERLY numbers" }])
    );
}

#[tokio::test]
async fn a_query_full_of_regex_syntax_is_matched_as_plain_text() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    save_call_saying(
        &store,
        "2026-10-15T09:00:00+00:00",
        &[
            line(Speaker::Me, "We spent 5.* more than planned"),
            line(Speaker::Them, "Nothing special here"),
        ],
    );
    let client = connect_to(folder.path()).await;

    let result = call(&client, "search_transcripts", json!({ "query": ".*" })).await;

    assert!(!is_tool_error(&result));
    assert_eq!(matched_texts(&result), ["We spent 5.* more than planned"]);
}

#[tokio::test]
async fn a_blank_search_is_reported_as_a_tool_error() {
    let folder = tempfile::tempdir().unwrap();
    let client = connect_to(folder.path()).await;

    let result = call(&client, "search_transcripts", json!({ "query": "   " })).await;

    assert!(is_tool_error(&result));
    assert_eq!(text_of(&result), "Search query is empty");
}

#[tokio::test]
async fn a_call_still_being_recorded_is_not_searched() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    save_call_saying(
        &store,
        "2026-10-15T09:00:00+00:00",
        &[line(Speaker::Them, "budget approved")],
    );
    let live = start_call(&store, "2026-10-16T09:00:00+00:00");
    store
        .append(&live, &line(Speaker::Them, "budget approved"))
        .unwrap();
    let client = connect_to(folder.path()).await;

    let result = call(&client, "search_transcripts", json!({ "query": "budget" })).await;

    assert_eq!(listed_ids(&result), ["2026-10-15-090000"]);
}

fn seed_mixed_folder(folder: &Path) -> SessionId {
    let store = store_in(folder);
    let id = start_call(&store, "2026-10-15T09:00:00+00:00");
    store
        .append(&id, &line(Speaker::Me, "Before we start"))
        .unwrap();
    store
        .finalise(&id, moment("2026-10-15T09:30:00+00:00"))
        .unwrap();
    start_call(&store, "2026-10-16T09:00:00+00:00");
    fs::write(folder.join("2026-10-17-090000.md"), "garbage").unwrap();
    id
}

async fn list_calls_every_way(client: &RunningService<RoleClient, ()>) {
    call(client, "list_transcripts", json!({})).await;
    call(
        client,
        "list_transcripts",
        json!({ "since": "2026-10-01", "limit": 5 }),
    )
    .await;
}

async fn read_calls_every_way(client: &RunningService<RoleClient, ()>, id: &SessionId) {
    for read in [id.as_str(), "2026-10-16-090000", "../x"] {
        call(client, "get_transcript", json!({ "id": read })).await;
    }
}

async fn use_every_tool_every_way(client: &RunningService<RoleClient, ()>, id: &SessionId) {
    list_calls_every_way(client).await;
    read_calls_every_way(client, id).await;
    call(client, "search_transcripts", json!({ "query": "before" })).await;
}

#[tokio::test]
async fn the_transcripts_folder_is_byte_identical_after_a_full_session_of_tool_calls() {
    let folder = tempfile::tempdir().unwrap();
    let id = seed_mixed_folder(folder.path());
    let before = folder_contents(folder.path());
    let client = connect_to(folder.path()).await;

    use_every_tool_every_way(&client, &id).await;

    assert_eq!(folder_contents(folder.path()), before);
}

#[tokio::test]
async fn the_mcp_server_follows_a_folder_change_without_restarting() {
    let settings = tempfile::tempdir().unwrap();
    let settings_path = settings.path().join("settings.json");
    let folder_a = tempfile::tempdir().unwrap();
    let folder_b = tempfile::tempdir().unwrap();
    let id_in_a = save_call(&store_in(folder_a.path()), "2026-10-19T10:00:00+00:00");
    let id_in_b = save_call(&store_in(folder_b.path()), "2026-10-20T10:00:00+00:00");
    point_settings_at(&settings_path, folder_a.path());
    let client = connect_following(&settings_path).await;

    let before = call(&client, "list_transcripts", json!({})).await;
    point_settings_at(&settings_path, folder_b.path());
    let after = call(&client, "list_transcripts", json!({})).await;

    assert_eq!(listed_ids(&before), [id_in_a.to_string()]);
    assert_eq!(listed_ids(&after), [id_in_b.to_string()]);
}
