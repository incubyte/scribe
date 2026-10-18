# Plan: 001-scribe-tray-transcriber

> Spec: `docs/specs/001-scribe-tray-transcriber.md` · Context: `docs/specs/001-scribe-tray-transcriber-context.md` (HEAD 78b4778) · Authored: 2026-10-17
> Risk: MODERATE (records third parties' speech; real-time OS audio FFI on two platforms; no regulated data) · Hard floors: slice 5.9 · Fabric standards: reachable (Clean Code, Incubyte Engineering Standards, Global Component design standards, SonarJS for TS only)

---

# Part 1 — Review this

*Everything a reviewer needs to approve or redirect the approach. If you only read one part, read this one.*

## What we're doing

Scribe is a tray app that transcribes the user's calls on their own machine. It saves one Markdown file per call, and a separate small program serves those files to AI apps over MCP. All behaviour is in the [spec](../specs/001-scribe-tray-transcriber.md#5-slices). This plan says how to build it in ten slices, so that lower-tier models can write the code without re-deciding the structure.

The approach is driven by one constraint: **Scribe is the same product on macOS and Windows, but almost every edge of it differs per OS.** Audio capture, the speech engine, the permissions and sleep detection all change. So everything Scribe decides lives in one Rust crate (`scribe-core`) that knows nothing about the OS, and each OS gets small adapter crates behind three interfaces (`AudioSource`, `Transcriber`, `TranscriptStore`). Windows support is then new adapters, not a rewrite.

We are deliberately copying patterns from **Anarlog** (MIT-licensed, open-source, same stack and same problem) for the risky macOS parts: the Core Audio tap, the device-change recovery and the SpeechAnalyzer bridge. That code is already running in production. One deliberate difference from Anarlog: we exclude Scribe's own audio from the tap, which Anarlog does not.

## How it breaks down

The slices follow the spec's confirmed order. Slice 5.1 is the skeleton (workspace, tray, CI). Slice 5.3 is the foundation every later slice stands on: the core recording pipeline, ports and file format. Later slices add adapters, UI, or a read side over the same core.

| # | Slice | Delivers | Why this tier | Tier |
|---|-------|----------|---------------|------|
| 5.1 | Show Scribe in the tray | Workspace, Tauri tray shell, CI on macOS and Windows with mutation testing | Many configs, but well documented | `excellent` |
| 5.2 | Privacy statement | First React window, design tokens, saved acknowledgement | Sets UI conventions; simple logic | `excellent` |
| 5.3 | Record my side (mic) | Core domain, three ports, Markdown writer, Swift bridge to SpeechAnalyzer, Start/Stop | Every later slice inherits this design; FFI to Swift | `best` |
| 5.4 | Capture the other side (macOS) | Core Audio tap adapter, permission watchdog, device-change rebuild, two-lane ordering | Unsafe real-time FFI and recovery logic | `best` |
| 5.5 | Live popover | Anchored popover, live two-lane view, recent sessions | Standard UI work over existing events | `excellent` |
| 5.6 | Drop echoed lines | Pure echo filter wired into ordering | Small pure function with careful tests | `excellent` |
| 5.7 | MCP list/get | `scribe-mcp` binary, Markdown reader, input limits, Copy MCP config | Untrusted-input boundary; rmcp is well documented | `excellent` |
| 5.8 | MCP search | `search_transcripts` | Pure search over 5.7's reader | `fast` |
| 5.9 | Windows model download | Resumable HTTPS download with pinned SHA-256 | Integrity-critical; floored | `excellent` (floor) |
| 5.10 | Record on Windows | WASAPI loopback, voice-activity chunking, Parakeet TDT v3 | Real-time capture + ONNX inference + language auto-detect | `best` |

Cross-slice notes:
- 5.3 builds an ordering stage that handles one lane; 5.4 extends it to two lanes; 5.6 adds the echo rule inside it. That is one component growing over three slices, not three components.
- 5.10 reuses everything from 5.3–5.6 unchanged and adds only Windows adapters plus a power-event hook.

## Decisions made

**1. Transcribers are fed audio and polled for results; they never call back.** `Transcriber::push(chunk) -> Vec<TranscriptEvent>` and `finish() -> Vec<TranscriptEvent>`. Swift-to-Rust callbacks across FFI bring lifetime and threading hazards. Anarlog's SpeechAnalyzer bridge already works by pushing audio and collecting queued results, and the Windows "chunk at pauses" engine fits the same shape. Lands in 5.3, reused in 5.10. *Rejected: callback or streaming-subscription transcribers. They need `'static` closures through C and Swift, the most common source of crashes in this kind of bridge.*

**2. Every channel runs on its own worker thread; one writer thread orders the output.** Each lane (Me, Them) has a thread that pulls audio, pushes it to its transcriber and forwards results. A single writer thread receives finished utterances from both lanes. A pure `SegmentSequencer` holds each utterance until the other lane has caught up past it, then releases it in start-time order. That gives AC 5.4.2 (time order) without locks around the file, and gives 5.6's echo filter a place to look at both lanes. *Rejected: each lane appending to the file directly. Lines would land out of order and the echo filter would have nothing to compare against.*

**3. One session clock, stamped at capture.** Each adapter stamps every audio chunk with its offset from a shared `SessionClock` that starts at Start. Transcript timestamps come from those offsets, never from engine-reported time alone. That keeps the two lanes within the 1 s drift budget (AC 5.4.2), whatever each engine thinks the time is.

**4. Audio is converted to 16 kHz mono 32-bit float at the adapter edge.** The core only ever sees `AudioChunk { samples: 16 kHz mono f32, captured_at }`. On macOS the adapters resample with rubato's `Fft` resampler (a shared `audio-resample` crate). On Windows, WASAPI's built-in conversion delivers 16 kHz directly. *Rejected: resampling in the core, which would pull an audio dependency into the domain.*

**5. Windows transcription splits speech at pauses and transcribes each utterance with Parakeet TDT v3 INT8, using Silero voice-activity detection.** This resolves [spec §9.2](../specs/001-scribe-tray-transcriber.md#9-open-questions). It is the only on-device option that auto-detects language with a reasonable download (about 670 MB). Text lands about 1–2 s after each utterance ends. The trade-off is that Windows shows no word-by-word partial text, only finished utterances. *Rejected: Nemotron 3.5 streaming in auto mode. It is a 2.6 GB download, documented as less accurate in auto mode, and cannot report the detected language.*

**6. The transcript file is the contract, and it has a strict format.** Front matter is written by Scribe with a fixed key order, and the title is always double-quoted. Segment lines are `[hh:mm:ss] **Me|Them:** text`, with newlines in the text collapsed to spaces. Session ids must match `YYYY-MM-DD-HHmmss` with an optional `-N` suffix. That one rule both prevents path traversal in MCP (AC 5.7.3) and handles two sessions started in the same second. While recording, the file is append-only. Finalising rewrites the front matter through a temp file and an atomic rename in the same folder.

**7. `scribe-mcp` is read-only, lives in its own binary, and closes the connection on oversize input.** It depends only on `scribe-core` (library side) and `store-markdown`. Incoming lines are framed with rmcp's `JsonRpcMessageCodec::new_with_max_length(1 MiB)`; a line over the limit ends the session and is logged to stderr. Not-found and invalid-input errors are tool-level errors (`isError: true`), so AI clients show them to the model. *Rejected: skipping oversize lines and carrying on, which leaves the client waiting forever for a reply.*

**8. Recorder readiness is a pure function.** `RecorderState` (`Locked` | `Idle` | `Recording`) is computed from `Readiness { privacy_acknowledged, model_ready }` plus whether a session is active. The tray only renders it. That makes invariant 4.7 testable without any UI.

**9. Echo is detected on text, not audio.** A Me utterance is dropped when it overlaps a Them utterance within ±1 s and either its normalised text is at least 0.8 similar (normalised Levenshtein) or it is a contiguous run of at least 3 words of the Them text. *Rejected: acoustic echo cancellation, which is deferred by the spec.*

**10. Code-quality limits are enforced by tools, not by review.** Clippy runs with the org's limits (≤20 lines per function, ≤3 arguments, cognitive complexity ≤5) and `-D warnings`. ESLint uses the org's strict rule set. Mutation testing runs on every PR: cargo-mutants for Rust (only on code the PR changed) and Stryker for TypeScript, each posting a PR comment. Merge is blocked below 75%.

## Where this could go wrong

- **The Core Audio tap fails silently.** A missing permission gives all-zero audio, not an error. Guarded by the 5.4 checks that the watchdog fires within 5 s and that the plist key exists in the bundle.
- **SpeechAnalyzer's streaming input bug** ([spec §9.3](../specs/001-scribe-tray-transcriber.md#9-open-questions)). Slice 5.3 starts with a 2-minute check on the current macOS 26.x; a documented fallback exists. Guarded by the 5.3 "probe recorded" check.
- **ONNX runtime version clash on Windows.** The usual Silero crate pins a different `ort` version from parakeet-rs, so 5.10 runs Silero through parakeet-rs's own `ort` re-export. Guarded by the 5.10 single-`ort` check.
- **The ordering stage holds lines too long.** If one lane stalls, the other lane's lines could wait forever. Guarded by the 5.4 and 5.6 maximum-hold checks.
- **Commits during the build.** `/anthara:plan-implementer` commits once per slice, but your global instruction says not to commit unless asked. This needs your explicit go-ahead at hand-off.

## What's out of scope

Acoustic echo cancellation, separating multiple remote speakers, picking one meeting app to capture, meeting auto-detection, encryption at rest, Intel Macs, Linux, picking a transcription language on macOS, a choosable transcripts folder, live streaming over MCP, and code-signing setup. Signing needs certificates ([spec §9.5](../specs/001-scribe-tray-transcriber.md#9-open-questions)); CI builds unsigned artifacts until then. There is no post-merge end-to-end run on dev or staging, because a desktop app has no such environments; slice Verification blocks cover end-to-end checks.

---

# Part 2 — Build detail

*Prescriptive detail for the build agent. Reviewers can skim or skip.*

**Confidence markers used below:** `[verified]` = the planner read the code or docs and confirmed it. `[inferred]` = pattern-matched from a sibling implementation, not confirmed — the build agent should check before assuming.

## Shared conventions (all slices)

- **Architecture:** Hexagonal, per [`docs/ARCHITECTURE.md`](../ARCHITECTURE.md) and [spec §7.2](../specs/001-scribe-tray-transcriber.md#72-architectural-style). Ports are defined in `scribe-core`, secondary adapters implement them, and primary adapters (`apps/*`) wire them at a single composition root.
- **Dependency direction:** `apps/* → scribe-core ← adapters`. `scribe-core` has no `#[cfg(target_os)]` and no Tauri or OS crates. `scribe-mcp` never depends on `audio-*` or `stt-*`. Only `stt-parakeet` may depend on an HTTP client.
- **Workspace layout** (domain-based, per org standard): `Cargo.toml` (workspace), `apps/scribe-tray/` (Tauri: `src-tauri/` Rust plus `src/` React), `apps/scribe-mcp/`, `crates/{scribe-core, audio-resample, audio-mic, audio-macos, audio-windows, stt-speechanalyzer, stt-parakeet, store-markdown}`. Inside `scribe-core`, the modules are `recording/` and `library/`, plus `ports.rs`.
- **Lexicon** (from [context §4b](../specs/001-scribe-tray-transcriber-context.md#4b-slice-lexicon); do not rename):
  - Ports: `AudioSource`, `Transcriber`, `TranscriptStore`.
  - Domain: `Recorder`, `Session`, `Segment`, `EchoFilter`, `SessionId`, `Speaker::{Me, Them}`, `SessionStatus::{Recording, Complete, Incomplete}` (serialised `recording|complete|incomplete`), `RecorderState::{Locked, Idle, Recording}`.
  - Events: `RecordingStarted`, `PartialSegmentUpdated`, `SegmentFinalised`, `RecordingStopped`, `CaptureSilenced`, `OutputDeviceChanged`, `ModelDownloadProgressed|Completed|Failed`.
  - Planner-added names (used verbatim in every slice): `AudioChunk`, `SessionClock`, `TranscriptEvent::{Partial, Final}`, `Utterance`, `SegmentSequencer`, `Readiness`, `RecordingListener`, `SessionSummary`, `SessionDocument`, `TranscriptLibrary`, `SearchHit`.
- **Pinned versions:**
  - Rust and crates: Rust ≥ 1.88 (rmcp MSRV) `[verified]`; tauri 2.12 `[verified]`; rmcp 3.5 `[verified]`; cidre 0.29 `[verified]`; cpal 0.18 `[verified]`; rubato 5.0 `[verified]`; wasapi 0.26 `[verified]`; parakeet-rs 0.4.1 `[verified]`; swift-rs 1.0.9 `[verified]`; serde-saphyr 2.0 `[verified]`; chrono 0.4 `[verified]`; reqwest 0.13 (rustls default) `[verified]`; sha2 0.11 `[verified]`; strsim 0.11 `[inferred]`; dirs 6 `[inferred]`; thiserror 2 `[inferred]`; crossbeam-channel 0.5 `[inferred]`.
  - Frontend: Vite + React 19 + TypeScript strict; Vitest + @testing-library/react + vitest-axe; @tauri-apps/api 2.12 `[verified]`.
- **Errors:** one `thiserror` enum per crate (`CaptureError`, `TranscribeError`, `StoreError`, `LibraryError`, `DownloadError`). No `Option` used as an error and no sentinel values. Every boundary validates and fails fast.
- **Logging:** the `tracing` crate. Fields hold session ids, file names and error kinds only, never segment text (AC 5.3.11). `scribe-mcp` logs to stderr only.
- **Tests:**
  - Rust: behaviour tests go in each crate's `tests/` directory and exercise the public API with fakes (`FakeAudioSource`, `ScriptedTranscriber`, `InMemoryTranscriptStore`). These live in `scribe-core/src/testing.rs` behind a `testing` feature, so every crate's tests share them. Inline `#[cfg(test)]` is only for private pure helpers. Platform adapter tests are `#[ignore]` integration tests run manually on real hardware.
  - TypeScript: `*.spec.ts(x)` beside the component.
  - Test names are business language, cases are chosen with ZOMBIES, and parameterised tables stay at 6 rows or fewer. Test names never contain AC ids.

---

## Slice 5.1 — Show Scribe in the system tray

**Key decision:** This slice stands up the workspace, a Tauri 2 tray-only shell and CI on both OSes, so later slices only add code. It sets the tool-enforced quality limits once (Decision 10).

**Structure**
- `Cargo.toml` (new): workspace members, `[workspace.lints]` with `clippy::pedantic` warn and `clippy::too_many_lines`, `cognitive_complexity`, `too_many_arguments` deny. Rust edition 2024.
- `clippy.toml` (new): `too-many-lines-threshold = 20`, `too-many-arguments-threshold = 3`, `cognitive-complexity-threshold = 5` `[inferred: key names; run cargo clippy to confirm they are accepted]`.
- `apps/scribe-tray/src-tauri/tauri.conf.json` (new): `app.windows: []` (no window yet); `app.security.csp` set to the object form `{ "default-src": "'self'", "script-src": "'self'", "style-src": "'self'", "connect-src": "ipc: http://ipc.localhost" }` `[verified: object form]`; bundle identifier `com.incubyte.scribe`.
- `apps/scribe-tray/src-tauri/Cargo.toml`: `tauri = { version = "2.12", features = ["tray-icon"] }`, `tauri-plugin-opener` (crate version unverified; pin to the latest 2.x on crates.io at build time) `[inferred]`.
- `apps/scribe-tray/src-tauri/src/main.rs` (new): `fn main()`, which builds the app and calls `tray::install`.
- `apps/scribe-tray/src-tauri/src/tray.rs` (new): `pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<TrayIcon<R>>` and `fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>>`. Menu item ids are the constants `MENU_OPEN_FOLDER = "open-transcripts-folder"` and `MENU_QUIT = "quit"`. Uses `TrayIconBuilder::with_id("main")`, `.icon_as_template(true)`, `.tooltip("Scribe")`, `.show_menu_on_left_click(true)` (5.5 flips this), and `.on_menu_event` `[verified]`.
- `apps/scribe-tray/src-tauri/src/menu_action.rs` (new): `pub enum MenuAction { OpenTranscriptsFolder, Quit }` and `impl MenuAction { pub fn from_id(id: &str) -> Option<MenuAction> }`. This is a dispatch table, not a match spread across handlers.
- `crates/store-markdown/src/transcripts_folder.rs` (new): `pub struct TranscriptsFolder(PathBuf)` with `pub fn default_location() -> Result<TranscriptsFolder, StoreError>` (`dirs::document_dir()/Scribe`), `pub fn at(path: PathBuf) -> TranscriptsFolder`, `pub fn ensure_exists(&self) -> Result<&Path, StoreError>` (creates the folder with mode `0o700` on unix; on Windows it inherits the user-profile ACL `[inferred]`), and `pub fn path(&self) -> &Path`.
- macOS no-Dock: in `setup`, `#[cfg(target_os = "macos")] app.set_activation_policy(ActivationPolicy::Accessory)` `[verified]`.
- `apps/scribe-tray/src-tauri/capabilities/default.json` (new): `opener:allow-open-path` scoped to `$DOCUMENT/Scribe/**` `[inferred: $DOCUMENT variable name]`.
- `.github/workflows/ci.yml` (new):
  - Matrix `macos-latest` and `windows-latest`, running fmt check, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, then `npm ci && npm run lint && npm test` in `apps/scribe-tray`.
  - A `mutation` job on pull requests: `cargo mutants --in-diff` on the PR diff, plus `npx stryker run`.
  - A summary posted with `marocchino/sticky-pull-request-comment@v2` `[inferred]`.
  - Fails below 75%.
- `apps/scribe-tray/eslint.config.js` (new): `max-depth: 3`, `complexity: 5`, `no-param-reassign: error`, `no-console: warn`, `eqeqeq`, `curly`, `no-var`, `prefer-const` (org rule) `[verified: rule list from standard]`; plus `jsx-a11y` recommended.
- `CLAUDE.md` (new, short): the build, test and lint commands and a pointer to `docs/ARCHITECTURE.md` (context advisory).

**Collaborators:** `tray.rs` calls `TranscriptsFolder` from `store-markdown` and the opener plugin. Nothing depends on the tray.

**Control flow:**
- Menu event → `MenuAction::from_id`. An unknown id is ignored (guard).
- `OpenTranscriptsFolder` → `TranscriptsFolder::default_location()?.ensure_exists()?` → opener `open_path`. On error, log the error kind and do nothing else.
- `Quit` → `app.exit(0)`.

**Tests:**
- New `crates/store-markdown/tests/transcripts_folder.rs`:
  - creating the folder when it is missing;
  - the folder is private to the user (unix mode `0o700`; Windows `#[ignore]` manual) (AC 5.1.3);
  - an existing folder is left untouched.
- New `apps/scribe-tray/src-tauri/tests/menu_action.rs`: menu ids map to actions; unknown ids map to none (AC 5.1.2).
- AC 5.1.1 and 5.1.4 are checked by hand per the Verification block.
- AC 5.1.5 is shown by CI running on this PR.

**Standards that bite here:** Clean Code (small functions, intent-revealing names); Incubyte Engineering Standards (domain-based structure, mutation testing in CI commenting on PRs, failing tests block merge, ESLint strict rules, Conventional Commits, `<type>/<kebab>` branches); SonarJS `content-security-policy`.

**Quality checks**
- No window or Dock icon appears at launch on macOS; no taskbar entry on Windows. (AC 5.1.1)
- **CI runs the same steps on `macos-latest` and `windows-latest`, and a mutation summary comment appears on the PR.** (AC 5.1.5)
- The clippy limits are active: a deliberately 21-line function fails `cargo clippy` locally and is then removed. (Decision 10)
- The transcripts folder is created with no group or other permission bits on unix. (AC 5.1.3)
- `scribe-core` exists as an empty-ports crate with no dependencies other than `thiserror`.

---

## Slice 5.2 — Acknowledge the local-only privacy statement

**Key decision:** This slice adds the first React window and the persisted acknowledgement, and it introduces `Readiness` and `RecorderState` (Decision 8), so 5.3 can gate Start without new design. It also sets the CSS custom-property tokens (colour, spacing, focus ring) that 5.5 and 5.9 reuse.

**Structure**
- `crates/scribe-core/src/recording/readiness.rs` (new):
  - `pub struct Readiness { pub privacy_acknowledged: bool, pub model_ready: bool }`
  - `pub enum RecorderState { Locked, Idle, Recording }`
  - `pub fn recorder_state(readiness: &Readiness, has_active_session: bool) -> RecorderState`
- `apps/scribe-tray/src-tauri/src/privacy.rs` (new):
  - `pub fn is_acknowledged<R: Runtime>(app: &AppHandle<R>) -> bool`
  - `pub fn acknowledge<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()>`
  - `pub fn show_statement<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()>` builds a `WebviewWindowBuilder` with label `privacy`, size 420×300, centred, not resizable, with the route `/#privacy`.
  - Storage is `tauri-plugin-store` 2.5 with file `settings.json` and key `privacy_acknowledged` `[verified]`.
- `#[tauri::command] fn acknowledge_privacy(app) -> Result<(), String>`, which calls `acknowledge` and closes the `privacy` window.
- Tray menu gains `MENU_PRIVACY = "privacy"` and `MenuAction::ShowPrivacy`.
- `apps/scribe-tray/src/` (new React app):
  - `main.tsx` routes by hash to `PrivacyStatement`.
  - `privacy/PrivacyStatement.tsx` holds the four statements from [spec §5.2](../specs/001-scribe-tray-transcriber.md#52-acknowledge-the-local-only-privacy-statement) verbatim and an `I understand` button that `invoke`s `acknowledge_privacy`.
  - `theme/tokens.css` uses `prefers-color-scheme` light and dark, has a contrast-checked palette and a `--focus-ring` token.
- Capabilities: `store:default`, `core:window:allow-close`.

**Collaborators:** The tray calls `privacy::is_acknowledged` at startup and shows the statement if it returns false. The React app calls the Tauri command. `scribe-core` readiness is pure.

**Control flow:**
- On startup, if the acknowledgement is missing, show the statement.
- Clicking "I understand" acknowledges, closes the window, and refreshes the tray menu enabled-state (a no-op until 5.3 adds Start).
- Closing the window without acknowledging leaves the state unchanged.
- The "Privacy" menu item shows the statement regardless of state.

**Tests:**
- New `crates/scribe-core/tests/recorder_readiness.rs` (cases chosen with ZOMBIES):
  - locked until privacy is acknowledged;
  - locked while the speech model is missing;
  - idle when both are ready;
  - recording while a session is active.
  - Covers AC 5.2.2 (the gate half) and feeds 5.3.1.
- New `apps/scribe-tray/src/privacy/PrivacyStatement.spec.tsx`:
  - shows all four statements;
  - focus starts on "I understand" and Enter acknowledges (with `mockIPC`);
  - no axe violations.
  - Covers AC 5.2.1 and 5.2.5.
- AC 5.2.2–5.2.4 persistence is checked by hand via Verification (the plugin store is not mockable in-process `[inferred]`).

**Standards that bite here:** WCAG 2.2 AA (contrast ≥ 4.5:1, focus visible, keyboard operability, accessible name); Global Component a11y verification; SonarJS `prefer-read-only-props`, `jsx-no-leaked-render`; CSP.

**Quality checks**
- `recorder_state` is a pure function with no I/O, and its tests cover all three states. (Decision 8)
- Focus lands on "I understand" when the window opens, and the focus ring is visible in both colour schemes. (AC 5.2.5)
- Both themes pass the axe contrast check with zero violations. (AC 5.2.5)
- The four statement texts match the spec wording exactly. (AC 5.2.1)

---

## Slice 5.3 — Record my side of a call

**Key decision:** This is the foundation slice. It defines the three ports (Decisions 1, 3, 4), the `Recorder` with one worker thread per lane plus a writer thread (Decision 2, single lane for now), the Markdown writer (Decision 6), the swift-rs SpeechAnalyzer bridge (Decision 1) and the Start/Stop tray wiring. Everything later plugs into these names.

**Structure**
- `crates/scribe-core/src/ports.rs` (new):
  - `pub trait AudioSource: Send { fn start(&mut self, clock: SessionClock) -> Result<Receiver<AudioChunk>, CaptureError>; fn stop(&mut self); }`
  - `pub trait Transcriber: Send { fn push(&mut self, chunk: &AudioChunk) -> Result<Vec<TranscriptEvent>, TranscribeError>; fn finish(&mut self) -> Result<Vec<TranscriptEvent>, TranscribeError>; }`
  - `pub trait TranscriptStore: Send + Sync { fn create(&self, header: &SessionHeader) -> Result<SessionId, StoreError>; fn append(&self, id: &SessionId, segment: &Segment) -> Result<(), StoreError>; fn finalise(&self, id: &SessionId, ended_at: DateTime<FixedOffset>) -> Result<(), StoreError>; fn mark_abandoned_incomplete(&self) -> Result<usize, StoreError>; }`. The read methods are added in 5.7.
  - `pub trait RecordingListener: Send + Sync { fn on_event(&self, event: RecordingEvent); }`
- `crates/scribe-core/src/recording/`:
  - `audio.rs`: `pub struct AudioChunk { pub samples: Vec<f32>, pub captured_at: Duration }` and `pub const SAMPLE_RATE_HZ: u32 = 16_000`.
  - `clock.rs`: `pub struct SessionClock` (Copy) with `pub fn start() -> SessionClock` and `pub fn elapsed(&self) -> Duration`.
  - `segment.rs`: `pub enum Speaker { Me, Them }`; `pub struct Utterance { pub start: Duration, pub end: Duration, pub text: String }`; `pub enum TranscriptEvent { Partial(String), Final(Utterance) }`; `pub struct Segment { pub speaker: Speaker, pub start: Duration, pub end: Duration, pub text: String }`.
  - `session.rs`:
    - `pub struct SessionId(String)` with `pub fn parse(raw: &str) -> Result<SessionId, InvalidSessionId>`, accepting only `^\d{4}-\d{2}-\d{2}-\d{6}(-\d{1,3})?$`;
    - `pub fn from_start(started_at: DateTime<FixedOffset>, collision_suffix: u16) -> SessionId`;
    - `pub struct SessionHeader { id, title, started_at, ended_at: Option<…>, status: SessionStatus, engine: String, language: String }`;
    - `pub fn default_title(started_at) -> String` returning `Call YYYY-MM-DD HH:mm`.
  - `events.rs`: `pub enum RecordingEvent { RecordingStarted { id }, PartialSegmentUpdated { speaker, text }, SegmentFinalised(Segment), RecordingStopped { id }, CaptureSilenced { speaker }, OutputDeviceChanged }`.
  - `sequencer.rs`: `pub struct SegmentSequencer` with `pub fn new(lanes: &[Speaker]) -> SegmentSequencer`, `pub fn accept(&mut self, segment: Segment)`, `pub fn advance(&mut self, speaker: Speaker, watermark: Duration)`, `pub fn release_ready(&mut self) -> Vec<Segment>` and `pub fn drain(&mut self) -> Vec<Segment>`. With one lane it releases immediately.
  - `recorder.rs`:
    - `pub struct Recorder` built with `pub fn new(store: Arc<dyn TranscriptStore>, listener: Arc<dyn RecordingListener>) -> Recorder`.
    - `pub fn start(&mut self, lanes: Vec<Lane>, header: SessionHeader) -> Result<SessionId, RecorderError>`
    - `pub fn stop(&mut self) -> Result<(), RecorderError>`
    - `pub fn is_recording(&self) -> bool`
    - `pub struct Lane { pub speaker: Speaker, pub source: Box<dyn AudioSource>, pub transcriber: Box<dyn Transcriber> }`
  - Private: `lane_worker.rs` with `fn run_lane(lane, clock, out: Sender<LaneMessage>)`, and `writer.rs` with `fn run_writer(rx, sequencer, store, id, listener)`. `LaneMessage` is `Final(Segment)` | `Watermark(Speaker, Duration)` | `Partial(Speaker, String)` | `Ended(Speaker)`.
- `crates/scribe-core/src/testing.rs` (feature `testing`): `FakeAudioSource::with_chunks(Vec<AudioChunk>)`, `ScriptedTranscriber::new(Vec<Vec<TranscriptEvent>>)`, `InMemoryTranscriptStore` and `RecordingListenerSpy`.
- `crates/audio-resample/src/lib.rs` (new): `pub struct MonoResampler` with `pub fn new(input_rate_hz: u32) -> Result<MonoResampler, ResampleError>` and `pub fn process(&mut self, interleaved: &[f32], channels: u16) -> Vec<f32>`. It downmixes by averaging, then uses rubato `Fft` with `FixedSync::Input` over fixed input chunks and buffers the remainder `[verified: rubato 5 README guidance]`.
- `crates/audio-mic/src/lib.rs` (new):
  - `pub struct MicSource` implementing `AudioSource`.
  - It uses cpal `default_host().default_input_device()` and `default_input_config()`, with one generic `build_stream::<S: SizedSample>` over I16, I32 and F32 converted via `f32::from_sample` `[verified]`.
  - The callback pushes into a ringbuf; a pump thread resamples and stamps chunks with `clock.elapsed()` every 100 ms of audio.
  - A permission-denied or build error maps to `CaptureError::PermissionDenied | DeviceUnavailable` `[inferred: cpal error variant names]`.
- `crates/stt-speechanalyzer/` (new; macOS only via `[target.'cfg(target_os="macos")'.dependencies]`):
  - `build.rs`: `swift_rs::SwiftLinker::new("26.0").with_package("scribe-speech", "./swift-lib/").link()` plus the `/usr/lib/swift` rpath `[verified: pattern from anarlog crates/transcribe-speechanalyzer]`.
  - `swift-lib/Sources/lib.swift` exports `@_cdecl` functions `scribe_speech_start(locale: SRString) -> SRString`, `scribe_speech_push(samples: SRData) -> SRString`, `scribe_speech_finish() -> SRString`, `scribe_speech_asset_status(locale) -> SRString` and `scribe_speech_install_asset(locale) -> SRString`. Each returns JSON: `{ "ok": bool, "error"?: string, "events"?: [{ "kind": "partial"|"final", "text", "start_s"?, "end_s"? }] }`.
  - The Swift side uses `SpeechTranscriber(locale:, transcriptionOptions: [], reportingOptions: [.volatileResults], attributeOptions: [.audioTimeRange])`, `SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith:)` plus `AVAudioConverter` from 16 kHz mono Float32, `AsyncStream<AnalyzerInput>.makeStream()`, `analyzer.start(inputSequence:)`, and a detached task that drains `transcriber.results` into a locked queue. `finish` calls `continuation.finish()` and then `finalizeAndFinishThroughEndOfInput()` `[verified: anarlog lib.swift + WWDC25 session 277]`.
  - `src/lib.rs`: `pub struct SpeechAnalyzerTranscriber` implementing `Transcriber` (one instance per lane; the Swift side keys sessions by a lane token), `pub fn english_asset_status() -> AssetStatus` and `pub fn install_english_asset() -> Result<(), TranscribeError>`.
- `crates/store-markdown/src/`:
  - `writer.rs`: `pub struct MarkdownTranscriptStore` with `pub fn new(folder: TranscriptsFolder) -> MarkdownTranscriptStore`, implementing `TranscriptStore`.
  - `format.rs`: `pub fn render_header(header: &SessionHeader) -> String` (fixed key order, quoted title); `pub fn render_segment(segment: &Segment) -> String`, which formats `[hh:mm:ss] **Me:** text` with newlines collapsed; and `pub fn parse_header(raw: &str) -> Result<SessionHeader, StoreError>` (serde-saphyr `[verified: crate]`).
- `apps/scribe-tray/src-tauri/src/`:
  - `recording.rs`: `pub struct RecordingController` holding `Mutex<Recorder>` and `Readiness`, with `pub fn start(&self) -> Result<SessionId, …>`, `pub fn stop(&self)` and `pub fn state(&self) -> RecorderState`.
  - `lanes.rs`: `pub fn build_lanes() -> Vec<Lane>`, the cfg-gated composition root (macOS: mic plus SpeechAnalyzer).
  - `tray_listener.rs`: `pub struct TrayRecordingListener` implementing `RecordingListener`. It updates the tray icon, the menu text with elapsed time (1 s ticker) and emits Tauri events.
  - `power/macos.rs`: `pub fn on_will_sleep(callback: impl Fn() + Send + 'static)` using `objc2-app-kit` `NSWorkspaceWillSleepNotification` on `NSWorkspace::sharedWorkspace().notificationCenter()` `[verified: symbol exists; observer API inferred]`.
  - `Info.plist` (new, next to tauri.conf.json): `NSMicrophoneUsageDescription` `[verified: auto-merged]`.
  - Recording icon: `icons/tray-recording.png`, a template image whose shape differs (filled circle) from idle.
  - Menu gains `MENU_TOGGLE_RECORDING = "toggle-recording"`.

**Collaborators:** The tray constructs `Recorder` with `MarkdownTranscriptStore` and `TrayRecordingListener`, and passes lanes from `lanes.rs`. `Recorder` only knows the ports. Adapters depend on `scribe-core`.

**Control flow:**
- **Start:**
  1. Guard: the state must be `Idle`, otherwise return `RecorderError::NotReady` or `AlreadyRecording` (AC 5.3.1, 5.3.9).
  2. `SessionHeader` with status `recording`, then `store.create` (on a collision, retry with suffix `-2`…`-99`), then `SessionClock::start()`.
  3. For each lane, `source.start(clock)`. If a lane fails with `PermissionDenied`: stop the started lanes, delete nothing (the file has not been written yet), and return the error. To guarantee this, order it as "start sources first, then `store.create`".
  4. Spawn the lane workers and the writer, then emit `RecordingStarted`.
- **Lane worker:** loop on `recv` chunk → `transcriber.push` → forward `Partial` / `Final` (mapped to a `Segment` with the lane speaker and `start = utterance.start` relative to the clock) → send `Watermark(chunk.captured_at + chunk duration)`. When the channel closes, `transcriber.finish()`, forward the results, then send `Ended`. On a transcriber error, log the kind and stop the lane (the other lanes continue).
- **Writer:** for each message, update the sequencer; for each released segment, `store.append`, then emit `SegmentFinalised`. On `Ended` from all lanes, `sequencer.drain()` → append → `store.finalise` → emit `RecordingStopped`.
- **Stop:** `source.stop()` on each lane (this closes the channels), join the threads.
- **Sleep:** the power hook calls `controller.stop()`.
- **Startup:**
  1. `store.mark_abandoned_incomplete()`, which flips any `status: recording` file to `incomplete` via an atomic header rewrite (AC 5.3.8).
  2. Check the SpeechAnalyzer English asset. If it is missing, `install_english_asset` on a background thread with a tray tooltip of "Preparing speech model…"; `Readiness.model_ready` is set when it finishes.
- **Finalise:** read the file, replace the header block, write `<id>.md.tmp` in the same folder with `0o600`, `sync_data`, then rename.

**Tests:**
- New `crates/scribe-core/tests/recording_session.rs`, using fakes:
  - start creates a recording session and emits RecordingStarted;
  - finished utterances are appended as Me lines in order;
  - stop finalises the session as complete;
  - start is refused while locked;
  - start is refused while already recording;
  - a lane that fails to start leaves no session behind;
  - a transcriber error ends only that lane.
  - Covers AC 5.3.1, 5.3.3, 5.3.4 (ordering and content, not latency), 5.3.6, 5.3.9, and invariants 4.4 and 4.5.
- New `crates/store-markdown/tests/transcript_file.rs`, using a temp dir:
  - the header has all keys and a quoted title;
  - segment lines render in the agreed format;
  - newlines in text are collapsed;
  - finalising sets ended_at and complete without touching segment lines;
  - abandoned recording files are marked incomplete;
  - files are private to the user;
  - a second session in the same second gets a suffix.
  - Covers AC 5.3.3, 5.3.5 (file side), 5.3.6, 5.3.8, invariant 4.5 and Decision 6.
- New `crates/scribe-core/tests/session_id.rs`: valid ids, path separators, `..`, empty input, over-long suffix (Decision 6, feeds AC 5.7.3).
- New `crates/scribe-core/tests/no_audio_at_rest.rs`: a full fake recording into a temp transcripts folder leaves exactly one `.md` and nothing in a watched temp dir (AC 5.3.10, the disk half).
- CI step (new): `cargo tree -e normal -i reqwest` must show only `stt-parakeet` as a dependent; no HTTP client in the macOS app graph (AC 5.3.10, the network half; invariant 4.1).
- New `crates/audio-resample/tests/resampling.rs`: 48 kHz stereo becomes 16 kHz mono of the expected length; a 1 kHz tone stays at 1 kHz (FFT peak).
- `crates/stt-speechanalyzer/tests/live_smoke.rs` `#[ignore]`: transcribes a bundled 5 s English WAV fixture (`tests/fixtures/hello.wav`) and gets non-empty final text. Run manually on macOS 26.
- AC 5.3.2, 5.3.5 (icon), 5.3.7 and the latency part of 5.3.4 are checked by hand per Verification.

**Standards that bite here:** Clean Code (SRP: the lane worker, writer, sequencer and store each have one reason to change; DIP; ≤20-line functions; ≤3 arguments, hence `Lane` is a struct; command-query separation, so `stop()` returns `()`); `confidential-information-logging`; Fail fast at boundaries (`SessionId::parse`); `file-permissions`; no null.

**Quality checks**
- **`scribe-core` compiles with no OS, Tauri, audio or HTTP dependency, and has no `cfg(target_os)` anywhere.** (ARCHITECTURE §7)
- No `tracing` call anywhere passes segment or partial text as a field or in a message. (AC 5.3.11)
- The file contains only lines from `render_segment` after the header; no segment line is rewritten during recording. (Invariant 4.5)
- The Swift bridge exposes exactly five `@_cdecl` functions and no Rust function pointers cross into Swift. (Decision 1)
- Starting sources happens before `store.create`, so a denied permission leaves no file. (AC 5.3.2)
- Every `Transcriber` and `AudioSource` impl is used only through the trait outside its own crate.

**Open at build time (AC 5.3.4):** whether `SpeechAnalyzer.start(inputSequence:)` fails with `nilError` on the build machine's macOS 26.x ([spec §9.3](../specs/001-scribe-tray-transcriber.md#9-open-questions)). Probe: run `live_smoke` first. If it fails with nilError, switch the Swift side to feeding fixed 2 s `AVAudioPCMBuffer` batches through `analyzer.analyzeSequence(_:)` `[inferred API name; confirm in Speech framework headers]`, keeping the same C ABI. Record the outcome in the escalation log.

---

## Slice 5.4 — Capture the other side of a call on macOS

**Key decision:** This slice adds a second lane (`Them`) through a Core Audio process tap that excludes Scribe's own process. It copies Anarlog's tap, aggregate-device and recovery pattern (Part 1 intro), extends `SegmentSequencer` to two lanes (Decision 2), and adds the all-zero-audio watchdog for the silent permission failure.

**Structure**
- `crates/audio-macos/src/` (new; macOS only):
  - `system_output.rs`: `pub struct SystemOutputSource` implementing `AudioSource`.
  - `tap.rs`: `fn create_tap() -> Result<TapHandle, CaptureError>`.
    - Uses `ca::TapDesc::with_mono_global_tap_excluding_processes(&exclusions)`, where `exclusions` holds the Core Audio process object id from `ca::Process::with_pid(std::process::id() as i32)`. If the lookup fails, use an empty list ("nothing to exclude yet") `[verified: API; inferred: empty-list fallback]`.
    - Then `create_process_tap()` → `TapGuard`, then an aggregate device dict (`is_private: true`, `tap_auto_start: false`, a fresh uuid, `tap_list: [sub-tap uid]`), then `AggregateDevice::with_desc`, `create_io_proc_id` and `device_start` `[verified: anarlog crates/audio-actual/src/speaker/macos.rs]`.
    - Read the format from `tap.asbd()`.
  - `io_proc.rs`: an `extern "C" fn` IOProc with the cidre signature `[verified]`. It copies `buffers[0]` into a `ringbuf` producer, with no allocation and no locks.
  - `pump.rs`: `fn run_pump(consumer, resampler, clock, tx, watchdog)` drains the ring every 100 ms, resamples with `MonoResampler`, stamps the chunk, sends it, and feeds the watchdog.
  - `device_watch.rs`: `pub struct OutputDeviceWatch` with `fn new(on_change: Sender<()>) -> Result<…>`, using `ca::System::OBJ.add_prop_listener(HW_DEFAULT_OUTPUT_DEVICE.global_addr(), …)`, and `remove_prop_listener` on Drop `[verified: anarlog device-monitor]`. Events are debounced over 300 ms.
- `crates/scribe-core/src/recording/silence_watchdog.rs` (new, pure): `pub struct SilenceWatchdog` with `pub fn new(threshold: Duration) -> SilenceWatchdog` and `pub fn observe(&mut self, chunk: &AudioChunk) -> WatchdogVerdict` (`Healthy` | `SilencedFor(Duration)` | `AlreadyReported`). A chunk counts as silent only if every sample is exactly `0.0`. It reports once per silent stretch.
- The `scribe-core` sequencer is extended to two lanes: release segment S when every other lane's watermark ≥ `S.start`, or when S has been held longer than `MAX_HOLD` (1.5 s of S's own lane time past `S.end`).
- `apps/scribe-tray/src-tauri/Info.plist`: add `NSAudioCaptureUsageDescription` `[verified: required; Xcode cannot add it]`.
- `apps/scribe-tray/src-tauri/src/notifications.rs` (new): `pub fn notify_cannot_hear_other_side<R>(app)` (text from [context §4b](../specs/001-scribe-tray-transcriber-context.md#4b-slice-lexicon)). Clicking it opens `x-apple.systempreferences:com.apple.preference.security?Privacy_AudioCapture` `[inferred: anchor name; fall back to the Privacy pane root if it does not open]`. Uses `tauri-plugin-notification` 2.7 `[verified]`.
- `lanes.rs`: the macOS lane list becomes `[Me: MicSource + SpeechAnalyzer, Them: SystemOutputSource + SpeechAnalyzer]`.

**Collaborators:** `SystemOutputSource` uses `audio-resample` and `scribe-core`. The tray listener reacts to `CaptureSilenced { speaker: Them }` by sending the notification. The Recorder forwards watchdog verdicts as events; the lane worker owns one watchdog per lane.

**Control flow:**
- **Start:** create the tap, the aggregate device and the IOProc, start the device and the pump.
- **Pump:** every 100 ms, drain, resample, send. Every 128 drains, re-read `tap.asbd()`; if the sample rate changed, rebuild the resampler.
- **Device change:** debounce, then tear down in order (stop device → destroy IOProc → drop aggregate → drop tap), then create everything again. Retry with backoff of 1, 2 and 4 s; after 3 failures, emit a `CaptureError` and end the lane. The pump keeps stamping from the same `SessionClock`, so lanes stay aligned (AC 5.4.5, Decision 3).
- **Watchdog:** a `SilencedFor(≥5 s)` verdict within the first 5 s after Start emits `CaptureSilenced { Them }` once. The Me lane is unaffected (AC 5.4.4).
- **Stop:** tear down in the same order.

**Tests:**
- Extend `crates/scribe-core/tests/recording_session.rs`:
  - Me and Them lines are written in start-time order;
  - a stalled lane does not hold the other lane's lines beyond the maximum hold;
  - the other side going silent raises one warning while my side keeps recording.
  - Covers AC 5.4.1, 5.4.2 and 5.4.4.
- New `crates/scribe-core/tests/silence_watchdog.rs`:
  - real quiet (tiny non-zero samples) is healthy;
  - exact zeros for five seconds are reported once;
  - recovery resets the report.
  - Covers AC 5.4.4.
- New `crates/scribe-core/tests/segment_ordering.rs` (pure sequencer, ZOMBIES): zero segments, one lane, interleaved lanes, equal start times (Me first), drain on stop.
- `crates/audio-macos/tests/tap_smoke.rs` `#[ignore]`: on a signed build, plays a tone with `afplay` and asserts non-zero samples arrive within 2 s. Manual.
- AC 5.4.3, 5.4.5 and 5.4.6 are checked by hand per Verification.

**Standards that bite here:** Clean Code (each teardown step is a named function, nesting ≤ 3); Separation of concerns (the IOProc only copies bytes); Fail fast (tap creation errors map to typed `CaptureError`); no allocation or locks on the real-time thread (performance hot path, [context §7](../specs/001-scribe-tray-transcriber-context.md#7-hot-paths-and-performance-landmines)).

**Quality checks**
- **Scribe's own process object id is in the tap's exclusion list whenever the lookup succeeds.** (Invariant 4.8)
- The IOProc body performs no heap allocation, no locking and no logging.
- Teardown order is device stop → IOProc destroy → aggregate drop → tap drop, on both Stop and device change.
- The sequencer never holds a segment longer than `MAX_HOLD` past its lane watermark. (AC 5.4.2, 5.6.4)
- `CaptureSilenced` is emitted at most once per silent stretch. (AC 5.4.4)
- The bundled `Info.plist` contains both `NSMicrophoneUsageDescription` and `NSAudioCaptureUsageDescription`. (AC 5.4.3)

---

## Slice 5.5 — Watch the transcript live in the popover

**Key decision:** A hidden `popover` webview window is anchored to the tray icon with tauri-plugin-positioner and fed by the events 5.3 and 5.4 already emit. Left-click now opens the popover and right-click opens the menu. The React side is a thin view over a single `useRecordingFeed` hook.

**Structure**
- `tauri.conf.json`: window `{"label":"popover","url":"/#popover","visible":false,"decorations":false,"alwaysOnTop":true,"resizable":false,"skipTaskbar":true,"width":360,"height":440}` `[inferred: exact keys; tauri 2 window config]`.
- `apps/scribe-tray/src-tauri/src/popover.rs` (new):
  - `pub fn toggle<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()>` uses `move_window_constrained(Position::TrayCenter)` `[verified]`.
  - `pub fn install_blur_hide<R: Runtime>(window: &WebviewWindow<R>)` hides on `Focused(false)`, ignoring blur for 300 ms after show `[inferred]`.
- `tray.rs`: `.show_menu_on_left_click(false)`. `on_tray_icon_event` forwards every event to `tauri_plugin_positioner::on_tray_event` `[verified: required]` and toggles the popover on `Click { button: Left, button_state: Up }` `[verified]`. Capability `positioner:default`.
- Commands:
  - `#[tauri::command] fn recorder_snapshot(state) -> RecorderSnapshot { state: RecorderState, started_at: Option<String>, lines: Vec<LiveLine> }`, so a reopened popover can catch up.
  - `#[tauri::command] fn recent_sessions(limit: u8) -> Result<Vec<SessionSummary>, String>`, which uses the store read API added here as `TranscriptStore::list(&self, limit: usize) -> Result<Vec<SessionSummary>, StoreError>`. 5.7 reuses it.
  - `start_recording` and `stop_recording` commands that delegate to `RecordingController`.
  - `open_session(id)`: validates with `SessionId::parse`, then opens the file with the opener plugin.
- `crates/store-markdown/src/reader.rs` (new): `fn list_summaries(folder, limit) -> Result<Vec<SessionSummary>, StoreError>`. It reads headers only (stopping at the second `---`) and sorts by `started_at` descending. `SessionSummary { id, title, started_at, duration_seconds: Option<u64>, status }` lives in `scribe-core`.
- `apps/scribe-tray/src/popover/`:
  - `Popover.tsx`: an idle view or a recording view.
  - `StatusBar.tsx`: a pulsing dot (CSS animation disabled under `prefers-reduced-motion`), an elapsed timer and a Stop button.
  - `LiveTranscript.tsx`: an `ol` of lines keyed by `${speaker}-${startMs}` (a stable key per SonarJS `no-uniq-key`). The speaker is a text label. The partial line has class `partial` and `aria-hidden`. A visually hidden `aria-live="polite"` region announces finalised segments only. Auto-scroll is skipped when the user has scrolled up.
  - `RecentSessions.tsx`: a list of buttons.
  - `useRecordingFeed.ts`: `listen` to the `recording://*` events and reduce them with a pure `feedReducer(state, event)`.
- Event names (Rust `emit` → TS `listen`): `recording://started`, `recording://partial`, `recording://segment`, `recording://stopped`.

**Collaborators:** `TrayRecordingListener` (from 5.3) emits the events. The popover uses commands only. `feedReducer` is pure, and React components are presentational.

**Control flow:**
- Tray left-click → forward to positioner → toggle (show, position, focus | hide).
- On popover mount, `recorder_snapshot` + `recent_sessions(5)`, then subscribe to events.
- `partial` → replace the lane's partial line.
- `segment` → append a final line and clear that lane's partial.
- `stopped` → switch to the idle view and refresh recent sessions.
- Esc → hide.

**Tests:**
- New `apps/scribe-tray/src/popover/feedReducer.spec.ts`:
  - a partial line is replaced by the final wording;
  - partials of the two lanes are kept apart;
  - stopping clears the live lines.
  - Covers AC 5.5.3.
- New `Popover.spec.tsx` (`mockIPC` with `shouldMockEvents: true` `[verified]`):
  - idle shows Start and five recent sessions;
  - recording shows the timer and Stop;
  - Start is disabled while locked;
  - only finished lines are announced to screen readers;
  - the tab order runs from status to recent sessions;
  - no axe violations.
  - Covers AC 5.5.2, 5.5.3 and 5.5.5.
- New `StatusBar.spec.tsx`: the pulse animation is off when reduced motion is preferred (AC 5.5.4).
- New `crates/store-markdown/tests/session_listing.rs`: newest first, limit respected, duration from ended_at, recording sessions listed with their status.
- AC 5.5.1 (anchor, blur-hide, recording unaffected) is checked by hand.

**Standards that bite here:** WCAG 2.2 AA 1.4.1 (speaker not shown by colour alone), 2.1.1, 2.4.7, 4.1.3; SonarJS `no-uniq-key`, `prefer-read-only-props`, `jsx-no-leaked-render`; ESLint complexity 5; Clean Code CQS (the reducer is pure).

**Quality checks**
- `feedReducer` is a pure function with no Tauri imports.
- **The live region announces only finalised segments; partial text is `aria-hidden`.** (AC 5.5.5)
- List keys come from speaker plus start time, never the array index.
- `open_session` rejects ids that fail `SessionId::parse` before touching the filesystem.
- Hiding or showing the popover sends no command to the recorder. (AC 5.5.1)

---

## Slice 5.6 — Drop echoed lines from my side

**Key decision:** A pure `EchoFilter` (Decision 9) is consulted by `SegmentSequencer` when it releases a Me segment. Both are core-only, so the slice is pure logic plus tests, and Windows gets it for free.

**Structure**
- `crates/scribe-core/src/recording/echo_filter.rs` (new):
  - `pub struct EchoFilter` with `pub fn new(overlap_tolerance: Duration, similarity_threshold: f64) -> EchoFilter` and `pub fn is_echo(&self, mine: &Segment, theirs: &[Segment]) -> bool`.
  - Private: `fn normalise(text: &str) -> String` (lowercase, strip punctuation, collapse whitespace); `fn overlaps(a, b, tolerance) -> bool`; `fn is_similar(a, b) -> bool` (`strsim::normalized_levenshtein ≥ threshold`); `fn is_contained_run(mine, theirs) -> bool` (≥ 3 words, contiguous).
  - Constants: `ECHO_OVERLAP_TOLERANCE = 1 s`, `ECHO_SIMILARITY_THRESHOLD = 0.8`.
- `sequencer.rs` (modify): it holds the recently released Them segments for 3 s of lane time. On releasing a Me segment, it drops it if `echo_filter.is_echo(..)` is true; a dropped segment is never emitted. The hold rule from 5.4 stays, and a Me segment waits for the Them watermark ≥ `me.end + tolerance`, capped by `MAX_HOLD`.

**Collaborators:** The sequencer owns an `EchoFilter`. No adapter changes.

**Control flow:** Release Me segment → collect overlapping Them segments (held plus recent) → if `is_echo`, drop it; otherwise emit.

**Tests:**
- New `crates/scribe-core/tests/echo_filter.rs` (≤ 6-row tables, business names):
  - my line repeating their words at the same moment is dropped;
  - my line containing a run of their words is dropped;
  - my different words while they speak are kept;
  - identical words more than a second apart are kept;
  - a two-word overlap is not treated as echo.
  - Covers AC 5.6.1 and 5.6.2.
- Extend `segment_ordering.rs`:
  - an echoed line never reaches the transcript;
  - a my-side line is held no longer than the maximum hold.
  - Covers AC 5.6.1 and 5.6.4.
- New fixture test `echo_filter_headphones.rs`: a scripted headphones-only session (no overlapping text) drops zero segments (AC 5.6.3).

**Standards that bite here:** Clean Code (named predicates, cyclomatic ≤ 5, named constants); ZOMBIES; parameterised tests used sparingly.

**Quality checks**
- `EchoFilter` has no I/O and no clock; all time comes from segment offsets.
- **A Me segment is never released later than `MAX_HOLD` after its lane watermark passes its end.** (AC 5.6.4)
- Dropped segments produce no `SegmentFinalised` event. (AC 5.6.1)
- The thresholds are named constants, not inline literals.

---

## Slice 5.7 — Read saved transcripts from any AI app over MCP

**Key decision:** A new binary, `scribe-mcp`, built with rmcp 3.5 serves `list_transcripts` and `get_transcript` over size-capped stdio (Decision 7). It goes through a `TranscriptLibrary` in `scribe-core::library` that reads via new `TranscriptStore` read methods. The tray gains "Copy MCP config", which points at the binary bundled as a Tauri sidecar.

**Structure**
- `crates/scribe-core/src/ports.rs` (modify): add `fn read(&self, id: &SessionId) -> Result<SessionDocument, StoreError>` and `fn list_all(&self) -> Result<Vec<SessionSummary>, StoreError>` (`list(limit)` from 5.5 stays). `SessionDocument { header: SessionHeader, segments: Vec<Segment>, raw: String }`.
- `crates/scribe-core/src/library/`:
  - `mod.rs`: `pub struct TranscriptLibrary` with `pub fn new(store: Arc<dyn TranscriptStore>) -> TranscriptLibrary`.
  - `pub fn list(&self, query: ListQuery) -> Result<Vec<SessionSummary>, LibraryError>`, where `ListQuery { limit: Limit, since: Option<NaiveDate> }` and `Limit` is a newtype clamped to 1..=100 with default 20.
  - `pub fn get(&self, raw_id: &str) -> Result<SessionDocument, LibraryError>`.
  - Both exclude `SessionStatus::Recording`. `LibraryError::{InvalidId, NotFound(SessionId), Store(StoreError)}`.
- `crates/store-markdown/src/reader.rs` (extend):
  - `fn read_document(folder, id) -> Result<SessionDocument, StoreError>` and `fn parse_segment_line(line) -> Option<Segment>` (a private pure helper).
  - A malformed file → `StoreError::Malformed { file_name }`; `list_all` skips it and logs the file name only (AC 5.7.5).
- `apps/scribe-mcp/` (new binary): `Cargo.toml` with `rmcp = { version = "3.5", features = ["server","macros","transport-io"] }`, `tokio` (`macros`, `rt-multi-thread`, `io-std`), `tokio-util` (`codec`) and `futures` `[verified]`.
  - `src/server.rs`: `pub struct ScribeServer { library: TranscriptLibrary }` with `#[tool_router(server_handler)]` `[verified: tools-only shortcut]`.
    - `#[tool(description = "…")] async fn list_transcripts(&self, Parameters(req): Parameters<ListTranscriptsRequest>) -> Result<Json<TranscriptList>, String>`
    - `#[tool(description = "…")] async fn get_transcript(&self, Parameters(req): Parameters<GetTranscriptRequest>) -> Result<String, String>`, which returns the raw markdown.
    - A `String` error makes it a tool error with `isError: true` `[verified]`.
    - The error text for an unknown id is exactly `Transcript not found: <id>`.
  - `src/requests.rs`: `#[derive(Deserialize, JsonSchema)] struct ListTranscriptsRequest { limit: Option<u8>, since: Option<String> }` and `GetTranscriptRequest { id: String }`, plus `#[derive(Serialize, JsonSchema)] struct TranscriptList { transcripts: Vec<TranscriptSummaryDto> }`. Use the `rmcp::schemars` re-export `[verified]`.
  - `src/transport.rs`: `pub fn bounded_stdio() -> (impl Sink<…>, impl Stream<…>)`.
    - Built from `FramedRead::new(stdin(), JsonRpcMessageCodec::<RxJsonRpcMessage<RoleServer>>::new_with_max_length(MAX_MESSAGE_BYTES))`, mapped with `take_while(is_ok)` and then unwrap.
    - Paired with `FramedWrite::new(stdout(), JsonRpcMessageCodec::<TxJsonRpcMessage<RoleServer>>::default())` `[verified: codec + tuple IntoTransport; wiring inferred]`.
    - `MAX_MESSAGE_BYTES = 1_048_576`.
  - `src/main.rs`: `#[tokio::main] async fn main() -> anyhow::Result<()>` builds `TranscriptsFolder::default_location()`, `MarkdownTranscriptStore`, `TranscriptLibrary` and `ScribeServer`, then `.serve(bounded_stdio()).await?.waiting().await` `[verified]`. Logs go to stderr.
- Tray:
  - `tauri.conf.json` `bundle.externalBin: ["binaries/scribe-mcp"]` `[verified]`.
  - `apps/scribe-tray/scripts/stage-sidecar.(sh|ps1)` builds `scribe-mcp --release` and copies it to `src-tauri/binaries/scribe-mcp-$(rustc --print host-tuple)` `[verified: naming]`. It runs as `beforeBuildCommand`.
  - `mcp_config.rs` (new):
    - `pub fn bundled_mcp_path() -> Result<PathBuf, McpConfigError>`, which is `current_exe().parent()/scribe-mcp(.exe)` `[verified: resolution]`.
    - `pub fn config_snippet(path: &Path) -> String`, a pure function producing `{"mcpServers":{"scribe":{"command":"<path>"}}}` with JSON escaping via `serde_json`.
  - Menu `MENU_COPY_MCP_CONFIG = "copy-mcp-config"` → `clipboard().write_text` (`tauri-plugin-clipboard-manager` 2.5, capability `clipboard-manager:allow-write-text` `[verified]`) → a notification "MCP config copied".

**Collaborators:** `scribe-mcp → scribe-core::library → TranscriptStore (store-markdown)`. The tray uses only `mcp_config` and the clipboard. No `audio-*` or `stt-*` dependency in `scribe-mcp` (checked by `cargo tree`).

**Control flow:**
- **list:**
  1. Parse `since`; an invalid date is a tool error.
  2. Clamp the limit.
  3. `list_all` → filter out recording sessions → filter by `since` → sort newest first → take the limit.
- **get:**
  1. `SessionId::parse(raw)`; on error, tool error `Invalid transcript id: <raw>`.
  2. `store.read`; not found → `Transcript not found: <id>`.
  3. A status of recording is treated as not found.
  4. Return the raw text.
- **Transport:** an oversize line ends the stream, so the server shuts down cleanly with an error logged to stderr.

**Tests:**
- New `apps/scribe-mcp/tests/mcp_tools.rs`. Each test seeds a temp folder via `MarkdownTranscriptStore`, serves `ScribeServer` over `tokio::io::duplex(65_536)`, and connects with `().serve(client_side)` `[verified]`.
  - Listing:
    - an AI app sees saved calls newest first;
    - an AI app can limit how many calls it sees;
    - an AI app can list only calls since a date;
    - a call still being recorded is not listed.
  - Reading:
    - an AI app reads a full transcript including its header;
    - an unknown transcript returns not found;
    - an id that tries to leave the transcripts folder is rejected;
    - a corrupt transcript file is skipped rather than crashing the server.
  - Safety:
    - the transcripts folder is byte-identical after a full session of tool calls.
  - Covers AC 5.7.1–5.7.5 and invariant 4.6.
- New `apps/scribe-mcp/tests/oversize_message.rs`: a message over 1 MiB closes the connection without processing it (AC 5.7.6).
- New `apps/scribe-tray/src-tauri/tests/mcp_config.rs`: the snippet holds the absolute binary path; paths with spaces and backslashes are escaped correctly (AC 5.7.7).
- CI step (extend): `cargo tree -p scribe-mcp` contains no `cidre`, `cpal`, `wasapi`, `swift-rs` or `parakeet-rs`.

**Standards that bite here:** `standard-input` (size cap plus schema validation); Fail fast at boundaries; `slow-regex` (the id check uses the `regex` crate, which is linear-time, or a hand parser); `confidential-information-logging` (stderr logs ids only); Clean Code CQS.

**Quality checks**
- **`SessionId::parse` runs before any filesystem access in `get_transcript`.** (AC 5.7.3)
- `scribe-mcp` contains no code path that opens a file for writing. (AC 5.7.4, invariant 4.6)
- Sessions with status `recording` are excluded in `TranscriptLibrary`, not in the MCP layer, so 5.8 inherits the rule. (AC 5.7.2)
- The tool error text matches `Transcript not found: <id>` exactly. (AC 5.7.3)
- The stdin framing uses a bounded codec; no unbounded `read_until` remains on the server path. (AC 5.7.6)

---

## Slice 5.8 — Search saved transcripts over MCP

**Key decision:** This slice adds `TranscriptLibrary::search`, a literal, case-insensitive match over parsed segments (no regex), and exposes it as the third tool. It reuses 5.7's reader, its exclusion of recording sessions, and its test harness.

**Structure**
- `crates/scribe-core/src/library/search.rs` (new):
  - `pub struct SearchQuery(String)` with `pub fn parse(raw: &str) -> Result<SearchQuery, LibraryError>`. Empty or whitespace input is `LibraryError::EmptyQuery`; the query is trimmed and lowercased.
  - `pub struct SearchHit { pub id: SessionId, pub title: String, pub started_at: DateTime<FixedOffset>, pub matches: Vec<SegmentMatch> }` and `pub struct SegmentMatch { pub at: String /* hh:mm:ss */, pub speaker: Speaker, pub text: String }`.
  - `fn find_matches(doc: &SessionDocument, query: &SearchQuery) -> Vec<SegmentMatch>`, using `to_lowercase().contains`.
- `TranscriptLibrary::search(&self, query: &SearchQuery, limit: Limit) -> Result<Vec<SearchHit>, LibraryError>`: newest first, sessions with at least one match, limited.
- `apps/scribe-mcp/src/server.rs` (modify): `#[tool] async fn search_transcripts(&self, Parameters(req): Parameters<SearchTranscriptsRequest>) -> Result<Json<SearchResults>, String>`.

