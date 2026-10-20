use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::DateTime;
use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::{RoleClient, ServiceExt};
use scribe_core::capture::SessionMoment;
use scribe_core::library::TranscriptLibrary;
use scribe_core::ports::{ScreenshotStore, TranscriptStore};
use scribe_core::recording::{SessionHeader, SessionId};
use scribe_core::testing::frame_filled;
use scribe_mcp::server::ScribeServer;
use serde_json::{Value, json};
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder};

const DUPLEX_BUFFER_BYTES: usize = 65_536;
const RED: [u8; 4] = [255, 0, 0, 255];
const TWELVE_MINUTES_THIRTY_FOUR: Duration = Duration::from_secs(12 * 60 + 34);
const FORTY_ONE_MINUTES_TWO: Duration = Duration::from_secs(41 * 60 + 2);
const SESSION: &str = "2026-10-20-143005";
const SERVE_BUDGET: Duration = Duration::from_secs(1);

struct Call {
    store: Arc<MarkdownTranscriptStore>,
    id: SessionId,
    folder: PathBuf,
}

impl Call {
    fn started_in(folder: &Path) -> Call {
        let store = Arc::new(MarkdownTranscriptStore::new(TranscriptsFolder::at(
            folder.to_path_buf(),
        )));
        let started_at = DateTime::parse_from_rfc3339("2026-10-20T14:30:05+05:30").unwrap();
        let header = SessionHeader::recording(started_at, "speechanalyzer", "en-US");
        let id = store.create(&header).unwrap();
        store.finalise(&id, started_at).unwrap();
        Call {
            store,
            id,
            folder: folder.to_path_buf(),
        }
    }

    fn screenshot_at(&self, offset: Duration, (width, height): (u32, u32)) {
        let moment = SessionMoment {
            session: self.id.clone(),
            offset,
        };
        self.store
            .save(&moment, &frame_filled(width, height, RED))
            .unwrap();
    }

    fn session_folder(&self) -> PathBuf {
        self.folder.join(self.id.month()).join(self.id.as_str())
    }

    fn transcript_file(&self) -> PathBuf {
        self.session_folder().join(format!("{}.md", self.id))
    }

    async fn connect(&self) -> RunningService<RoleClient, ()> {
        let library = TranscriptLibrary::new(self.store.clone(), self.store.clone());
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

async fn transcript_text(client: &RunningService<RoleClient, ()>, id: &SessionId) -> String {
    let result = call(client, "get_transcript", json!({ "id": id.as_str() })).await;
    text_of(&result).to_owned()
}

fn text_of(result: &Value) -> &str {
    result["content"][0]["text"].as_str().unwrap()
}

fn is_tool_error(result: &Value) -> bool {
    result["isError"] == json!(true)
}

fn png_size(image_block: &Value) -> (u32, u32) {
    let png = STANDARD
        .decode(image_block["data"].as_str().unwrap())
        .unwrap();
    let edge = |from: usize| u32::from_be_bytes(png[from..from + 4].try_into().unwrap());
    (edge(16), edge(20))
}

fn tree_snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    collect_files(root, &mut files);
    files.sort();
    files
}

fn collect_files(folder: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) {
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push((path.clone(), fs::read(&path).unwrap()));
        }
    }
}

#[tokio::test]
async fn a_transcript_with_screenshots_ends_with_their_references_in_offset_order() {
    let folder = tempfile::tempdir().unwrap();
    let recorded = Call::started_in(folder.path());
    recorded.screenshot_at(FORTY_ONE_MINUTES_TWO, (4, 3));
    recorded.screenshot_at(TWELVE_MINUTES_THIRTY_FOUR, (4, 3));
    recorded.screenshot_at(TWELVE_MINUTES_THIRTY_FOUR, (4, 3));
    let client = recorded.connect().await;

    let text = transcript_text(&client, &recorded.id).await;

    let transcript = fs::read_to_string(recorded.transcript_file()).unwrap();
    let section = text.strip_prefix(&transcript).unwrap();
    assert!(section.starts_with("\n## Screenshots\n\n"));
    assert!(section.contains("Call get_screenshot with a name to see it."));
    assert!(section.ends_with(
        "\n- [00:12:34] 2026-10-20-143005/00-12-34.png\n- [00:12:34] 2026-10-20-143005/00-12-34-2.png\n- [00:41:02] 2026-10-20-143005/00-41-02.png\n"
    ));
    assert!(!section.contains(folder.path().to_str().unwrap()));
}

