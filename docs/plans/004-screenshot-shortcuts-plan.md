# Plan: 004-screenshot-shortcuts

> Spec: [`docs/specs/004-screenshot-shortcuts.md`](../specs/004-screenshot-shortcuts.md) · Context: [`docs/specs/004-screenshot-shortcuts-context.md`](../specs/004-screenshot-shortcuts-context.md) · Authored: 2026-10-20
> Risk: MODERATE (new OS capture edge on two platforms, moves user data between folders, and MCP accepts a client-supplied name that resolves to a file; no regulated material) · Hard floors: slices 1, 3 · Fabric standards: reachable

---

# Part 1 — Review this

*Everything a reviewer needs to approve or redirect the approach. If you only read one part, read this one.*

## What we're doing

Two global shortcuts take a screenshot of the monitor under the pointer: one takes the whole monitor, the other opens a region selector that can also snap to a window. While Scribe is recording, the image is saved into the session's own folder as `HH-MM-SS.png`, its offset into the call, and copied to the clipboard. When nothing is recording, it is only copied. Claude reaches the images through MCP: `get_transcript` lists their names, and a new `get_screenshot` tool returns one scaled to 1568 px on the long edge. Both shortcuts can be rebound in Settings.

The constraint that shapes most of the design is that **Scribe is hexagonal and must build and test its core on Linux CI**. Screen capture is a new OS edge with two very different implementations. Every rule around it is the same on both systems: which monitor, where the image goes, what it is called, which rectangle is valid. So those rules live in `scribe-core` behind two new ports, with one adapter crate per OS and a thin tray shell. The second constraint is that **a capture must never wait on the recorder**. Stopping a recording holds the recorder lock for seconds, so a capture reads a small "live session" slot instead. The recorder fills that slot at start and clears it the moment stopping begins.

There is one deliberate exception to "everything through the core". The frozen frame reaches the region selector as raw pixels over Tauri's binary IPC, straight from tray memory, because the 500 ms budget leaves no room for an encode and decode on the way. A temp file is ruled out because frames must never touch disk.

## How it breaks down

Storage first, then one complete capture path, then everything that branches off it. Slice 1 gives every new session its own folder. Slice 2 lays the whole capture spine (ports, both OS adapters, the core capture flow, the shortcut, clipboard and notification) for the simplest case, a full-screen capture during a recording. Slices 3 to 8 each add one branch to that spine. The order is the spec's ([spec §5](../specs/004-screenshot-shortcuts.md#5-slices)).

| # | Slice | Delivers | Why this tier | Tier |
|---|-------|----------|---------------|------|
| 1 | Keep each new session in its own folder | New transcripts at `YYYY-MM/<id>/<id>.md`; three layouts read; session folders move whole | Rewrites every layout lookup and the user-data move; collision and permission rules must survive | `excellent` (floored) |
| 2 | Capture the full screen under the cursor into the recording | Both ports, macOS and Windows adapters, core capture flow, live-session slot, shortcut, save, clipboard, notification | New abstractions on two OSes with threading pitfalls; everything later builds on it | `best` |
| 3 | Let the AI client list and view a session's screenshots | `## Screenshots` section in `get_transcript`; `get_screenshot` scaled to 1568 px | Untrusted input resolves to a file; image scaling within a time budget | `excellent` (floored) |
| 4 | Guide the user to Screen Recording access on macOS | Access check before capture; system request; notification with Open Settings | Small, mirrors an existing notification exactly | `excellent` |
| 5 | Copy to the clipboard when nothing is recording | Clipboard-only path; toast; notice window with "Don't show this again" | New window and setting, both with close precedents | `excellent` |
| 6 | Select a region on the monitor under the cursor | Region shortcut, selector window above everything, frozen frame on a canvas, drag, size readout, cancel, crop | Native window levels, binary IPC, mixed-DPI mapping and focus under the accessory policy | `best` |
| 7 | Snap to a window in the region selector | Window outlines captured with the frame; hover highlight; click captures a window | Geometry over data slice 2 already captures | `excellent` |
| 8 | Rebind the screenshot shortcuts in Settings | Settings rows, listening mode, validation, live re-registration, persistence | Keyboard capture and Escape handling inside the popover | `excellent` |

Slice 2 is the big one on purpose. Splitting the macOS and Windows adapters into separate slices would leave a half-supported shortcut on one OS between slices. Slices 6 and 7 share the selector component; slice 7 only adds a mode to it.

## Decisions made