**Collaborators:** The same as 5.7.

**Control flow:** Parse the query (error if empty) → `list_all` minus recording sessions → newest first → read each document → find matches → keep non-empty → stop at the limit.

**Tests:**
- Extend `apps/scribe-mcp/tests/mcp_tools.rs`:
  - an AI app finds calls that mention a phrase, regardless of case;
  - search characters such as `.*` are matched literally;
  - an empty search is rejected;
  - a call still being recorded is not searched.
  - Covers AC 5.8.1 and 5.8.2.
- New `crates/scribe-core/tests/transcript_search_performance.rs` `#[ignore]`: 500 generated one-hour transcripts are searched in under 1 s in a release build. Run in CI nightly or by hand (AC 5.8.3).

**Standards that bite here:** `regular-expr` / `slow-regex` (no regex built from the query); ZOMBIES (zero hits, one, many, limit boundary).

**Quality checks**
- **No `Regex` is constructed anywhere from the search query.** (AC 5.8.1)
- Search reuses `TranscriptLibrary`'s exclusion of recording sessions, with no second implementation. (AC 5.8.2)
- Each match carries its timestamp, speaker and text exactly as stored.

---

## Slice 5.9 — Download the Windows speech model on first run

**Key decision:** `stt-parakeet` gains a `ModelProvisioner` that downloads a pinned manifest of Hugging Face files at fixed commits, resumes partial downloads with HTTP Range requests, and verifies each file's SHA-256 before an atomic rename (Decision 5). The tray maps its progress into `Readiness.model_ready` and into popover and tooltip state. This slice is **floored** (integrity of executable model artefacts).