#[tokio::test]
async fn a_transcript_without_screenshots_is_returned_exactly_as_stored() {
    let folder = tempfile::tempdir().unwrap();
    let recorded = Call::started_in(folder.path());
    let client = recorded.connect().await;

    let text = transcript_text(&client, &recorded.id).await;

    assert_eq!(
        text,
        fs::read_to_string(recorded.transcript_file()).unwrap()
    );
}

#[tokio::test]
async fn a_month_file_transcript_is_returned_exactly_as_stored() {
    let folder = tempfile::tempdir().unwrap();
    let recorded = Call::started_in(folder.path());
    let month_file = folder
        .path()
        .join(recorded.id.month())
        .join(format!("{}.md", recorded.id));
    fs::rename(recorded.transcript_file(), &month_file).unwrap();
    let client = recorded.connect().await;

    let text = transcript_text(&client, &recorded.id).await;

    assert_eq!(text, fs::read_to_string(month_file).unwrap());
}

#[tokio::test]
async fn files_that_are_not_screenshot_names_are_left_out_of_the_listing() {
    let folder = tempfile::tempdir().unwrap();
    let recorded = Call::started_in(folder.path());
    recorded.screenshot_at(TWELVE_MINUTES_THIRTY_FOUR, (4, 3));
    fs::write(recorded.session_folder().join("notes.txt"), b"x").unwrap();
    fs::write(recorded.session_folder().join("shot.png"), b"x").unwrap();
    let client = recorded.connect().await;

    let text = transcript_text(&client, &recorded.id).await;

    assert!(text.ends_with("- [00:12:34] 2026-10-20-143005/00-12-34.png\n"));
    assert!(!text.contains("notes.txt"));
    assert!(!text.contains("shot.png"));
}

#[tokio::test]
async fn reading_a_transcript_changes_no_file_on_disk() {
    let folder = tempfile::tempdir().unwrap();
    let recorded = Call::started_in(folder.path());
    recorded.screenshot_at(TWELVE_MINUTES_THIRTY_FOUR, (4, 3));
    let client = recorded.connect().await;
    let before = tree_snapshot(folder.path());

    transcript_text(&client, &recorded.id).await;
    transcript_text(&client, &recorded.id).await;

    assert_eq!(tree_snapshot(folder.path()), before);
}

#[tokio::test]
async fn a_listed_screenshot_is_returned_as_a_label_then_a_png_image() {
    let folder = tempfile::tempdir().unwrap();
    let recorded = Call::started_in(folder.path());
    recorded.screenshot_at(TWELVE_MINUTES_THIRTY_FOUR, (4, 3));
    let client = recorded.connect().await;

    let result = call(
        &client,
        "get_screenshot",
        json!({ "name": "2026-10-20-143005/00-12-34.png" }),
    )
    .await;

    assert!(!is_tool_error(&result));
    assert_eq!(
        text_of(&result),
        "[00:12:34] 2026-10-20-143005/00-12-34.png — 4 × 3"
    );
    assert_eq!(result["content"][1]["type"], "image");
    assert_eq!(result["content"][1]["mimeType"], "image/png");
    assert_eq!(png_size(&result["content"][1]), (4, 3));
}

#[tokio::test]
async fn a_large_screenshot_is_returned_scaled_and_the_label_says_so() {
    let folder = tempfile::tempdir().unwrap();
    let recorded = Call::started_in(folder.path());
    recorded.screenshot_at(TWELVE_MINUTES_THIRTY_FOUR, (1800, 1000));
    let client = recorded.connect().await;

    let result = call(
        &client,
        "get_screenshot",
        json!({ "name": "2026-10-20-143005/00-12-34.png" }),
    )
    .await;

    assert_eq!(
        text_of(&result),
        "[00:12:34] 2026-10-20-143005/00-12-34.png — 1568 × 871 (scaled from 1800 × 1000)"
    );
    assert_eq!(png_size(&result["content"][1]), (1568, 871));
}

#[tokio::test]
async fn serving_a_screenshot_changes_no_file_on_disk() {
    let folder = tempfile::tempdir().unwrap();
    let recorded = Call::started_in(folder.path());
    recorded.screenshot_at(TWELVE_MINUTES_THIRTY_FOUR, (1800, 1000));
    let client = recorded.connect().await;
    let before = tree_snapshot(folder.path());

    call(
        &client,
        "get_screenshot",
        json!({ "name": "2026-10-20-143005/00-12-34.png" }),
    )
    .await;

    assert_eq!(tree_snapshot(folder.path()), before);
}