**1. Each new session lives in `<transcripts>/YYYY-MM/<id>/<id>.md`; lookups try session folder, then month file, then flat file.** The file keeps the id as its name ([spec §5.1](../specs/004-screenshot-shortcuts.md#51-keep-each-new-session-in-its-own-folder)). Older layouts are read in place and never reshuffled ([spec §10.4](../specs/004-screenshot-shortcuts.md#10-established-behaviour--documented-not-changed)). When the user moves the transcripts folder, a session folder moves whole, and older files keep today's behaviour of landing as month files. *Rejected: moving older files into session folders during a move, because §10.4 makes that the user's choice, not a side effect.*

**2. A capture reads a "live session" slot, never the recorder lock.** The recorder publishes the active session's id and clock into a shared slot when recording starts, and clears it as the first step of stopping, before any worker is joined. A capture therefore never stalls behind a stop, and "stopping" counts as not recording, as [AC 5.5.7](../specs/004-screenshot-shortcuts.md#55-copy-to-the-clipboard-when-nothing-is-recording) requires. The offset comes from the same `SessionClock` that stamps transcript lines, so `00-12-34.png` matches `[00:12:34]`. *Rejected: asking `RecordingController::state()`, which can block for seconds while a stop joins its workers; and wall-clock arithmetic, which drifts from the transcript stamps.*

**3. The capture rules live in a pure `scribe_core::capture` module behind two ports.** `ScreenCapturer` grabs the monitor under the pointer. `ScreenshotStore` saves, lists and serves a session's images. A `ScreenshotTaker` in the core ties a press to an outcome (saved, copied only, copied but not saved) using fakes in tests. The tray turns each outcome into clipboard writes and notifications. *Rejected: logic in the tray, because tray code that needs an `AppHandle` is untested by project convention ([context §6](../specs/004-screenshot-shortcuts-context.md#6-test-conventions)).*

**4. The destination is fixed at the press as a session id, not a path.** The store finds the session's folder when it saves. A recording that stops mid-selection still receives its screenshot ([invariant 4.2](../specs/004-screenshot-shortcuts.md#4-invariants)), and a session moved between press and save still gets it in its new place. *Rejected: resolving the folder path at the press, which would write into a folder that a move has just emptied.*

**5. Screenshots are written once with an exclusive create, never through a temp file.** The name comes from `offset_as_clock` with `:` replaced by `-`. A taken name moves on to `-2`, `-3` and so on, and a failed write removes its partial file. *Rejected: the `replace_atomically` temp-file pattern, which [AC 5.2.11](../specs/004-screenshot-shortcuts.md#52-capture-the-full-screen-under-the-cursor-into-the-recording) forbids.*

**6. macOS capture uses ScreenCaptureKit through `cidre`; Windows uses `xcap`.** On macOS: the pointer comes from a CoreGraphics event, the display from ScreenCaptureKit's display list, the native pixel size from the filter's content rectangle times its point-to-pixel scale, and RGBA by drawing into a bitmap context we own. Window outlines take their front-to-back order from the CoreGraphics on-screen window list and their frames, layers and owners from ScreenCaptureKit's typed window list. On Windows, `xcap` provides the monitor at the pointer, an RGBA capture and windows with a stacking order. *Rejected: `xcap` on macOS, because it calls `CGWindowListCreateImage`, which is obsoleted in the macOS 15 SDK ([spec §7.1](../specs/004-screenshot-shortcuts.md#71-tech-stack)).*

**7. Window placement units come from the adapter, as a category.** macOS reports the monitor in points and Windows in physical pixels, and the tray turns those into Tauri's logical or physical positions. Tauri's own cursor and monitor positions mix scale factors on macOS [verified in `tao` 0.37.1], so the selector is never placed from them. *Rejected: `AppHandle::cursor_position` with `monitor_from_point`, which misplaces the selector on a 1× monitor beside a 2× one.*

**8. Everything that crosses into the selector webview is in frame pixels.** The frame arrives as raw RGBA bytes through a binary IPC response and is painted onto a canvas the size of the frame, so no image encoding happens and no CSP change is needed. Window outlines arrive in frame pixels too. The webview converts pointer positions into frame pixels and returns the selection in frame pixels. Rust validates it before cropping. *Rejected: PNG or base64 transfer (encode and decode cost on a 5K frame) and a custom URI scheme (needs a CSP change for no gain).*

**9. On macOS the selector window is raised above the menu bar and full-screen apps.** It uses the screen-saver window level and can join every Space as a full-screen auxiliary, set on the native window through `objc2-app-kit`. Tauri's always-on-top stops at the floating level, below the menu bar [verified in `tao` 0.37.1]. *Rejected: Tauri's `always_on_top` and `visible_on_all_workspaces` alone, which fail [AC 5.6.3](../specs/004-screenshot-shortcuts.md#56-select-a-region-on-the-monitor-under-the-cursor).*

**10. Shortcuts are registered from Rust and stored as the hotkey library's canonical strings.** For example, `control+alt+meta+Digit1`. Validation, defaults and display labels are pure functions in the tray, so the frontend only reports what was pressed (`event.code` plus modifier flags) and shows the outcome Rust returns. The handler acts on key-down only. *Rejected: validating in TypeScript as well, which would duplicate the rules.*

**11. `get_screenshot` takes `<session id>/<name>.png`, parses it into typed parts before touching the disk, and serves any session, recording or not.** Scaling happens in `store-markdown` with a Lanczos filter, capped at 1568 px and never upscaled. Base64 encoding happens in `scribe-mcp`. *Rejected: refusing sessions still recording, as `get_transcript` does, because a screenshot is complete once written and the check would need an extra transcript read.*

**12. On macOS the access check runs before every capture.** When access is missing, Scribe asks macOS (which prompts the first time) and shows a notification whose text also says to quit and reopen Scribe if screenshots still fail after allowing access. That covers [spec §8.1](../specs/004-screenshot-shortcuts.md#8-open-questions) either way without hardware we don't have here.

## Where this could go wrong

- **A folder move that loses a screenshot or a transcript.** Folders cannot be hard-linked, and on POSIX renaming onto an existing empty folder silently succeeds. Guarded by slice 1's checks: the destination must be absent before a rename, a cross-volume move copies into a `.part` folder first, and the source goes only after the copy is complete.
- **The selector on the wrong monitor, or at the wrong scale, with mixed-DPI monitors.** Guarded by decisions 7 and 8 and slice 6's mixed-DPI check, which is a by-hand check because CI has no monitors.
- **A capture freezing the app.** This happens if anything waits on the recorder lock or on a ScreenCaptureKit completion from the main thread. Guarded by slice 2's checks: nothing in the capture path takes the recorder lock, and capture runs off the main thread with a bounded wait.
- **Path traversal through `get_screenshot`.** Guarded by slice 3's check that the reference is parsed into a `SessionId` and a `ScreenshotName` before any path is built.
- **Scope creep into nearby limits.** Window snapping that captures hidden window content ([§9.2](../specs/004-screenshot-shortcuts.md#9-deferred-follow-ups-and-accepted-limits)), a shortcut "off" switch ([§9.6](../specs/004-screenshot-shortcuts.md#9-deferred-follow-ups-and-accepted-limits)), and code to suppress the macOS monthly re-prompt ([§9.1](../specs/004-screenshot-shortcuts.md#9-deferred-follow-ups-and-accepted-limits)) will each look like an obvious improvement while slices 7, 8 and 4 are open. Each has an "is untouched" quality check.

## What's out of scope

From [spec §1](../specs/004-screenshot-shortcuts.md#1-overview--business-context): saving captures when nothing is recording, annotation, scrolling capture, video, regions spanning monitors, and full-resolution originals over MCP. From [spec §9](../specs/004-screenshot-shortcuts.md#9-deferred-follow-ups-and-accepted-limits): suppressing the macOS re-prompt (9.1), per-window capture of hidden content (9.2), keyboard-driven region selection (9.3), detecting shortcut conflicts on macOS (9.4), cross-monitor regions (9.5) and a shortcut off switch (9.6). Also out: migrating older transcripts into session folders ([§10.4](../specs/004-screenshot-shortcuts.md#10-established-behaviour--documented-not-changed)), and writing anything about screenshots into the `.md` file ([§10.1](../specs/004-screenshot-shortcuts.md#10-established-behaviour--documented-not-changed)).

---

# Part 2 — Build detail

*Prescriptive detail for the build agent. Reviewers can skim or skip.*

**Confidence markers used below:** `[verified]` = the planner read the code or docs and confirmed it. `[inferred]` = pattern-matched from a sibling implementation, not confirmed — the build agent should check before assuming.

## Shared conventions (all slices)

- **Architecture:** hexagonal per `docs/ARCHITECTURE.md` and [spec §7.2](../specs/004-screenshot-shortcuts.md#72-architectural-style). `scribe-core` gets the `capture` module and two ports, with no `cfg(target_os)`, no Tauri, no `image` crate and no filesystem access. `store-markdown` owns layout, PNG encode/decode and scaling. The two new adapter crates own OS capture. The tray is the composition root and owns shortcuts, windows, clipboard and notifications. Recording, transcription and the MCP read tools are otherwise unchanged.
- **Dependency direction:** `apps/*` → `scribe-core` ← adapters (`store-markdown`, `capture-macos`, `capture-windows`). `scribe-mcp` → `scribe-core` + `store-markdown` only, never a capture crate (CI greps `cargo tree`) [verified per context §4c].
- **Module map:**
  - `crates/scribe-core/src/capture/` (new) · Frame, PixelRect, Selection, MonitorShot, MonitorPlacement, WindowOutline, Destination, ScreenshotName, ScreenshotReference, ServedImage, ScreenshotTaker, outline normalisation · depends on `recording` (SessionId, LiveSession, offset_as_clock)
  - `crates/scribe-core/src/recording/live_session.rs` (new) · LiveSession, LiveSessionSlot · depends on SessionId, SessionClock
  - `crates/scribe-core/src/ports.rs` (modify) · `ScreenCapturer`, `ScreenshotStore`, `ScreenCaptureError`, new `StoreError` variants
  - `crates/scribe-core/src/library/` (modify) · screenshot listing and serving
  - `crates/scribe-core/src/testing.rs` (modify) · `FakeScreenCapturer`, `InMemoryScreenshotStore`
  - `crates/store-markdown/src/{month_layout,writer,relocation,private_file,transcripts_folder}.rs` (modify), `screenshot_files.rs` and `image_codec.rs` (new) · session layout, screenshot save/list/serve
  - `crates/capture-macos/` (new) · `ScreenCaptureKitCapturer` · `cidre` (`sc`, `cg`, `blocks`, `macos_15_0`)
  - `crates/capture-windows/` (new) · `DesktopCapturer` · `xcap` 0.9, `windows` 0.62
  - `apps/scribe-mcp/src/{server,requests,following_store,main}.rs` (modify), `screenshot_listing.rs` (new)
  - `apps/scribe-tray/src-tauri/src/screenshots/` (new, a domain folder like `claude_desktop/`) · shortcuts, capture shell, delivery, notices, region selector, idle notice, window level, bindings
  - `apps/scribe-tray/src/{idle-notice,region-selector}/` (new), `src/popover/` (modify) · webviews
- **Lexicon (from [context §4b](../specs/004-screenshot-shortcuts-context.md#4b-slice-lexicon) — do not rename):** `Frame`, `WindowOutline`, `Screenshot` (prose), `ScreenshotName`, `ScreenshotReference`, `ServedImage`, `ShortcutBinding`, `CaptureKind::{FullScreen, Region}`, `Destination::{IntoSession, ClipboardOnly}`, `ScreenCapturer`, `ScreenshotStore`, `path_for`, `existing_path`, `month_path`, `legacy_path`, `transcript_paths_in`, `session_folder_path`, `MAX_SERVED_EDGE_PX = 1568`, window labels `idle-notice` and `region-selector` with routes `index.html#idle-notice` and `index.html#region-selector`, settings keys `idle_notice_dismissed`, `screenshot_shortcut_full_screen`, `screenshot_shortcut_region`. Plan-introduced names: `LiveSession`, `LiveSessionSlot`, `ScreenshotTaker`, `PendingCapture`, `CaptureOutcome`, `CaptureFailure`, `MonitorShot`, `MonitorPlacement`, `PlacementUnits::{Points, Pixels}`, `PixelRect`, `Selection::{WholeMonitor, Area}`, `CaptureAccess::{Granted, NotGranted}`.
- **Tests:** Rust integration tests in `<crate>/tests/<topic>.rs` with behaviour-sentence names, AAA separated by blank lines, and `tempfile::tempdir()`. Core tests use the new fakes in `scribe_core::testing`. Tray tests cover `pub` pure functions only. Frontend tests are Vitest + testing-library + `vitest-axe`, co-located `*.spec.ts(x)`, with `mockIPC`. Timing budgets are `#[ignore = "timing budget: run by hand in release mode"]`. Hardware smoke tests are `#[ignore]`, following `audio-macos/tests/tap_smoke.rs`. Run order per `CLAUDE.md`.

---

## Slice 1 — Keep each new session in its own folder

**Key decision:** New recordings go to `<folder>/<YYYY-MM>/<id>/<id>.md`, and every lookup tries session folder → month file → flat (decision 1). Collision probing moves to the folder, and moving transcripts treats a session folder as one unit.

**Structure**

- `crates/store-markdown/src/month_layout.rs` (modify) — add `pub(crate) fn session_folder_path(folder: &Path, id: &SessionId) -> PathBuf` (`folder/<month>/<id>`) and `pub(crate) fn session_path(folder: &Path, id: &SessionId) -> PathBuf` (`session_folder_path/<id>.md`). `existing_path` keeps its signature and tries `session_path`, then `month_path`, then `legacy_path`, falling back to `session_path`. `transcripts_in_month_folder` also yields `<child>/<child>.md` for each child folder whose name passes `SessionId::parse` and whose transcript `is_file`. Add `pub(crate) fn session_folders_in(folder: &Path) -> Result<Vec<(SessionId, PathBuf)>, StoreError>`, returning session-folder paths for the mover.
- `crates/store-markdown/src/transcripts_folder.rs` (modify) — add `pub fn ensure_session_folder_exists(&self, id: &SessionId) -> Result<PathBuf, StoreError>`: month folder via the existing helper, then the session folder with `private_dir_builder` (0o700), mapping `AlreadyExists` to a distinct outcome the caller can probe on.
- `crates/store-markdown/src/writer.rs` (modify) — `try_create` creates the session folder for each `collision_candidates` id. A candidate is taken when its session folder already exists, **or** `month_path` or `legacy_path` already holds a file for that id. Create the transcript with `private_file::create_new` at `session_path`. `path_for` is unchanged and delegates to `existing_path`.
- `crates/store-markdown/src/relocation.rs` (modify) — `count_transcripts` counts session folders as one each, plus month and flat files. `move_transcripts` gains a second loop over `session_folders_in(from)`: `move_session_folder(to, id, source_folder) -> Option<MoveOutcome>`. That function ensures the destination month folder exists, keeps the folder in place when the destination path exists, and otherwise calls `fs::rename`. On `CrossesDevices`, it copies the folder recursively into `<dest>.part` (files `0o600`, folder `0o700`), renames `.part` to the destination, and only then removes the source. Month and flat files keep the existing `move_into_month` path.
- `apps/scribe-tray/src-tauri/src/commands.rs` — no change expected. `open_session` uses `path_for().is_file()` [verified per context §4].

**Collaborators:** `MarkdownTranscriptStore` calls the layout helpers and `TranscriptsFolder`. Nothing in `scribe-core` changes. Dependency direction is unchanged.

**Control flow:**
- `try_create`: for each candidate, skip it if a legacy or month file exists for it; try to create its session folder; if the folder already exists, try the next candidate; otherwise create the transcript file; if every candidate is taken, return `NoFreeSessionId`.
- `existing_path`: session transcript is a file → return it; month file is a file → return it; flat file is a file → return it; else return the session path, so `NotFound` maps as today.
- `move_session_folder`: destination exists → Kept; rename succeeds → Moved; cross-device → copy into `.part`, then rename `.part` into place, then remove the source → Moved; any other failure → leave the source, remove a stray `.part`, Kept.

**Tests:**
- Extend `store-markdown/tests/transcript_file.rs`: a new recording's transcript lands inside its own session folder (5.1.1); keep `a_second_call_started_in_the_same_second_gets_its_own_file` green; add "a new session never takes an id whose month-file transcript already exists".
- Extend `session_listing.rs` and `transcript_reading.rs`: flat, month-file and session-folder sessions all list and read (5.1.2); a stray non-id folder and a non-transcript file inside a session folder are ignored (5.1.5).
- Extend `month_folders.rs`: abandoned recordings in each layout are marked incomplete (5.1.3).
- Extend `moving_transcripts.rs`: a session folder with a PNG moves whole; a folder whose destination exists stays; a cross-volume move removes the source only after the copy, by simulating the error through the existing seam [inferred: reuse whatever `copy_across_volumes` tests use].
- Extend `no_audio_at_rest.rs` or `transcripts_folder.rs` under `#[cfg(unix)]`: session folder `0o700`, transcript `0o600` (5.1.6).
- `apps/scribe-mcp/tests/mcp_tools.rs`: unchanged; it compares against `store.path_for(..)` [verified per context §4].

**Standards that bite here:** Clean Code: small functions (a new function per layout case), fail fast at boundaries; Incubyte: ZOMBIES (zero, one or many sessions per month, collisions), business-language test names; `code-craft`: reuse `private_dir_builder` and `create_new`, no `fs::create_dir_all`.

**Quality checks**

- **No transcript or screenshot is ever lost in a move: the source is removed only after the destination is complete, and an existing destination is never overwritten** (AC 5.1.4).
- Lookup order is session → month → flat in `existing_path`, and no older file is moved or rewritten except by the user-confirmed move ([§10.4](../specs/004-screenshot-shortcuts.md#10-established-behaviour--documented-not-changed) unchanged).
- Discovery never recurses beyond `<month>/<id>/<id>.md`; one `read_dir` per month folder, none per session (AC 5.1.5; [context §7](../specs/004-screenshot-shortcuts-context.md#7-hot-paths-and-performance-landmines)).
- `append` still costs at most one extra `is_file` probe per call.
- No `fs::create_dir_all`; permissions are `0o700`/`0o600` (AC 5.1.6).

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.

---

## Slice 2 — Capture the full screen under the cursor into the recording

**Key decision:** This slice lays the whole capture spine (decisions 2–7 and 10) for the simplest case. The full-screen shortcut fires, the core `ScreenshotTaker` grabs the monitor under the pointer through `ScreenCapturer`, and fixes the destination from the live-session slot. It then saves through `ScreenshotStore` and returns an outcome that the tray turns into a clipboard write and one notification. In this slice the Clipboard-only branch exists in the core but is reached only by tests. Slice 5 adds its UI.

**Structure**

- `crates/scribe-core/src/recording/live_session.rs` (new) — `#[derive(Clone)] pub struct LiveSession { id: SessionId, clock: SessionClock }` with `pub fn id(&self) -> &SessionId` and `pub fn offset(&self) -> Duration`. `#[derive(Clone, Default)] pub struct LiveSessionSlot(Arc<RwLock<Option<LiveSession>>>)` with `pub fn current(&self) -> Option<LiveSession>`, `pub(crate) fn publish(&self, session: LiveSession)` and `pub(crate) fn clear(&self)`. Re-export from `recording/mod.rs`.
- `crates/scribe-core/src/recording/recorder.rs` (modify) — `Recorder` gains a `live: LiveSessionSlot` field and `pub fn live_session_slot(&self) -> LiveSessionSlot`. `record` publishes `LiveSession { id, clock: capturing.clock }` after `spawn_session`. `stop` calls `self.live.clear()` before `session.stop()`. `Recorder::new` keeps its signature.
- `apps/scribe-tray/src-tauri/src/recording.rs` (modify) — `RecordingController::new` takes the slot from the recorder before wrapping it in the `Mutex`, and gains `pub fn live_session(&self) -> Option<LiveSession>`, which never locks `recorder`.
- `crates/scribe-core/src/capture/mod.rs` (new) and its submodules:
  - `frame.rs` — `pub struct Frame { width: u32, height: u32, rgba: Vec<u8> }`. `pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Frame, InvalidFrame>` requires `rgba.len() == width*height*4` and non-zero sides. Add `width()`, `height()`, `rgba(&self) -> &[u8]`, `into_rgba(self) -> Vec<u8>` and `pub fn crop(&self, area: PixelRect) -> Result<Frame, InvalidSelection>`.
  - `geometry.rs` — `#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)] pub struct PixelRect { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }` (serde behind the existing `serde` feature [inferred: `scribe-core` has a `serde` feature, used by the tray]). `pub enum Selection { WholeMonitor, Area(PixelRect) }`.
  - `monitor.rs` — `pub struct MonitorShot { pub frame: Frame, pub outlines: Vec<WindowOutline>, pub placement: MonitorPlacement }`, `pub struct MonitorPlacement { pub x: i32, pub y: i32, pub width: u32, pub height: u32, pub units: PlacementUnits }`, `pub enum PlacementUnits { Points, Pixels }`, `pub struct WindowOutline { pub bounds: PixelRect }` (slice 7 fills outlines; this slice's adapters return an empty `Vec`).
  - `destination.rs` — `pub enum Destination { IntoSession { session: SessionId, offset: Duration }, ClipboardOnly }` with `pub fn at_press(live: Option<LiveSession>) -> Destination`.
  - `screenshot_name.rs` — `#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)] pub struct ScreenshotName { offset_seconds: u64, suffix: Option<u16> }` with `pub fn at(offset: Duration) -> ScreenshotName`, `pub fn candidates(offset: Duration) -> impl Iterator<Item = ScreenshotName>` (no suffix, then `-2..=99`), `pub fn parse(raw: &str) -> Result<ScreenshotName, InvalidScreenshotName>`, `pub fn offset(&self) -> Duration`, and `Display` → `HH-MM-SS[-n].png`. The digits come from `offset_as_clock` with `:` → `-` [verified: `offset_as_clock` floors seconds; hours may exceed two digits].
  - `taker.rs` — `pub struct ScreenshotTaker { capturer: Arc<dyn ScreenCapturer>, store: Arc<dyn ScreenshotStore> }` with `pub fn new(capturer, store) -> ScreenshotTaker`, `pub fn begin(&self, live: Option<LiveSession>) -> Result<PendingCapture, CaptureFailure>` and `pub fn finish(&self, pending: PendingCapture, selection: Selection) -> Result<CaptureOutcome, InvalidSelection>`. `pub struct PendingCapture { shot: MonitorShot, destination: Destination }` with accessors `shot()`, `destination()`. `pub enum CaptureOutcome { SavedIntoSession { screenshot: Frame, name: ScreenshotName }, CopiedOnly { screenshot: Frame }, CopiedButNotSaved { screenshot: Frame } }` with `pub fn screenshot(&self) -> &Frame`. `pub enum CaptureFailure { AccessNotGranted, CaptureFailed }` (the access branch is wired in slice 4).
  - errors: `InvalidFrame`, `InvalidSelection`, `InvalidScreenshotName` as `thiserror` unit structs.
- `crates/scribe-core/src/ports.rs` (modify) — `pub trait ScreenCapturer: Send + Sync { fn access(&self) -> CaptureAccess; fn request_access(&self); fn capture_under_pointer(&self) -> Result<MonitorShot, ScreenCaptureError>; }`, `pub enum CaptureAccess { Granted, NotGranted }`, `pub enum ScreenCaptureError { AccessNotGranted, NoMonitorUnderPointer, Failed(String) }`. `pub trait ScreenshotStore: Send + Sync { fn save(&self, session: &SessionId, offset: Duration, screenshot: &Frame) -> Result<ScreenshotName, StoreError>; fn list(&self, session: &SessionId) -> Result<Vec<ScreenshotName>, StoreError>; fn serve(&self, reference: &ScreenshotReference, max_long_edge: u32) -> Result<ServedImage, StoreError>; }` (`list` and `serve` are implemented in slice 3; in this slice the Markdown store returns `Ok(Vec::new())` and `Err(StoreError::ScreenshotNotFound{..})`). `ScreenshotReference` and `ServedImage` are declared in `crates/scribe-core/src/capture/reference.rs` (new) as data-only types, so the trait is complete; slice 3 adds parsing and `Display`. Add `StoreError::{NoFreeScreenshotName, ScreenshotNotFound { reference: ScreenshotReference }}`.
- `crates/scribe-core/src/testing.rs` (modify) — `FakeScreenCapturer` (scripted `MonitorShot` or error, settable access, counts `request_access` calls) and `InMemoryScreenshotStore` (records saves, scriptable save failure, `list` returns saved names).
- `crates/store-markdown/src/screenshot_files.rs` (new) + `image_codec.rs` (new) + `private_file.rs` (modify) — `impl ScreenshotStore for MarkdownTranscriptStore`: `save` resolves `session_folder_path` for the id under the current folder (a missing folder → `StoreError::Io`), PNG-encodes the frame with `image_codec::encode_png(&Frame, CompressionType::Default) -> Result<Vec<u8>, StoreError>` [verified: `image` 0.25.10 `PngEncoder::new_with_quality(w, CompressionType, FilterType)`], and writes each `ScreenshotName::candidates(offset)` with `private_file::create_new_bytes(path: &Path, bytes: &[u8]) -> io::Result<()>` (0o600, `create_new`; a failed write removes the file). `Cargo.toml`: `image = { version = "0.25", default-features = false, features = ["png"] }` [verified in lock].
- `crates/capture-macos/` (new crate, added to workspace members and `[workspace.dependencies]`) — `Cargo.toml` like `audio-macos`: `scribe-core` unconditional; under `cfg(target_os = "macos")`: `cidre = { version = "0.29", default-features = false, features = ["sc", "cg", "blocks", "macos_15_0"] }` [verified features exist]. `src/lib.rs` re-exports `ScreenCaptureKitCapturer` behind `#[cfg(target_os = "macos")]`. `src/capturer.rs`: `pub struct ScreenCaptureKitCapturer` with `pub fn new() -> ScreenCaptureKitCapturer` and `impl ScreenCapturer`. Helpers: `fn pointer_location() -> Option<cg::Point>` via `cg::Event::with_src(None)?.location()` (global top-left points) [verified]; `fn display_under(point) -> Result<(arc::R<sc::Display>, arc::R<sc::ShareableContent>), ScreenCaptureError>` by matching `sc::Display::frame()` [verified]; `fn capture_display(display, content) -> Result<Frame, ScreenCaptureError>` builds `sc::ContentFilter::with_display_excluding_windows(display, &empty)` [verified], sets `sc::StreamCfg` width and height to `filter.content_rect()` × `filter.point_pixel_scale()` [verified] with `set_shows_cursor(false)`, calls `sc::ScreenshotManager::capture_image_ch` with a `blocks::ResultCh::new2` closure [verified], and waits on a `sync_channel(1)` with `recv_timeout(Duration::from_secs(5))`, mirroring `audio-mic/src/permission.rs`. `fn rgba_of(image: &cg::Image) -> Result<Frame, ScreenCaptureError>` draws into `cg::Context::new_bitmap` over an owned `Vec<u8>` (8 bits, `width*4` bytes per row, device RGB, premultiplied-last; screen content is opaque) [verified signature], **inside the completion closure**, so only owned Rust data crosses threads. Placement = the display frame in points with `PlacementUnits::Points`. `access` maps `cg::screen_capture_access::preflight()` [verified]; `request_access` calls `request()`. Files that need `unsafe` take `#![allow(unsafe_code)]` per the `tap.rs` precedent.
- `crates/capture-windows/` (new crate) — under `cfg(windows)`: `xcap = "0.9"` and `windows = { version = "0.62", features = ["Win32_Foundation", "Win32_UI_WindowsAndMessaging"] }`. `pub struct DesktopCapturer` with `impl ScreenCapturer`: the pointer from `GetCursorPos` (`#![allow(unsafe_code)]`), the monitor from `xcap::Monitor::from_point(x, y)`, the frame from `monitor.capture_image()` (RGBA) [verified], and placement from `monitor.x()`, `y()`, `width()`, `height()` with `PlacementUnits::Pixels` [inferred: physical virtual-desktop pixels under Tauri's per-monitor DPI awareness; check by hand on a mixed-DPI setup]. `access` → `Granted`; `request_access` does nothing.
- `apps/scribe-tray/src-tauri/Cargo.toml` (modify) — `tauri-plugin-global-shortcut = "2"` (2.3.x and 2.4.x share `with_handler` and `on_shortcut`; resolver 3 picks an MSRV-compatible one) [verified API]; target deps `capture-macos` (macOS) and `capture-windows` (Windows).
- `apps/scribe-tray/src-tauri/src/screenshots/mod.rs` (new) — `pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()>` builds `ScreenshotTaker` from the per-OS capturer (`mod macos` / `mod windows` with the same `pub use` names, per `lanes.rs`) and the managed `Arc<MarkdownTranscriptStore>`, and manages `ScreenshotService`.
- `screenshots/capture.rs` (new) — `pub struct ScreenshotService { taker: ScreenshotTaker, busy: AtomicBool }` with `pub fn take_full_screen<R: Runtime>(&self, app: &AppHandle<R>, live: Option<LiveSession>)`. A `BusyClaim` guard mirrors `StartClaim` in `recording.rs`.
- `screenshots/delivery.rs` (new) — `pub fn deliver<R: Runtime>(app: &AppHandle<R>, outcome: &CaptureOutcome)` writes `app.clipboard().write_image(&tauri::image::Image::new(rgba, w, h))` [verified: `Clipboard::write_image` in 2.3.3], then shows the notice.
- `screenshots/notices.rs` (new, `pub`) — `pub fn notice_for(outcome: &CaptureOutcome) -> Notice` and `pub fn capture_failed_notice() -> Notice`, with `pub struct Notice { pub title: &'static str, pub body: String }` holding the exact AC strings. Shown through `notifications::show_plain`.
- `screenshots/shortcuts.rs` (new) — registers the plugin with `Builder::new().with_handler(..)`, acting only on `ShortcutState::Pressed` [verified]. Registers the full-screen default `control+alt+meta+Digit1` on macOS and `shift+control+alt+Digit1` on Windows [verified: `HotKey` parses `+`-separated, case-insensitive tokens; `meta`/`super`/`cmd` = ⌘]. A registration error shows one notification naming the shortcut (AC 5.2.10). The defaults and kind lookup live in `screenshots/bindings.rs` (`pub`), which slice 8 extends.
- `apps/scribe-tray/src-tauri/src/lib.rs` (modify) — add the plugin and call `screenshots::install` in `start_scribe` after `recording::install`.

**Collaborators:** `ScreenshotTaker` (core) ← `ScreenCapturer` (adapter) and `ScreenshotStore` (`store-markdown`). The tray constructs both and owns the plugin, clipboard and notifications. The core never sees Tauri, `cidre`, `xcap` or `image`.

**Control flow:**
- Shortcut handler: ignore unless Pressed; look up the kind for the shortcut; the full-screen kind spawns a worker thread that calls `take_full_screen`.
- `take_full_screen`: claim busy or return; read `live_session()` from the controller; `taker.begin(live)`; on `CaptureFailed` show the capture-failed notice and return; `taker.finish(pending, Selection::WholeMonitor)`; `deliver`.
- `ScreenshotTaker::begin`: `capturer.capture_under_pointer()` → `CaptureFailed` on any error; destination = `Destination::at_press(live)`; return `PendingCapture`.
- `ScreenshotTaker::finish`: crop for an `Area`, or take the whole frame; match the destination — `IntoSession` → `store.save(session, offset, &screenshot)`, Ok → `SavedIntoSession`, Err → `CopiedButNotSaved` with a log of the file name only; `ClipboardOnly` → `CopiedOnly`.
- macOS `capture_under_pointer`: pointer → display → filter and config → completion → RGBA → `MonitorShot`; a timeout or error → `Failed` with a non-pixel message.

**Tests:**
- New `scribe-core/tests/screenshot_naming.rs`: zero offset → `00-00-00.png`; 12m34s → `00-12-34.png`; the second candidate → `00-12-34-2.png`; past 99 hours keeps three digits; parse round-trips; parse rejects `..`, `/`, a missing `.png`, suffix `-1` and `-100` (5.2.5).
- New `scribe-core/tests/screenshot_capture.rs` (fakes): a capture while recording is saved into that session with the offset of the press (5.2.2, 5.2.5); a save failure still yields an image for the clipboard (5.2.8); a capture failure yields no outcome (5.2.9); nothing recording → copied only.
- Extend `scribe-core/tests/recording_session.rs`: the live session is visible while recording and is gone as soon as stopping begins.
- New `store-markdown/tests/screenshots.rs`: a saved screenshot lands in the session folder as a PNG of the same size (5.2.2, 5.2.4); a second save in the same second does not overwrite the first (5.2.5); the file is user-only and no other file appears in the folder (5.2.11); saving for a session whose folder was deleted fails (5.2.8).
- New tray `src-tauri/tests/screenshot_notices.rs` for `notice_for` strings (5.2.7–5.2.9).
- New `capture-macos/tests/capture_smoke.rs` (`#[ignore]`, hardware): a capture's pixel size equals the display's native size (5.2.3, 5.2.4).

**Standards that bite here:** `architecture-craft`: ports in core, adapters at the edge, no cycles; Clean Code: small functions, typed errors, fail fast (`Frame::new` validates); Incubyte: business-language names, ZOMBIES on naming; `code-craft`: construct with collaborators, no comments, reuse `offset_as_clock`, `private_dir_builder`, `show_plain`.

**Quality checks**

- **Nothing in the capture path takes the recorder `Mutex`; `live_session()` reads only the slot, and the slot is cleared before any worker join** (decision 2; AC 5.5.7).
- `scribe-core` has no `cfg(target_os)`, no Tauri, `cidre`, `xcap` or `image` imports, and builds and tests on Linux.
- The offset comes from `SessionClock::elapsed` via `LiveSession::offset`, never from wall-clock time (AC 5.2.5).
- No ScreenCaptureKit wait and no PNG encoding run on the main thread; the macOS wait is bounded.
- The pixel buffer is converted to owned RGBA inside the completion block; no `arc::R` crosses threads.
- The file name never overwrites: `create_new` per candidate; a failed write leaves no partial file (AC 5.2.5, 5.2.11).
- No temp file, and no log line with pixels or user-derived paths (AC 5.2.11; ARCHITECTURE §5).
- The handler acts on Pressed only; a second press while busy does nothing ([invariant 4.8](../specs/004-screenshot-shortcuts.md#4-invariants)).
- Notification strings match ACs 5.2.7–5.2.10 verbatim.
- `scribe-mcp`'s dependency tree contains no capture crate.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** In the running app on macOS and Windows, start a recording, put the pointer on a second monitor, press the full-screen shortcut: a PNG of that monitor at native size appears in the session folder, pastes into another app, and the notification shows the offset.

---

## Slice 3 — Let the AI client list and view a session's screenshots

**Key decision:** `get_transcript` appends a `## Screenshots` section of references only when a session has screenshots. `get_screenshot` parses `<id>/<name>.png` into typed parts and serves a PNG scaled to at most 1568 px (decision 11). Listing and serving live in `scribe-core::library` per ARCHITECTURE §8. Image work lives in `store-markdown`, so `scribe-mcp` gains only `base64`.

**Structure**

- `crates/scribe-core/src/capture/reference.rs` (modify) — `#[derive(Clone, PartialEq, Eq, Debug)] pub struct ScreenshotReference { session: SessionId, name: ScreenshotName }` with `pub fn new(session, name)`, `pub fn parse(raw: &str) -> Result<ScreenshotReference, InvalidScreenshotReference>` (split on exactly one `/`, `SessionId::parse` + `ScreenshotName::parse`), accessors, and `Display` → `<id>/<name>`. `pub struct ServedImage { pub png: Vec<u8>, pub width: u32, pub height: u32, pub original_width: u32, pub original_height: u32 }` with `pub fn was_scaled(&self) -> bool`. Both types were declared in slice 2 for the trait; this slice completes parsing and `Display`.
- `crates/scribe-core/src/library/mod.rs` + `error.rs` (modify) — `TranscriptLibrary::new(transcripts: Arc<dyn TranscriptStore>, screenshots: Arc<dyn ScreenshotStore>)`. `pub fn screenshots(&self, id: &SessionId) -> Result<Vec<ScreenshotReference>, LibraryError>` (sorted by `ScreenshotName` order). `pub fn screenshot(&self, raw_reference: &str, max_long_edge: u32) -> Result<ServedImage, LibraryError>`. `LibraryError::InvalidScreenshotName(String)` → `"Invalid screenshot name: {0}"` and `LibraryError::ScreenshotNotFound(String)` → `"No screenshot named {0}"` (AC 5.3.10). `StoreError::ScreenshotNotFound` and `StoreError::Malformed` map to `ScreenshotNotFound`.
- `crates/store-markdown/src/screenshot_files.rs` + `image_codec.rs` (modify) — `list`: one `read_dir` of `session_folder_path`; a missing folder → empty `Vec`; keep entries whose file name parses as `ScreenshotName`. `serve`: resolve `session_folder_path/<name>`; missing → `ScreenshotNotFound`; decode with `image::load_from_memory_with_format(.., ImageFormat::Png)`; `image_codec::fit_long_edge(width, height, max) -> (u32, u32)` (a pure function; unchanged when already within `max`; otherwise the long edge = `max` and the short edge rounded to keep the ratio); resize with `imageops::resize(.., FilterType::Lanczos3)` only when scaling [verified API]; encode with `CompressionType::Fast`. Reads only, writes nothing.
- `apps/scribe-mcp/src/following_store.rs` (modify) — `impl ScreenshotStore for SettingsFollowingStore`, forwarding each call after `follow_settings()`, mirroring the existing `TranscriptStore` impl [verified].
- `apps/scribe-mcp/src/main.rs` (modify) — one `Arc<SettingsFollowingStore>` passed as both trait objects to `TranscriptLibrary::new` [inferred: main builds the library; check its exact construction].
- `apps/scribe-mcp/src/screenshot_listing.rs` (new) — `pub fn screenshots_section(references: &[ScreenshotReference]) -> Option<String>` → `None` for empty; otherwise `\n## Screenshots\n\n<intro sentence from spec §5.3>\n\n- [HH:MM:SS] <reference>\n…`. Use `offset_as_clock(name.offset())` for the stamp. `pub const MAX_SERVED_EDGE_PX: u32 = 1568;` `pub fn served_label(reference: &ScreenshotReference, image: &ServedImage) -> String` → `"[00:12:34] <ref> — 1568 × 882 (scaled from 5120 × 2880)"`, or without the parenthesis when not scaled.
- `apps/scribe-mcp/src/requests.rs` (modify) — `pub struct GetScreenshotRequest { pub name: String }` with a `schemars` description naming the `<session id>/<HH-MM-SS>.png` shape.
- `apps/scribe-mcp/src/server.rs` (modify) — `get_transcript` appends `screenshots_section` to `document.raw` when it returns `Some`. New `#[tool] async fn get_screenshot(&self, Parameters(request)) -> Result<CallToolResult, String>` → `CallToolResult::success(vec![Content::text(label), Content::image(base64, "image/png")])` [verified: `rmcp` 3.5.1 `Content::image`, `CallToolResult::success`, and `IntoCallToolResult` for `Result<T, E>`]. Both descriptions mention "Scribe" and say what AC 5.3.12 requires. `Cargo.toml`: `base64 = "0.22"` [verified in lock].

**Collaborators:** server → `TranscriptLibrary` → `ScreenshotStore` (the `SettingsFollowingStore` decorator → `MarkdownTranscriptStore`). The MCP layer owns presentation (section text, label, base64); the store owns pixels.

**Control flow:**
- `get_transcript`: get the document (unchanged errors); list references for its id; a list error → log and treat as empty; a non-empty list → append the section.
- `get_screenshot`: parse the reference → invalid → tool error; serve with 1568 → not found or undecodable → `"No screenshot named <reference>"`; else return the label and the image.
- `serve`: file missing → not found; decode fails → malformed; fits → re-encode at the original size; else resize then encode.

**Tests:**
- New `scribe-core/tests/screenshot_reference.rs`: a listed name parses; `..`, a backslash, an absolute path, extra segments and a non-PNG name are rejected (5.3.9).
- Extend `scribe-core/tests/transcript_library.rs`: references come back in offset order; an unknown screenshot is reported as "No screenshot named …".
- Extend `store-markdown/tests/screenshots.rs`: a 5120 × 2880 screenshot is served at 1568 × 882 (5.3.6); a 1200 × 800 one is served unchanged (5.3.7); serving changes no file in the folder, compared through `folder_contents`-style snapshots (5.3.8); a corrupt PNG is not found (5.3.10); `fit_long_edge` boundary cases at exactly 1568 and at portrait orientation.
- New `scribe-mcp/tests/screenshots.rs` (the existing file is 548 lines): the section lists references in offset order (5.3.1); a session without screenshots is byte-identical to the file (5.3.2, 5.3.4); stray files are ignored (5.3.3); `get_screenshot` returns a label then a PNG image block (5.3.5); traversal names are tool errors and the server keeps answering (5.3.9, 5.3.10); a screenshot added after connecting is listed and served (5.3.13); an ignored timing test for 5K under one second (5.3.11).
- Extend `scribe-mcp/tests/mcp_tools.rs` `an_ai_app_discovers_…`: the exact tool list now includes `get_screenshot`; the "Scribe in every description" test covers it (5.3.12).

**Standards that bite here:** Clean Code: fail fast at boundaries (typed parse before any path join); spec [§6.1.1](../specs/004-screenshot-shortcuts.md#61-clean-code); `code-craft`: presentation in the MCP layer, pixels in the store; Incubyte: ZOMBIES on `fit_long_edge`.

**Quality checks**

- **A reference is parsed into `SessionId` + `ScreenshotName` before any path is built; no raw request text reaches a `Path::join`** (AC 5.3.9; floor: security).
- The `.md` file is never written by either tool, and the no-screenshot response equals `document.raw` byte for byte (AC 5.3.2, 5.3.4; [§10.1](../specs/004-screenshot-shortcuts.md#10-established-behaviour--documented-not-changed) unchanged).
- The served long edge is ≤ 1568, and smaller images are never upscaled (AC 5.3.6, 5.3.7; [§10.2](../specs/004-screenshot-shortcuts.md#10-established-behaviour--documented-not-changed) unchanged).
- `serve` writes nothing anywhere (AC 5.3.8).
- `get_transcript` decodes no PNG; listing is one `read_dir` ([context §7](../specs/004-screenshot-shortcuts-context.md#7-hot-paths-and-performance-landmines)).
- `scribe-mcp` gains no capture or `image` dependency; outgoing data is not routed through `MAX_MESSAGE_BYTES`.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** In Claude Desktop connected to the local build, ask for a call with screenshots: the transcript ends with the list, and asking to see one shows the image.

---

## Slice 4 — Guide the user to Screen Recording access on macOS

**Key decision:** `ScreenshotTaker::begin` checks `capturer.access()` first. `NotGranted` calls `request_access()` and returns `CaptureFailure::AccessNotGranted` without capturing, and the tray shows a notification with an Open Settings button (decision 12). Windows always reports `Granted`, so nothing changes there.

**Structure**

- `crates/scribe-core/src/capture/taker.rs` (modify) — `begin` gains the access guard before `capture_under_pointer`. Adapters returning `ScreenCaptureError::AccessNotGranted` mid-capture also map to `CaptureFailure::AccessNotGranted`.
- `apps/scribe-tray/src-tauri/src/notifications.rs` (modify) — macOS `system_settings` module: `pub fn notify_screen_recording_needed<R: Runtime>(app: &AppHandle<R>)`, mirroring `notify_cannot_hear_other_side` exactly (spawned thread, `MainButton::SingleAction("Open Settings")`, `opener().open_url`) [verified per context §4a]. URL `x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture` [inferred: sibling `Privacy_AudioCapture` verified in the repo]. Title "Scribe needs Screen Recording access to take screenshots"; body "Allow Scribe in Screen & System Audio Recording. If screenshots still fail after allowing it, quit and reopen Scribe." A Windows/other no-op twin keeps call sites free of `cfg`.
- `screenshots/capture.rs` (modify) — `AccessNotGranted` → `notify_screen_recording_needed`.

**Collaborators:** core `ScreenshotTaker` → `ScreenCapturer::access/request_access`; tray → notifications.

**Control flow:** `begin`: access not granted → request access → return `AccessNotGranted`; otherwise as before. Tray: `AccessNotGranted` → permission notification; `CaptureFailed` → capture-failed notification.

**Tests:** Extend `scribe-core/tests/screenshot_capture.rs`: without access, no capture is attempted, access is requested once, nothing is saved (5.4.1); recording state is untouched by a failed capture (5.4.3).

**Standards that bite here:** `code-craft`: mirror the existing notification, no new pattern; Clean Code: guard clause first.

**Quality checks**

- The access check runs before any capture call; a missing access writes no file and touches no clipboard (AC 5.4.1).
- `mac_notification_sys` is only ever called on a spawned thread.
- No code tries to suppress or pre-empt the monthly macOS re-prompt ([§9.1](../specs/004-screenshot-shortcuts.md#9-deferred-follow-ups-and-accepted-limits) untouched).
- Notification title and button text match AC 5.4.2 verbatim.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** On macOS with Scribe removed from Screen & System Audio Recording, press the shortcut: the system prompt appears the first time, the notification's Open Settings opens the right pane, and after allowing access (and relaunching if needed) the next press captures.

---

## Slice 5 — Copy to the clipboard when nothing is recording

**Key decision:** The `CopiedOnly` outcome from slice 2 now gets its UI: the toast "Screenshot copied to the clipboard" every time, plus, until dismissed, a small `idle-notice` window centred on the captured monitor. The preference is stored like `privacy_acknowledged`.

**Structure**

- `apps/scribe-tray/src-tauri/src/screenshots/idle_notice.rs` (new) — `const DISMISSED_KEY: &str = "idle_notice_dismissed"`, `const WINDOW_LABEL: &str = "idle-notice"`, `const WINDOW_ROUTE: &str = "index.html#idle-notice"`. `pub fn is_dismissed<R>(app) -> bool` and `pub fn show<R>(app, placement: &MonitorPlacement) -> tauri::Result<()>` mirror `privacy::{is_acknowledged, show_statement}`: refocus if open, else build at 420 × 220, centred within the placement, `always_on_top(true)`, `resizable(false)`, `focused(true)`. `#[tauri::command] pub async fn dismiss_idle_notice<R>(app, window, dont_show_again: bool) -> Result<(), String>` stores the flag when `true` and closes the window. `async` because a webview window must not be built inside a sync command on Windows [verified per context §4c].
- `screenshots/delivery.rs` (modify) — `CopiedOnly` → toast, then `idle_notice::show` on the main thread when not dismissed. The placement comes from `PendingCapture`, so `deliver` takes `(&CaptureOutcome, &MonitorPlacement)`.
- `screenshots/placement.rs` (new) — `pub fn centred_in(placement: &MonitorPlacement, size: (f64, f64)) -> tauri::Position` → `Logical` for `Points`, `Physical` for `Pixels` (decision 7). Slice 6 reuses `pub fn covering(placement) -> (Position, Size)`.
- `apps/scribe-tray/src-tauri/capabilities/default.json` (modify) — add `idle-notice` to `windows`.
- `apps/scribe-tray/src/idle-notice/IdleNotice.tsx` (new) — mirrors `PrivacyStatement.tsx`: `.window` with a title, body text from AC 5.5.3, a native `<input type="checkbox">` with a `<label>` "Don't show this again" (no shared component: it is the only checkbox, per YAGNI), and an OK `.button` focused in `useEffect`. Enter or Esc on the window invokes `dismiss_idle_notice` with the current checkbox value. `src/popover/commands.ts` gains `dismissIdleNotice(dontShowAgain: boolean)`, or a new `src/idle-notice/commands.ts` if the popover module should not grow [inferred: follow the existing per-feature placement].
- `apps/scribe-tray/src/main.tsx` (modify) — `"#idle-notice": IdleNotice` in `ROUTES`.
- `apps/scribe-tray/src-tauri/src/lib.rs` (modify) — register `dismiss_idle_notice`.

**Collaborators:** delivery → idle_notice → `tauri-plugin-store`; the webview → command. Core untouched.

**Control flow:** `CopiedOnly`: write the clipboard → toast → dismissed? stop : show notice. The notice: OK, Enter or Esc → dismiss with the checkbox value → store when ticked → close.

**Tests:**
- New `apps/scribe-tray/src/idle-notice/IdleNotice.spec.tsx`: OK has focus on open; Enter and Esc close; Tab reaches the checkbox and Space toggles it; closing with it ticked sends `dontShowAgain: true`, unticked sends `false`; `vitest-axe` finds no violations (5.5.3–5.5.6).
- Extend `scribe-core/tests/screenshot_capture.rs`: nothing recording → copied only and nothing saved (5.5.1); a stopping recording counts as not recording, using the live-session slot (5.5.7).
- Extend tray `screenshot_notices.rs`: the idle toast text (5.5.2).

**Standards that bite here:** Design Standards: WCAG 2.2 AA focus and contrast in context; reuse `.window`/`.button`; SonarJS complexity ≤ 5; `code-craft`: mirror `privacy.rs`.

**Quality checks**

- An idle capture writes nothing to disk ([§10.3](../specs/004-screenshot-shortcuts.md#10-established-behaviour--documented-not-changed) unchanged; AC 5.5.1).
- The toast shows on every idle capture, whether or not the notice is dismissed (AC 5.5.2, 5.5.4).
- The route is registered and the label is in capabilities, so the window never falls back to the privacy statement.
- The window is built off a sync command path.
- OK, not the checkbox, has focus on open (AC 5.5.6).

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** In the running app with nothing recording, press the full-screen shortcut: the image pastes elsewhere, the toast appears, and the notice opens on that monitor; tick "Don't show this again", relaunch, capture again: only the toast appears.
3. **Accessibility.** Same session: Tab moves between the checkbox and OK, focus is visible on both, and Enter, Esc and Space work as described.

---

## Slice 6 — Select a region on the monitor under the cursor

**Key decision:** The region shortcut calls `ScreenshotTaker::begin` immediately (freezing the frame), parks the `PendingCapture` in the service, and opens a `region-selector` window covering exactly that monitor (decision 7), raised above everything on macOS (decision 9). The webview pulls the frame as raw RGBA through a binary IPC response, paints it on a canvas, and returns a frame-pixel rectangle (decision 8). `finish(Selection::Area)` validates and crops it, and delivery is unchanged from slices 2 and 5.

**Structure**

- `crates/scribe-core/src/capture/frame.rs` (modify) — `crop` validates `width ≥ MIN_SELECTION_EDGE_PX` and `height ≥ MIN_SELECTION_EDGE_PX` (`pub const MIN_SELECTION_EDGE_PX: u32 = 4`), and that `x + width ≤ frame.width` and `y + height ≤ frame.height` with checked arithmetic; anything else is `InvalidSelection`.
- `apps/scribe-tray/src-tauri/src/screenshots/capture.rs` (modify) — `ScreenshotService` gains `pending: Mutex<Option<PendingCapture>>`. `pub fn begin_region<R>(&self, app, live)`: claim busy → `begin` → park the pending capture → open the selector on the main thread. `pub fn complete_region(&self, area: PixelRect) -> Result<CaptureOutcome, InvalidSelection>`: take the pending capture → `finish(Selection::Area(area))`; on `InvalidSelection`, put the pending capture back and return the error. `pub fn cancel_region(&self)`: drop the pending capture and release busy.
- `screenshots/region_selector.rs` (new) — `const WINDOW_LABEL: &str = "region-selector"`, `const WINDOW_ROUTE: &str = "index.html#region-selector"`. `pub fn open<R>(app, placement: &MonitorPlacement) -> tauri::Result<()>` builds a window at `placement::covering(placement)` with `decorations(false)`, `transparent(false)`, `resizable(false)`, `skip_taskbar(true)`, `always_on_top(true)`, `visible_on_all_workspaces(true)`, `shadow(false)` and `focused(true)`, then calls `window_level::raise_above_menu_bar(&window)` and `set_focus`. It installs its own close-on-blur via `on_window_event(WindowEvent::Focused(false))` → `cancel_region` + close, mirroring `popover::install_blur_hide` without reusing `BlurGuard` [verified per context §4c]. Commands: `#[tauri::command] pub fn region_frame(state) -> Result<tauri::ipc::Response, String>` returns `Response::new(rgba_bytes)` of the pending frame [verified: `tauri::ipc::Response` takes `Vec<u8>`] (copy the bytes; the pending capture keeps its frame for the crop). `#[tauri::command] pub fn region_context(state) -> Result<RegionContext, String>` returns `{ frame_width, frame_height, outlines: Vec<PixelRect> }` (empty until slice 7). `#[tauri::command] pub async fn complete_region<R>(app, window, area: PixelRect) -> Result<RegionCompletion, String>` returns `#[serde(tag = "outcome", rename_all = "snake_case")] enum RegionCompletion { Captured, TooSmall }`; `Captured` closes the window and calls `deliver` off the main thread. `#[tauri::command] pub fn cancel_region<R>(window)` closes and cancels.
- `screenshots/window_level.rs` (new) — `pub fn raise_above_menu_bar<R: Runtime>(window: &WebviewWindow<R>)`: on macOS (`#![allow(unsafe_code)]`), in `run_on_main_thread`, take `window.ns_window()` as an `NSWindow` and set the level to `NSScreenSaverWindowLevel` and the collection behaviour to `CanJoinAllSpaces | FullScreenAuxiliary | Stationary` [inferred: `objc2-app-kit` 0.3 `NSWindow::setLevel` and `setCollectionBehavior` behind the `NSWindow` feature; add the `NSWindow` and `NSResponder` features to the existing dependency]. On other OSes it does nothing; Windows' always-on-top already covers the taskbar for a borderless window [inferred; check by hand].
- `screenshots/shortcuts.rs` (modify) — register the region default `control+alt+meta+Digit2` on macOS and `shift+control+alt+Digit2` on Windows; the region kind → `begin_region`; both kinds are ignored while busy (invariant 4.8).
- `apps/scribe-tray/src-tauri/capabilities/default.json` (modify) — add `region-selector`. CSP unchanged: IPC already allowed by `connect-src ipc: http://ipc.localhost` [verified in `tauri.conf.json`], and canvas `putImageData` needs no `img-src`.
- `apps/scribe-tray/src/region-selector/RegionSelector.tsx` (new) — full-viewport `<canvas>` sized to `frame_width × frame_height` and CSS-stretched to `100vw × 100vh`; draws `ImageData` from the `region_frame` ArrayBuffer [inferred: `invoke` resolves a binary `Response` as an `ArrayBuffer`]. Dim overlay plus the selection drawn on a second canvas or SVG layer; crosshair cursor; `W × H` readout near the selection; hint "Drag to select · Esc to cancel"; a `.visually-hidden` `aria-live="assertive"` region announcing the AC 5.6.13 sentence on mount; a two-tone outline (1 px dark under 1 px light) for 3:1 contrast on any content. Esc and right-click → `cancel_region`; pointer up → `complete_region(area)`; `TooSmall` keeps the selector open.
- `apps/scribe-tray/src/region-selector/selection.ts` (new, pure) — `toFramePoint(clientX, clientY, viewport, frame) -> FramePoint`, `rectBetween(a, b) -> PixelRect` clamped to the frame, `isLargeEnough(rect) -> boolean` (≥ 4 × 4), `sizeLabel(rect) -> string` (`"1240 × 680"`).
- `apps/scribe-tray/src/region-selector/regionSelector.css` (new) — `body:has(.region-selector)` override of the `--surface` background ([context §5](../specs/004-screenshot-shortcuts-context.md#5-known-dragons)); tokens only.
- `apps/scribe-tray/src/main.tsx` (modify) — `"#region-selector": RegionSelector`.
- `apps/scribe-tray/src-tauri/src/lib.rs` (modify) — register the four commands.

**Collaborators:** shortcut → service → core `ScreenshotTaker`; service → region_selector → window_level; webview ↔ commands; completion → delivery. The pending frame lives only in tray memory.

**Control flow:**
- `begin_region`: busy → return; `begin` fails → notice (and the slice 4 access path) and release busy; else park and open.
- Webview mount: fetch context and frame → draw → announce. Pointer down → start; move → update rect and readout; up → too small ? keep : `complete_region`; Esc, right-click or blur → cancel.
- `complete_region`: no pending → error; invalid → put back, `TooSmall`; else close the window, release busy, deliver.

**Tests:**
- New `scribe-core/tests/region_selection.rs`: a selection inside the frame crops exactly those pixels at native size (5.6.5); one pixel past the edge is rejected (5.6.8); 3 × 3 is rejected and 4 × 4 accepted (5.6.7); overflowing coordinates are rejected.
- New `apps/scribe-tray/src/region-selector/selection.spec.ts`: a pointer at a 2× viewport maps to doubled frame pixels and at 1× maps one to one (5.6.11); a drag backwards normalises; clamping at the frame edge (5.6.8); size label formatting (5.6.4).
- New `apps/scribe-tray/src/region-selector/RegionSelector.spec.tsx` (`mockIPC`): it announces the instruction on mount (5.6.13); Esc and right-click cancel without completing (5.6.6); a small drag does not complete (5.6.7); a normal drag completes with frame-pixel coordinates (5.6.5); `vitest-axe` clean.
- Extend `scribe-core/tests/screenshot_capture.rs`: a region completed after the recording stopped still goes into that session at the offset of the press (invariant 4.2).
- An ignored timing note is not possible for the 500 ms budget in CI; it is a by-hand check (Verification 2).

**Standards that bite here:** Clean Code: fail fast on the selection in Rust; Design Standards: WCAG 2.2 AA, contrast in context (two-tone outline), announcement; SonarJS: complexity ≤ 5, `max-depth` 3 (keep pointer handlers small, geometry in `selection.ts`); `architecture-craft`: cropping stays in core.

**Quality checks**

- **The crop comes from the frame captured at the press; nothing re-captures after the selector opens** (AC 5.6.10; [§9.2](../specs/004-screenshot-shortcuts.md#9-deferred-follow-ups-and-accepted-limits) untouched).
- The frame never touches disk and never travels as base64 or PNG to the webview ([invariant 4.5](../specs/004-screenshot-shortcuts.md#4-invariants)).
- Rust rejects any rectangle outside the frame or under 4 × 4 before cropping (AC 5.6.7, 5.6.8).
- The selector covers exactly the captured monitor and is placed from the adapter's placement, never from Tauri's cursor position (AC 5.6.2, 5.6.11).
- Both shortcuts are ignored while a capture is pending (AC 5.6.9).
- Losing focus cancels and frees the pending frame (AC 5.6.6).
- No regions across monitors ([§9.5](../specs/004-screenshot-shortcuts.md#9-deferred-follow-ups-and-accepted-limits) untouched).
- `BlurGuard` and the popover blur handling are unchanged.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** On macOS with a full-screen app in front and on Windows, press the region shortcut on a 5K display and on a 1× external display: the frozen, dimmed frame covers that monitor, including the menu bar or taskbar, within 500 ms; drag shows the size; release pastes exactly the selection; Esc, right-click and `⌘Tab`/`Alt+Tab` cancel without a notification.
3. **Accessibility.** Same session: VoiceOver or Narrator reads the instruction on open; the outline is visible on both a white and a black area of the screen; Esc works without a mouse.

---

## Slice 7 — Snap to a window in the region selector

**Key decision:** Window outlines are captured with the frame in each adapter, normalised in the core into frame-pixel rectangles ordered front to back, and passed to the selector through `region_context`. The webview hit-tests on pointer move. A click without a drag sends the highlighted outline as the area, so Rust's validation and crop from slice 6 apply unchanged.

**Structure**

- `crates/scribe-core/src/capture/outlines.rs` (new) — `pub struct WindowCandidate { pub left: f64, pub top: f64, pub width: f64, pub height: f64, pub is_app_window: bool, pub is_own: bool }` in the adapter's placement units, front to back. `pub fn visible_outlines(candidates: &[WindowCandidate], placement: &MonitorPlacement, frame_width: u32, frame_height: u32) -> Vec<WindowOutline>` drops non-app and own windows, translates by the placement origin, scales by `frame_width / placement.width`, clips to the frame, drops empty rectangles, and keeps the order.
- `crates/capture-macos/src/outlines.rs` (new) — `fn window_candidates(content: &sc::ShareableContent, own_pid) -> Vec<WindowCandidate>`: the order from `cg::WindowList::new(ListOpt::ON_SCREEN_ONLY | ListOpt::EXCLUDE_DESKTOP_ELEMENTS, WINDOW_ID_NULL)` ids [verified]; metadata by matching `sc::Window::id()` → `frame()`, `window_layer()`, `owning_app()?.process_id()`, `is_on_screen()` [verified]; `is_app_window = layer == 0 && is_on_screen`; `is_own = pid == std::process::id()`. Called in `capture_under_pointer` with the same `ShareableContent` as the frame.
- `crates/capture-windows/src/outlines.rs` (new) — `xcap::Window::all()` [verified], skip minimized, `is_own` by `pid()`, `is_app_window = true`, bounds from `x()`, `y()`, `width()`, `height()`, sorted by `z()` descending [verified: `z` counts down from the topmost window in `EnumWindows` order].
- Both adapters fill `MonitorShot::outlines` via `visible_outlines`.
- `apps/scribe-tray/src-tauri/src/screenshots/region_selector.rs` (modify) — `region_context` returns the pending outlines.
- `apps/scribe-tray/src/region-selector/windowHitTest.ts` (new, pure) — `topmostOutlineAt(point: FramePoint, outlines: PixelRect[]) -> PixelRect | undefined` (first match in order).
- `RegionSelector.tsx` (modify) — hover state draws the outline and its `W × H`; a click without movement past `DRAG_THRESHOLD_PX = 4` (CSS px) → `complete_region(highlighted)` when there is one, else nothing; a drag past the threshold → free selection.

**Collaborators:** adapters → core `visible_outlines`; tray passes the outlines through; the webview hit-tests. No new command.

**Control flow:** Pointer move with no button → highlight = topmost outline under the point. Pointer down then up within the threshold → highlight ? complete with it : ignore. Pointer down then move past the threshold → drag mode (slice 6).

**Tests:**
- New `scribe-core/tests/window_outlines.rs`: own windows and non-app windows are never outlined (5.7.4); a window half off the monitor is clipped (5.7.2); points at 2× scale to doubled pixels; the front-to-back order is kept (5.7.1); a window wholly on another monitor is dropped (5.7.4).
- New `apps/scribe-tray/src/region-selector/windowHitTest.spec.ts`: the frontmost of two overlapping windows wins (5.7.1); a point on no window gives none (5.7.5).
- Extend `RegionSelector.spec.tsx`: hover shows the outline and size (5.7.1); a click captures the highlighted window's rectangle (5.7.2); a drag clears the highlight (5.7.3); a click on empty space does nothing (5.7.5); outlines come from the context fetched once, so later changes do not shift them (5.7.6).

**Standards that bite here:** `code-craft`: categorical filtering in a pure function; Clean Code: small functions; SonarJS complexity ≤ 5.

**Quality checks**

- Outlines and frame come from the same moment, the same `ShareableContent` on macOS (AC 5.7.6).
- Scribe's own windows, layer ≠ 0 windows, the desktop and the Dock or taskbar are never outlined (AC 5.7.4).
- A window capture is a crop of the frozen frame; no per-window capture API is called ([§9.2](../specs/004-screenshot-shortcuts.md#9-deferred-follow-ups-and-accepted-limits) untouched).
- The window rectangle goes through the same Rust validation as a dragged region.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** With overlapping windows on two monitors, open the selector: hovering outlines the front window with its size, a click pastes just that window, the Dock, menu bar and Scribe's popover are never outlined, and dragging still selects a free region.
3. **Accessibility.** Same session: the window outline is visible against light and dark windows.

---

## Slice 8 — Rebind the screenshot shortcuts in Settings

**Key decision:** A new `ScreenshotShortcuts` section in the Settings view lists both bindings with labels from Rust. **Change** enters listening mode, which pauses both global shortcuts, captures `event.code` plus modifiers, and sends them to `rebind_screenshot_shortcut`. Rust validates, registers and persists, and returns a typed outcome (decision 10). Escape cancels the rebind without hiding the popover.

**Structure**

- `apps/scribe-tray/src-tauri/src/screenshots/bindings.rs` (modify, `pub`, pure) — `#[derive(Clone, Copy, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] pub enum ShortcutKind { FullScreen, Region }` (the spec's `CaptureKind` stays in the core for capture semantics; this is the binding key). `pub struct ShortcutBinding { kind: ShortcutKind, keys: String }` with `pub fn default_for(kind: ShortcutKind, platform: Platform) -> ShortcutBinding`, `pub fn from_pressed(kind, pressed: &PressedKeys) -> Result<ShortcutBinding, BindingProblem>`, `pub fn from_stored(kind, raw: Option<&str>, platform) -> ShortcutBinding` (falls back to the default and logs a warning), `pub fn keys(&self) -> &str` (the canonical string), and `pub fn label(&self, platform: Platform) -> String` (macOS `⌃⌥⇧⌘` + key, Windows `Ctrl+Shift+Alt+` + key). `pub struct PressedKeys { pub code: String, pub control: bool, pub alt: bool, pub shift: bool, pub meta: bool }` (deserialised from the webview). `pub enum BindingProblem { NeedsModifier, DuplicateOf(ShortcutKind) }`. `pub enum Platform { MacOs, Windows }` with `pub fn current() -> Platform`, the one `cfg` point. The accepted key set is `Key[A-Z]`, `Digit[0-9]` and `F1`–`F12`; at least one of control, alt or meta. The canonical string follows `HotKey::into_string` order `shift+control+alt+meta+<Code>` [verified in `global-hotkey` source].
- `screenshots/shortcuts.rs` (modify) — `pub struct ShortcutRegistry { bindings: Mutex<Bindings> }` (managed) with `pub fn load<R>(app) -> ShortcutRegistry` reading `screenshot_shortcut_full_screen` and `screenshot_shortcut_region` via `from_stored`. `pub fn register_all<R>(&self, app) -> Vec<ShortcutKind>` returns the kinds the OS refused. `pub fn pause<R>(&self, app)` and `resume<R>(&self, app)` unregister and re-register both. `pub fn rebind<R>(&self, app, kind, pressed) -> RebindOutcome` validates, then checks for a duplicate against the other kind, unregisters the old binding, registers the new one, re-registers the old on failure, then persists and returns. `pub fn reset<R>(&self, app, kind) -> RebindOutcome` = rebind to the default. `pub fn kind_of(&self, shortcut: &Shortcut) -> Option<ShortcutKind>` for the handler. The slice 2/6 hard-coded defaults are replaced by the registry.
- `screenshots/commands.rs` (new) — `#[tauri::command] screenshot_shortcuts(state) -> Vec<ShortcutView>` (`{ kind, label }`); `pause_screenshot_shortcuts`; `resume_screenshot_shortcuts`; `rebind_screenshot_shortcut(kind, pressed: PressedKeys) -> RebindOutcome`; `reset_screenshot_shortcut(kind) -> RebindOutcome`. `#[serde(tag = "outcome", rename_all = "snake_case")] pub enum RebindOutcome { Accepted { label: String }, NeedsModifier, DuplicateOf { kind: ShortcutKind }, InUseByAnotherApp }`, mirroring `FolderChange`.
- `screenshots/notices.rs` (modify) — the unavailable-shortcut notice body gains "Change it in Scribe Settings" (AC 5.8.8).
- `apps/scribe-tray/src/popover/ScreenshotShortcuts.tsx` (new) — a section with the heading "Screenshot shortcuts" and one row per kind: the name, the label in a `<kbd>`, a **Change** `.actions__link` with `aria-label` "Change <row> shortcut, currently <label>", and a **Reset to default** `.actions__link`. Listening shows "Press the new shortcut…" and focuses the row. Outcome messages go to the view's existing `settings__status` line via a prop callback.
- `apps/scribe-tray/src/popover/useScreenshotShortcuts.ts` (new) — owns bindings, the listening row and status, mirroring `useTranscriptsFolder`. `startListening(kind)` → `pause_screenshot_shortcuts`; keydown handler (capture phase) → modifier-only keys ignored; Escape → `resume` + restore + `stopPropagation` + `preventDefault`; else `rebind` → message via `describeRebind(outcome)` → `resume` on anything but `Accepted`.
- `apps/scribe-tray/src/popover/shortcutCapture.ts` (new, pure) — `pressedKeysFrom(event: KeyboardEvent) -> PressedKeys | undefined` (reads `event.code`, never `event.key`), `isModifierOnly(code)`, `describeRebind(outcome, rowName) -> string` with the AC 5.8.3–5.8.5 strings.
- `apps/scribe-tray/src/popover/{types,guards,commands}.ts` (modify) — `ShortcutView`, `RebindOutcome` and guards `isRebindOutcome` and `isShortcutViewList`; command wrappers.
- `apps/scribe-tray/src/popover/SettingsView.tsx` (modify) — render `ScreenshotShortcuts` below `SettingsPanel`, sharing the status line. `SettingsPanel` props unchanged ([context §2](../specs/004-screenshot-shortcuts-context.md#2-patterns-to-follow)).
- `apps/scribe-tray/src/popover/Popover.tsx` (modify) — `useEscapeToHide` ignores an Escape whose event was already default-prevented, so the rebind's Escape does not hide the popover.
- `apps/scribe-tray/src-tauri/src/lib.rs` (modify) — manage `ShortcutRegistry` and register the five commands.

**Collaborators:** webview → commands → registry → plugin and `tauri-plugin-store`; the handler → registry `kind_of` → service. Bindings stay out of the core: they are a tray concern.

**Control flow:**
- `rebind`: invalid → `NeedsModifier`; same keys as the other kind → `DuplicateOf`; unregister the old; registering the new fails → re-register the old → `InUseByAnotherApp`; persist → `Accepted`.
- Launch: load with fallback → `register_all` → refused kinds → one notice each.
- Listening: pause → keydown → Escape ? cancel : rebind → resume unless accepted (an accepted rebind is already registered; the other kind is re-registered by `resume`).

**Tests:**
- New `apps/scribe-tray/src-tauri/tests/shortcut_bindings.rs`: defaults per platform (5.8.7); the region default is labelled `⌃⌥⌘2` on macOS and `Ctrl+Shift+Alt+2` on Windows (5.8.1); a shift-only combination needs a modifier (5.8.3); the other row's combination is a duplicate (5.8.4); a stored value that is missing, empty or unparseable falls back to the default (5.8.9); stored strings round-trip.
- New `apps/scribe-tray/src/popover/shortcutCapture.spec.ts`: ⌥ on macOS still yields `Digit2` from `code`; modifier-only presses are ignored; outcome messages match the ACs.
- New `apps/scribe-tray/src/popover/ScreenshotShortcuts.spec.tsx` (`mockIPC`): both rows render with labels (5.8.1); Change pauses shortcuts and shows the listening text (5.8.2); Escape restores and resumes and does not hide the popover (5.8.2); an accepted combination updates the label (5.8.6); a rejected one keeps the old label and announces the reason (5.8.3–5.8.5); Reset restores the default (5.8.7); Change is enabled while recording (5.8.10); accessible names and `vitest-axe` clean (5.8.11).
- Extend `Popover.spec.tsx`: an Escape already handled by Settings does not hide the popover.

**Standards that bite here:** Clean Code: one validation source (Rust); SonarJS: discriminated `RebindOutcome` union, complexity ≤ 5; Design Standards: reuse `.actions__link`/`settings__status`, WCAG 2.2 AA, focus stays on the row; `code-craft`: mirror `FolderChange` and `useTranscriptsFolder`.

**Quality checks**

- **A failed rebind leaves the old shortcut registered and working** (AC 5.8.5).
- Global shortcuts are paused for exactly the listening window and always resumed on cancel, failure or unmount (AC 5.8.2).
- The frontend reads `event.code` only and does not re-implement validation.
- A bad stored binding never prevents launch (AC 5.8.9).
- No "off" binding and no capability widening; no `global-shortcut:*` permission in `capabilities/default.json` ([§9.6](../specs/004-screenshot-shortcuts.md#9-deferred-follow-ups-and-accepted-limits) untouched).
- `SettingsPanel` props are unchanged, and the transcripts-folder flow behaves as before.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** In the running app, open Settings, change the region shortcut to `⌃⌥⌘R`, and use it at once; relaunch and it still works; try ⇧R alone and a combination owned by another app (Windows): each shows its message and the old one keeps working; Reset restores the default; Escape while listening keeps the popover open.
3. **Accessibility.** Same session: every Change and Reset control is reachable by Tab with visible focus; the screen reader reads the row name and current value; outcomes are announced.

**Open at build time (AC 5.8.5):** whether `register` reports a failure on macOS when another app holds the combination ([spec §8.2](../specs/004-screenshot-shortcuts.md#8-open-questions)). Only a machine with a second app holding the combination can answer it. Probe: register `⌃⌥⌘R` in another app (for example, a Shortcuts keyboard shortcut), then rebind to it in Scribe. If no error comes back, leave the code as is ([§9.4](../specs/004-screenshot-shortcuts.md#9-deferred-follow-ups-and-accepted-limits)) and note it in the slice hand-off.

---

## Hard floors

- **Slice 1** — migrations (data): it moves user transcripts and screenshots between folders and changes the on-disk layout. Floor tier `≥ excellent`.
- **Slice 3** — security: a client-supplied string resolves to a file on disk (path-traversal surface). Floor tier `≥ excellent`.

## Escalation log

- 2026-10-20 — slice 2 — supersedes Structure, Control flow — Structure: `ScreenshotStore::save(&self, session: &SessionId, offset: Duration, screenshot: &Frame)` and `Destination::IntoSession { session: SessionId, offset: Duration }` → new core type `pub struct SessionMoment { pub session: SessionId, pub offset: Duration }`, `ScreenshotStore::save(&self, moment: &SessionMoment, screenshot: &Frame)`, `Destination::IntoSession(SessionMoment)`, plus a new `StoreError::ScreenshotNotEncoded` variant for PNG encode failure. Control flow: `store.save(session, offset, &screenshot)` → `store.save(&moment, &screenshot)`. Reason: the planned four-argument port method fails the workspace clippy lint (`clippy.toml` sets `too-many-arguments-threshold = 3`, self counted), and an `#[allow]` on a port was judged worse. Floor: unchanged.
- 2026-10-20 — slice 2 — supersedes Structure — macOS full-screen default `control+alt+meta+Digit1` → `control+alt+super+Digit1`; canonical order `shift+control+alt+meta+<Code>` → `shift+control+alt+super+<Code>`; Windows default unchanged (`shift+control+alt+Digit1`). Reason: caught before build, a correction of a wrong [verified] marker. The planner checked the global-hotkey dev-branch source, not the 0.8.0 version the lockfile resolves, which accepts only `super`, `cmd` or `command` for ⌘ and serialises it as `super`. Floor: unchanged.
- 2026-10-20 — slice 6 — supersedes Structure — macOS region default `control+alt+meta+Digit2` → `control+alt+super+Digit2`; Windows default unchanged (`shift+control+alt+Digit2`). Reason: caught before build, a correction of a wrong [verified] marker; the global-hotkey 0.8.0 in the lockfile accepts `super`, `cmd` or `command` for ⌘, not `meta`. Floor: unchanged.
- 2026-10-20 — slice 8 — supersedes Structure — canonical string order in `ShortcutBinding::keys()` `shift+control+alt+meta+<Code>` → `shift+control+alt+super+<Code>`; the macOS defaults follow the slice 2 entry, Windows defaults unchanged. Reason: caught before build, a correction of a wrong [verified] marker; the lockfile-resolved global-hotkey 0.8.0 serialises ⌘ as `super`, not `meta`. Floor: unchanged.