**Structure**
- `crates/stt-parakeet/src/model_manifest.rs` (new): `pub struct ModelFile { pub name: &'static str, pub url: &'static str, pub sha256: &'static str, pub bytes: u64 }` and `pub const PARAKEET_MODEL: &[ModelFile]`. All values are `[verified]` from the HF API (2026-10-17):

  | name | repo @ revision | bytes | sha256 |
  |---|---|---|---|
  | `encoder-model.int8.onnx` | istupakov/parakeet-tdt-0.6b-v3-onnx @ 8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce | 652183999 | 6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09 |
  | `decoder_joint-model.int8.onnx` | same | 18202004 | eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70 |
  | `nemo128.onnx` | same | 139764 | a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f |
  | `vocab.txt` | same | 93939 | d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d |
  | `config.json` | same | 97 | 666903c76b9798caf2c210afd4f6cd60b08a8dbf9800ec8d7a3bc0d2148ac466 |
  | `silero_vad.onnx` (from `onnx/model.onnx`) | onnx-community/silero-vad @ e71cae966052b992a7eca6b17738916ce0eca4ec | 2243022 | a4a068cd6cf1ea8355b84327595838ca748ec29a25bc91fc82e6c299ccdc5808 |

  The URL form is `https://huggingface.co/<repo>/resolve/<revision>/<path>`.