async fn assert_rejected_as_invalid(client: &RunningService<RoleClient, ()>, name: &str) {
    let result = call(client, "get_screenshot", json!({ "name": name })).await;
    assert!(is_tool_error(&result), "{name} was not rejected");
    assert_eq!(text_of(&result), format!("Invalid screenshot name: {name}"));
}

#[tokio::test]
async fn a_name_that_is_not_a_session_and_screenshot_name_is_rejected_and_the_server_keeps_answering()
 {
    let folder = tempfile::tempdir().unwrap();
    let recorded = Call::started_in(folder.path());
    recorded.screenshot_at(TWELVE_MINUTES_THIRTY_FOUR, (4, 3));
    let client = recorded.connect().await;

    assert_rejected_as_invalid(&client, "2026-10-20-143005/../00-12-34.png").await;
    assert_rejected_as_invalid(&client, "2026-10-20-143005\\00-12-34.png").await;
    assert_rejected_as_invalid(&client, "/etc/passwd").await;
    assert_rejected_as_invalid(&client, "2026-10-20-143005/sub/00-12-34.png").await;
    assert_rejected_as_invalid(&client, "2026-10-20-143005/00-12-34.jpg").await;
    assert_rejected_as_invalid(&client, "2026-10-20-143005/2026-10-20-143005.md").await;

    let still_answering = call(
        &client,
        "get_screenshot",
        json!({ "name": "2026-10-20-143005/00-12-34.png" }),
    )
    .await;
    assert!(!is_tool_error(&still_answering));
}

#[tokio::test]
async fn a_well_formed_name_with_no_screenshot_behind_it_is_not_found() {
    let folder = tempfile::tempdir().unwrap();
    let recorded = Call::started_in(folder.path());
    let client = recorded.connect().await;

    let result = call(
        &client,
        "get_screenshot",
        json!({ "name": "2026-10-20-143005/00-12-34.png" }),
    )
    .await;

    assert!(is_tool_error(&result));
    assert_eq!(
        text_of(&result),
        "No screenshot named 2026-10-20-143005/00-12-34.png"
    );
}

#[tokio::test]
async fn a_screenshot_file_that_cannot_be_decoded_is_not_found_and_the_server_keeps_answering() {
    let folder = tempfile::tempdir().unwrap();
    let recorded = Call::started_in(folder.path());
    fs::write(recorded.session_folder().join("00-12-34.png"), b"not a png").unwrap();
    let client = recorded.connect().await;

    let result = call(
        &client,
        "get_screenshot",
        json!({ "name": "2026-10-20-143005/00-12-34.png" }),
    )
    .await;

    assert!(is_tool_error(&result));
    assert_eq!(
        text_of(&result),
        "No screenshot named 2026-10-20-143005/00-12-34.png"
    );
    let transcript = transcript_text(&client, &recorded.id).await;
    assert!(transcript.contains("## Screenshots"));
}

#[tokio::test]
async fn a_screenshot_taken_while_the_server_runs_is_listed_and_served_without_a_restart() {
    let folder = tempfile::tempdir().unwrap();
    let recorded = Call::started_in(folder.path());
    let client = recorded.connect().await;
    let before = transcript_text(&client, &recorded.id).await;

    recorded.screenshot_at(TWELVE_MINUTES_THIRTY_FOUR, (4, 3));

    let after = transcript_text(&client, &recorded.id).await;
    assert!(!before.contains("## Screenshots"));
    assert!(after.ends_with("- [00:12:34] 2026-10-20-143005/00-12-34.png\n"));
    let served = call(
        &client,
        "get_screenshot",
        json!({ "name": format!("{SESSION}/00-12-34.png") }),
    )
    .await;
    assert_eq!(png_size(&served["content"][1]), (4, 3));
}

#[tokio::test]
#[ignore = "timing budget: run by hand in release mode"]
async fn serving_a_five_k_screenshot_stays_under_one_second() {
    let folder = tempfile::tempdir().unwrap();
    let recorded = Call::started_in(folder.path());
    recorded.screenshot_at(TWELVE_MINUTES_THIRTY_FOUR, (5120, 2880));
    let client = recorded.connect().await;

    let started = Instant::now();
    let result = call(
        &client,
        "get_screenshot",
        json!({ "name": "2026-10-20-143005/00-12-34.png" }),
    )
    .await;
    let elapsed = started.elapsed();

    eprintln!("served a 5120 x 2880 screenshot in {elapsed:?}");
    assert!(!is_tool_error(&result));
    assert!(elapsed < SERVE_BUDGET, "serving took {elapsed:?}");
}