- `crates/stt-parakeet/src/provisioner.rs` (new):
  - `pub struct ModelProvisioner` with `pub fn new(models_dir: PathBuf, fetcher: Box<dyn RangeFetcher>) -> ModelProvisioner`.
  - `pub fn is_ready(&self) -> bool`, which checks that all files exist with the expected size. Hashes are verified once at download time, and a `.verified` marker is written per file.
  - `pub fn provision(&self, progress: &dyn Fn(DownloadProgress)) -> Result<(), DownloadError>`.
  - `pub struct DownloadProgress { pub received_bytes: u64, pub total_bytes: u64 }`.
  - `DownloadError::{Network, HashMismatch { file }, Io}`.
- `crates/stt-parakeet/src/fetcher.rs` (new):
  - `pub trait RangeFetcher: Send { fn fetch_from(&self, url: &str, offset: u64, sink: &mut dyn FnMut(&[u8]) -> io::Result<()>) -> Result<FetchOutcome, DownloadError>; }`, where `FetchOutcome::{Resumed, Restarted}`.
  - `pub struct HttpsFetcher` uses blocking `reqwest` 0.13 with rustls, https only and certificate verification on by default `[verified]`, sending `Range: bytes=<offset>-`. A `206` response appends; a `200` response restarts.
- Models directory: `%LOCALAPPDATA%\Scribe\models\parakeet-tdt-0.6b-v3-int8\`, via `dirs::data_local_dir()`.
- Tray:
  - `model_setup.rs` (new, Windows-gated): at startup, `if !is_ready → spawn provision`. Progress is throttled to 1% steps and emitted as `ModelDownloadProgressed { percent }`. It also emits `ModelDownloadCompleted` (set `Readiness.model_ready`) and `ModelDownloadFailed { reason_kind }`.
  - Command `retry_model_download`.
  - Tooltip: `Scribe — downloading speech model 42%`.
- Popover: `ModelDownloadPanel.tsx` with a `progress` element and an accessible label, an `aria-live="polite"` status, and a failure state "Download failed — Retry" whose button is focusable.

**Collaborators:** `ModelProvisioner` takes a `RangeFetcher` (a fake in tests). The tray composes `HttpsFetcher`. `stt-parakeet` is the only crate with an HTTP dependency (5.3 CI check).

**Control flow:**
- For each file in the manifest:
  - If the file and its marker exist, skip it.
  - Otherwise:
    1. `part = <name>.part`; `offset = len(part)`.
    2. Hash the existing part into a SHA-256 hasher.
    3. `fetch_from(offset)`. `Restarted` → truncate and reset the hasher. Each chunk → write, update the hasher, report progress.
    4. At the end, compare the hex digest to the manifest. On a mismatch, delete the part and return `HashMismatch`. On a match, `sync`, rename, and write the marker.
- On any error, return it. The tray shows Retry; the next attempt resumes from the part file.

**Tests:**
- New `crates/stt-parakeet/tests/model_download.rs`, using `FakeFetcher` with in-memory bytes plus a matching or wrong hash:
  - a first launch downloads every model file and reports progress;
  - an interrupted download resumes from where it stopped;
  - a server that ignores the resume request restarts the file cleanly;
  - a corrupted file is deleted and reported as failed;
  - a ready model is not downloaded again.
  - Covers AC 5.9.1–5.9.4.
- New `crates/stt-parakeet/tests/model_manifest.rs`: every URL is https and pinned to a 40-character commit, and every hash is 64 hex characters (AC 5.9.1, `no-clear-text-protocols`).
- New `apps/scribe-tray/src/popover/ModelDownloadPanel.spec.tsx`: progress is announced, the failure offers Retry by keyboard, no axe violations (AC 5.9.5).

**Standards that bite here:** `unverified-certificate`, `no-clear-text-protocols`, `hashing` (SHA-256); Fail fast; Clean Code (the provisioner loop calls named steps); WCAG 4.1.3.

**Quality checks**
- **No file is renamed into place before its SHA-256 matches the manifest; a mismatch deletes the `.part` file.** (AC 5.9.2)
- `HttpsFetcher` has no option, flag or code path that disables TLS verification or allows `http://`.
- Each manifest URL contains a full commit hash, never `main`.
- `Readiness.model_ready` becomes true only on `ModelDownloadCompleted`. (AC 5.9.3)

---

## Slice 5.10 — Record a call on Windows

**Key decision:** This slice adds the Windows adapters. WASAPI loopback of the default output device becomes `Them`, cpal stays the mic for `Me`, and a `ParakeetTranscriber` cuts speech at pauses using Silero (Decision 5) and implements the same `Transcriber` port (Decision 1). It also adds the Windows sleep hook and microphone-denied handling. Core, store, popover and MCP are unchanged.

**Structure**
- `crates/audio-windows/src/` (new; Windows only):
  - `loopback.rs`: `pub struct LoopbackSource` implementing `AudioSource`.
    - A capture thread calls `wasapi::initialize_mta()`, then `DeviceEnumerator::new()?.get_default_device(&Direction::Render)?`, then gets the audio client, then `initialize_client(&WaveFormat::new(32, 32, &SampleType::Float, 16_000, 1, None), &Direction::Capture, &StreamMode::EventsShared { autoconvert: true, buffer_duration_hns: 1_000_000 })` `[verified: loopback = Capture on a Render device, shared mode only]`.
    - Then `get_audiocaptureclient`, the event handle and `start_stream`. It loops `wait_for_event(200)` → read into the deque → every 100 ms stamp an `AudioChunk` and send.
    - The audio-client getter and read method names are `[inferred]`; confirm on docs.rs wasapi 0.26.
  - `device_watch.rs`: `DeviceEventCallbacks::new()` + `set_default_device_callback`, then `enumerator.register_notification_callback`. Keep the registration alive. The callback only sends on a channel `[verified]`. The capture thread rebuilds the stream (backoff of 1, 2 and 4 s, as in 5.4).
- `audio-mic` (modify): on Windows a cpal build error whose HRESULT is `E_ACCESSDENIED (0x80070005)` maps to `CaptureError::PermissionDenied` `[inferred: surfaced through the cpal error message; confirm by blocking the mic in Settings]`.
- `crates/stt-parakeet/src/`:
  - `vad.rs`: `pub struct SileroVad` with `pub fn load(path: &Path) -> Result<SileroVad, TranscribeError>` and `pub fn speech_probability(&mut self, window: &[f32; 512]) -> f32`. It uses the `ort` re-exported by parakeet-rs, so there is only one `ort` version in the graph `[inferred: re-export path parakeet_rs::ort; if absent, depend on ort =2.0.0-rc.14 directly to match]`. It keeps the Silero recurrent state between calls.
  - `utterance_cutter.rs` (pure): `pub struct UtteranceCutter` with `pub fn new(config: CutterConfig) -> UtteranceCutter` and `pub fn feed(&mut self, chunk: &AudioChunk, speech: &[bool]) -> Vec<PendingUtterance>`.
    - `CutterConfig { speech_threshold: 0.5, end_silence: 600 ms, max_utterance: 15 s, pre_roll: 200 ms }`.
    - `PendingUtterance { start: Duration, samples: Vec<f32> }`.
  - `transcriber.rs`: `pub struct ParakeetTranscriber` implementing `Transcriber`.
    - `push` runs VAD per 512-sample window → cutter → for each pending utterance, `ParakeetTDT::transcribe_samples(samples, 16_000, 1, Some(TimestampMode::Sentences))` → `TranscriptEvent::Final(Utterance { start, end: start + len, text })` `[verified: API and signature]`. Empty text is skipped.
    - `finish` flushes the cutter.
    - It never emits `Partial` (Decision 5 trade-off).
  - `ParakeetTDT::from_pretrained(models_dir, None)` loads on CPU `[verified: default cpu feature]`. The file names must match parakeet-rs's expected names for the istupakov INT8 export `[inferred; confirm in the parakeet-rs source for TDT loading, and if the loader expects other names, rename in the manifest's local file names, not the URLs]`.
- Tray:
  - `lanes.rs` Windows branch: `[Me: MicSource + ParakeetTranscriber, Them: LoopbackSource + ParakeetTranscriber]`. The header has `engine: parakeet` and `language: auto`.
  - `power/windows.rs`: `on_will_sleep` via `windows::Win32::System::Power::PowerRegisterSuspendResumeNotification` with a `DEVICE_NOTIFY_CALLBACK` that handles `PBT_APMSUSPEND` `[verified: API exists; callback wiring inferred]`.
  - `notifications.rs`: `notify_microphone_blocked`, whose action opens `ms-settings:privacy-microphone` `[verified]`.

**Collaborators:** The Windows adapters depend on `scribe-core` and `audio-resample` (the mic still resamples). `ParakeetTranscriber` reads models from 5.9's directory. The Recorder, sequencer, echo filter, store, popover and MCP are reused untouched.

**Control flow:**
- **Start (Windows):** the mic and the loopback start → the lane workers push chunks to their `ParakeetTranscriber` → the VAD marks windows → the cutter closes an utterance after 600 ms of silence or at 15 s → TDT decodes → `Final` → the sequencer (with echo filter) → the file.
- **Device change:** rebuild loopback with backoff; the clock is unchanged.
- **Mic denied:** `PermissionDenied` before `store.create` → no file → notification (AC 5.10.5).
- **Sleep:** the power hook → `controller.stop()`.

**Tests:**
- New `crates/stt-parakeet/tests/utterance_cutting.rs` (pure, ZOMBIES):
  - silence produces no utterance;
  - one sentence followed by a pause produces one utterance with its start time;
  - a sixteen-second monologue is split at the maximum length;
  - a short pause inside a sentence does not split it;
  - flushing on stop emits the speech in progress.
- Extend `crates/scribe-core/tests/recording_session.rs`: a transcriber that only emits final segments still produces ordered two-lane transcripts (AC 5.10.1).
- `crates/stt-parakeet/tests/parakeet_smoke.rs` `#[ignore]`: an English and a German WAV fixture each produce non-empty text in the right language, and the time per 5 s utterance is under 1.4 s on the reference CPU (AC 5.10.3 and 5.10.6). Manual on Windows.
- `crates/audio-windows/tests/loopback_smoke.rs` `#[ignore]`: a played tone arrives as non-zero 16 kHz samples (AC 5.10.2).
- AC 5.10.4 and 5.10.5 are checked by hand per Verification.

**Standards that bite here:** Clean Code (the cutter is pure and separate from inference; SRP); Separation of concerns; no allocation or locks in the WASAPI wait loop beyond the deque; Fail fast (model-missing → `TranscribeError::ModelMissing`, mapped to `Locked`).

**Quality checks**
- **The dependency graph contains exactly one `ort` version.** (`cargo tree -d -i ort` is empty)
- `ParakeetTranscriber` never emits `TranscriptEvent::Partial`. (Decision 5)
- `UtteranceCutter` has no ONNX, clock or I/O dependency.
- No Windows-specific type or `cfg` appears in `scribe-core`, `store-markdown` or `scribe-mcp`.
- The header for Windows sessions is `engine: parakeet`, `language: auto`. (AC 5.10.1)
- An utterance never exceeds `max_utterance` (15 s) of audio. (Hot path: bounded decode time)

**Open at build time (AC 5.10.6):** whether a CPU-only reference machine (x64, 4 cores, 8 GB) decodes a 5 s utterance within about 1.4 s, so the total stays within 2 s. This can only be measured on that hardware. Probe: run `parakeet_smoke` in release mode. If it is too slow, enable parakeet-rs's `directml` feature `[verified: feature exists]` and re-measure. If it is still slow, lower `end_silence` to 450 ms and record the decision in the escalation log.

---

## Hard floors

- **Slice 5.9**, category **security** (it downloads executable model artefacts that need TLS and SHA-256 integrity verification). Floor tier `≥ excellent`. The reviewer runs the security and architecture dimensions unconditionally over `crates/stt-parakeet/src/{fetcher,provisioner,model_manifest}.rs`.

No slice touches auth, authorization, payment or migrations. The MCP input boundary (5.7) is covered by its own quality checks; it is not a designated floor category.

## Escalation log

- 2026-10-17 — slice 5.2 — correction (no tier change): `tauri-plugin-store` 2.5 requires Rust 1.90 > workspace MSRV 1.88; pinned 2.4 instead. The plan marked 2.5 `[verified]` from crates.io without checking its MSRV — version pins should be verified against the workspace `rust-version`.
- 2026-10-17 — slice 5.1 — correction: `tauri-plugin-opener` resolved to 2.7; ESLint pinned to 9 (jsx-a11y does not support 10).
- 2026-10-18 — slice 5.3 — probe outcome (AC 5.3.4): `SpeechAnalyzer.start(inputSequence:)` works on macOS 26.6.2 (25G83). `live_smoke` transcribed the fixture exactly ("hello, this is a short test of the scribe transcriber running on this mac."), so the streaming path stays and the `analyzeSequence(_:)` fallback was not needed (that API name is confirmed in the Speech framework interface).
- 2026-10-18 — slice 5.3 — corrections (no tier change): `serde-saphyr` 2.0 needs Rust 1.89, so it is pinned to 1.1 (picked by the MSRV-aware resolver); `tauri-plugin-notification` 2.5+ needs Rust 1.90, so 2.4 is pinned now (5.4 planned 2.7). cpal 0.18 has one `Error` with `ErrorKind::PermissionDenied` rather than per-call error enums, and macOS usually delivers silence instead of an error on a denied mic, so `audio-mic` checks `AVCaptureDevice` authorisation first. The Swift C ABI takes a session token on `push` and `finish` so each lane has its own analyzer. `unsafe_code` moved from `forbid` to `deny` so the three FFI modules can opt in locally. The Swift runtime rpath must also be set by the tray's build script. CI's macOS runner is pinned to `macos-26` because the bridge needs the macOS 26 SDK.
- 2026-10-18 — slice 5.5 — corrections (no tier change): `tauri-plugin-positioner` 2.4 needs Rust 1.90, so 2.3 (resolves to 2.3.4, MSRV 1.77) is pinned. Event names `recording://started|partial|segment|stopped` already existed in `TrayRecordingListener`, so they were reused. Blur-hide uses a pure `BlurGuard` (300 ms settle after show, plus ignoring the tray click that caused a blur-hide). Esc hides via a `hide_popover` command, so no `core:window` hide permission or `positioner:default` capability was needed. Recent sessions show in both views so the tab order matches AC 5.5.5.
- 2026-10-18 — slice 5.10 — corrections (no tier change): parakeet-rs 0.4.1 pins `ort =2.0.0-rc.14`, which needs Rust 1.92 (workspace MSRV 1.88), so `parakeet-rs =0.4.0` is pinned and `ort =2.0.0-rc.13` (MSRV 1.88) is a direct dependency. 0.4.0 has no `parakeet_rs::ort` re-export, so this is the plan's fallback; the graph has exactly one `ort` and CI now enforces it. The manifest's local file names already match the TDT loader (`encoder-model.int8.onnx`, `decoder_joint-model.int8.onnx`, `vocab.txt`; `nemo128.onnx` is unused). The Silero export is v5 (`input`, `state` [2,1,128], `sr` → `output`, `stateN`); the VAD prepends 64 context samples (576-sample input). cpal 0.18 surfaces a blocked Windows microphone as `ErrorKind::BackendError` with the message "… (os error -2147024891)", which is how `E_ACCESSDENIED` is detected. `TranscribeError::ModelMissing` was not added because the core is unchanged; a missing model surfaces as `EngineUnavailable`, and readiness keeps the recorder `Locked` until the download completes. `icons/icon.ico` was missing, which blocks every Windows tauri-build, so it was added. Probe outcome (AC 5.10.6, macOS M-series CPU only): 5.1 s English and 6.0 s German utterances decoded in 151 ms and 172 ms; the Windows reference-hardware timing is still a manual check. Finding: with finals delayed by end-of-utterance silence, a Them line that starts within about 600 ms of a Me line ending can be saved before it, because lane watermarks track audio fed, not finals emitted. Fixing that needs a port change (a transcriber-reported "finalised through" watermark), which is outside this slice.
