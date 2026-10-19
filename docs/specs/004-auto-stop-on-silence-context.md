# Task context: 004-auto-stop-on-silence

> Generated for plan 004 "auto-stop-on-silence" (standard tier, no spec) at HEAD ec0f01b7e101e8fce0c221be626d94fc049e43ad · 2026-10-19T16:29:33Z
> Read first by every agent in /anthara:plan-implementer's slice loop.

**No spec file exists**, so this file cites no spec anchors. The slices below follow the natural seams of the brief. If the plan slices the work differently, map onto these.
- **A** a pure core detector that says "nobody has talked on any lane for N" (default about 2 min).
- **B** wire the detector into the session so core announces it through `RecordingListener`.
- **C** the tray reacts: it stops that session by id and shows a notification.
- **D** (only if the plan asks for it) a user setting to turn it off or change N. YAGNI by default: a named constant is enough.

**De-facto style:** Hexagonal (ADR 1 in `docs/ARCHITECTURE.md`). Detection is domain logic and belongs in `scribe-core::recording`, so later MCP-scheduled recordings get it for free. Stopping, notifying and settings belong in `apps/scribe-tray`. ARCHITECTURE §7 rules apply: no `#[cfg(target_os)]` in core, no Tauri types below `apps/`, no shared mutable singletons for recorder state, and no audio on disk.

## 1. Affected modules
| Module / file | Role | Notes |
|---|---|---|
| `crates/scribe-core/src/recording/silence_watchdog.rs` | `SilenceWatchdog` (`new(threshold)`, `observe(&AudioChunk) -> WatchdogVerdict`), `WatchdogVerdict` (`Healthy`, `SilencedFor(Duration)`, `AlreadyReported`) | Per lane. Judges on **exact zeros** only, which means a dead capture, not a quiet room. It is the shape to copy, but not the signal to use (see 8) |
| `crates/scribe-core/src/recording/lane_worker.rs` | `run_lane` / `LaneRun` (one thread per lane), `LaneMessage` (`Final`, `Transcribing`, `Progress`, `Partial`, `Silenced`, `Ended`), `LaneProgress { speaker, finalised_through, heard_through }`, `SILENCE_WARNING_AFTER` | Every chunk goes to `transcriber.push`, then `Progress` is sent. Segments are rebased onto the session clock in `segment_from` |
| `crates/scribe-core/src/recording/writer.rs` | `SessionWriter` (single thread, `run_writer`), `handle(LaneMessage)`, `finish` → `store.finalise` + `RecordingEvent::RecordingStopped` | **The one place that sees every lane.** This is the natural home for a both-lanes detector |
| `crates/scribe-core/src/recording/recorder.rs` | `Recorder` (`start`, `record`, `stop`, `stop_session(&SessionId)`, `state`), `ActiveSession::stop` (stops sources, joins workers and writer), `CapturingLanes`, `Lane`, `Lane::is_required` (Me only) | The writer cannot stop sources, because `ActiveSession` owns them. A stop must come from whoever holds the `Recorder` |
| `crates/scribe-core/src/recording/events.rs` | `RecordingEvent` (`RecordingStarted`, `PartialSegmentUpdated`, `SegmentFinalised`, `RecordingStopped`, `CaptureSilenced`, `CaptureUnavailable`, `OutputDeviceChanged`) | A new variant must be added to the exhaustive `match` in `tray_listener.rs` (`TrayUpdater::apply`) |
| `crates/scribe-core/src/recording/audio.rs` | `AudioChunk { samples, captured_at }`, `ending_at`, `duration`, `ends_at`, `is_exact_silence`, `SAMPLE_RATE_HZ` (16 kHz) | `captured_at` = `SessionClock` elapsed, so it is **comparable across lanes** |
| `crates/scribe-core/src/recording/clock.rs` | `SessionClock` (`start`, `elapsed`), a real `Instant` | Cannot be faked. Tests drive time through chunk `captured_at` |
| `crates/scribe-core/src/recording/mod.rs` | Public re-exports | Export any new pub type here, as is done for `SilenceWatchdog`, `WatchdogVerdict` |
| `apps/scribe-tray/src-tauri/src/tray_listener.rs` | `TrayRecordingListener` (sends into an mpsc channel), `TrayUpdater` thread (`run` with a 1 s `TICK`, `apply`, `capture_silenced`, `stopped`) | `stopped()` already calls `RecordingController::stop_session(id)`. That is the existing "core ended it, tray joins it" path |
| `apps/scribe-tray/src-tauri/src/recording.rs` | `RecordingController` (`Mutex<Recorder>`; `start`, `stop`, `stop_session`, `state`), `publish_state`, `start_in_background`, `install`, `stop_before_sleep` | `stop_before_sleep` → `controller.stop()` is the precedent for a non-user stop trigger |
| `apps/scribe-tray/src-tauri/src/notifications.rs` | `show_plain(app, title, body)`, `TITLE`/`BODY` consts, per-OS `notify_*` | 252 lines. Clippy `too_many_lines = deny` is per function, but keep additions small |
| `apps/scribe-tray/src-tauri/src/other_side_alert.rs` | `Alert { title, body }` const, tested in `tests/other_side_alert.rs` | A testable home for notification copy |
| `apps/scribe-tray/src-tauri/src/privacy.rs` | `app.store(SETTINGS_FILE_NAME)`, `ACKNOWLEDGED_KEY`, `is_acknowledged` / `acknowledge` | The settings read/write pattern (slice D only) |

## 2. Patterns to follow
- **Stateful pure judge fed one observation at a time**: `watchdog.observe(&chunk) -> WatchdogVerdict`, report once, then re-arm when activity returns. Defined in `silence_watchdog.rs` (`SilenceWatchdog`). Used in `lane_worker.rs` (`LaneRun::watch_for_silence`).
- **Time is session-relative `Duration`**, never wall clock. Chunk `captured_at`/`ends_at`, `LaneProgress.heard_through`, and `Segment.start/end` all share one `SessionClock` origin. Wall clock appears only in `SessionHeader.started_at` and in `SessionWriter::ended_at`.
- **Core announces, the app reacts.** Core emits `RecordingEvent` through `Arc<dyn RecordingListener>` (`ports.rs`). The tray maps each event in `TrayUpdater::apply` to small methods (`capture_silenced`, `stopped`).
- **Stop by id, not "stop whatever is running"**: `Recorder::stop_session` / `RecordingController::stop_session` ignore a stale id, so a late signal cannot kill a newer session (tests `the_end_of_an_earlier_session_leaves_the_current_recording_running`, `the_current_session_can_be_stopped_by_its_id`).
- **Named constants for thresholds**, at the top of the module (`SILENCE_WARNING_AFTER`, `MAX_HOLD`, `ECHO_SIMILARITY_THRESHOLD`).
- **Speaker-specific behavior is a `match speaker`** in the app (`capture_silenced`), never an `if` on a label string.
- Advisory (ai-ergonomics): `testing.rs` is 394 lines. If it needs a new helper, keep the helper small rather than growing a new fake.

## 3. Reusable utilities
- `crates/scribe-core/src/testing.rs`: `chunk_at(Duration)` (100 ms of zeros), `FakeAudioSource::with_chunks`, `ScriptedTranscriber::{new, scripted, finalising_through, still_speaking_since}`, `finished(text, start, end)`, `partial(text)`, `InMemoryTranscriptStore` (`header`, `segments`), `RecordingListenerSpy` (`events`).
- `crates/scribe-core/tests/recording_session.rs` helpers: `Call::{ready, start, record}`, `chunks_between(from_ms, end_ms, sample)`, `lane_lasting`, `lane_hearing`, `stalled_lane`, `wait_until`, `utterance(from_ms, to_ms, text)`, `silence_warnings` (filter by event variant, so copy it for the new event).
- `apps/scribe-tray/src-tauri/src/notifications.rs` (`show_plain`). It logs failures by itself.
- `crates/store-markdown` `SETTINGS_FILE_NAME`, plus `tauri_plugin_store::StoreExt` (`app.store(..)`), as used in `privacy.rs`.

## 4. Adjacent code that must coordinate (per slice)
- **Slice A (detector):** `recording/mod.rs` (exports); `segment.rs` (`Speaker`, `Segment`).
- **Slice B (session wiring):** `writer.rs` (`SessionWriter::handle`, `advance`, `end_lane`); `lane_worker.rs` (`LaneMessage`, `LaneProgress`); `recorder.rs` (`Recorder::session_writer` builds `SessionWriter`; add any new field there); `events.rs`.
- **Slice C (tray reaction):** `tray_listener.rs` (`TrayUpdater::apply`, `stopped`); `recording.rs` (`RecordingController::stop_session`, `publish_state`); `notifications.rs`; the frontend `useRecordingFeed.ts` already handles `recording://stopped` (no change needed).
- **Slice D (setting, optional):** `privacy.rs` (store pattern); `recording.rs` (`install`, where the detector config would be read); `SettingsPanel.tsx` / `SettingsView.tsx` if the setting is user-facing.

## 4a. Slice imitation targets
- **Slice A:** mirror `silence_watchdog.rs` (`SilenceWatchdog::new/observe/judge`, `WatchdogVerdict`): threshold in `new`, `Option<Duration>` for "since", a `reported` flag, and an enum verdict. Its tests are in `tests/silence_watchdog.rs`.
- **Slice B:** mirror `lane_worker.rs` (`LaneRun::watch_for_silence`) for "observe, then on the reporting verdict emit once", and `writer.rs` (`SessionWriter::handle`, the `LaneMessage::Silenced` arm → `listener.on_event`) for emission.
- **Slice C:** mirror `tray_listener.rs` (`TrayUpdater::capture_silenced` + `stopped`) and `notifications.rs` (`notify_mcp_config_copied` → `show_plain`).
- **Slice D:** mirror `privacy.rs` (`is_acknowledged` / `acknowledge`, with a `*_KEY` const and `.unwrap_or(default)`).

## 4b. Slice lexicon
Existing names (do not rename or invent synonyms): `Speaker::Me` (mic, required lane) / `Speaker::Them` (system output); **lane** (`Lane`, `run_lane`, `open_lanes`); **chunk** (`AudioChunk`); **session** (`SessionId`, `ActiveSession`, `SessionWriter`); **segment** (`Segment`, finalised) vs **partial** (`PartialSegmentUpdated`); **heard_through** / **finalised_through**; **silenced** = exact-zero capture (`CaptureSilenced`). Spelling is British: `finalise`, `finalised`.
- **Slice A:** keep "quiet" distinct from "silenced". Proposed names: `QuietWatch`/`QuietVerdict` or similar, plus a threshold const like `AUTO_STOP_AFTER_QUIET = 120 s`. Pick one name and reuse it in B to D.
- **Slice B:** a new event variant such as `RecordingEvent::ConversationWentQuiet { id: SessionId }`. It should carry `id`, as `RecordingStopped` does.
- **Slice C:** `notify_stopped_after_quiet` (or similar) in `notifications.rs`. Copy wording in the style "Scribe stopped recording" / "No one has spoken for 2 minutes…".
- **Slice D:** settings key snake_case, like `privacy_acknowledged` and `TRANSCRIPTS_FOLDER_KEY`.

## 4c. Slice traps
- **Slice A:** (1) do NOT reuse `is_exact_silence` as "nobody talking". A mic in a quiet room is never exact zero (test `a_quiet_room_with_faint_sound_is_never_reported_as_silenced`). (2) do NOT use `Instant`/`SessionClock` inside the judge. Take `Duration` observations so tests stay deterministic. (3) report once, and re-arm only on new speech.
- **Slice B:** (1) do NOT drive time only from the Them lane's chunks. Sources skip empty batches (`samples.is_empty() ||` in each adapter's `forward_captured_audio`/`deliver`), so a silent WASAPI loopback can deliver nothing. Advance "now" from the max `heard_through` over all lanes (the Me mic pumps every 100 ms). (2) it must work with a single lane (Them `CaptureUnavailable`, or Them `Ended` after a transcriber failure). (3) do NOT make the writer stop sources or call `Recorder` (it has no handle, and that would be a cycle). Emit an event instead. (4) Parakeet (Windows) emits **no partials**, only finals after 600 ms end-silence, and at most 15 s per utterance. So "activity" must count `LaneMessage::Final` (and `Partial` where present). Count it on arrival in `handle`, not after `sequencer.release_ready`, which can hold a segment back up to `MAX_HOLD`. (5) do NOT add per-lane activity to `LaneMessage::Progress` semantics that `SegmentSequencer` relies on.
- **Slice C:** (1) stop via `stop_session(&id)`, never `stop()`, so a stale event cannot end a newer recording. (2) `stop_session` runs on the `TrayUpdater` thread and joins the writer. That is safe, because the writer only `send`s into an unbounded channel. But do not hold `LiveFeed` or other locks across it. The later `RecordingStopped` → `stopped()` → `stop_session` is then a no-op. (3) do NOT call `publish_state` or emit `recording://stopped` yourself. `stopped()` does it when `RecordingStopped` arrives. (4) the user's own Stop and the auto-stop can race. `Mutex<Recorder>` serialises them, and the loser sees `active == None` → `Ok(())`.
- **Slice D:** (1) do NOT read settings inside core. Pass a `Duration` or an `Option` in at construction (`Recorder::new` / `session_writer`). (2) do NOT add a global or static for the threshold (§7 singletons rule).

## 5. Known dragons
- `SessionWriter::ended_at` uses `SessionClock::elapsed()` at finish. An auto-stopped transcript's `ended_at` and duration will include the ~2 min of quiet, unless the plan decides to trim it (open decision, see 8).
- `ActiveSession::stop` joins every lane and the transcriber `finish()`. SpeechAnalyzer's `final_results_wait` can make the stop take seconds. Do not call it with the `RecordingController` mutex held from a UI-latency path beyond what `stop_recording` already does.
- macOS `notify_cannot_hear_other_side` uses `mac_notification_sys` on its own thread (actionable). A plain auto-stop notice should use `show_plain`, not this path.
- `lane_worker.rs` `watch_for_silence` reports only if the lane has **never** heard sound (`has_heard_sound`). That is a deliberate asymmetry. Do not "fix" it while touching the file.
- No `TODO`/`FIXME`/`LEGACY` markers in first-party Rust/TS.

## 6. Test conventions
- Core: integration-style tests in `crates/scribe-core/tests/*.rs` (no inline `#[cfg(test)]` in core). They use the `testing` feature, enabled through dev-dependency self-reference. Names are behavior sentences (`five_seconds_of_exact_zeros_are_reported_once`). Pure detector → a new `tests/<detector>.rs` like `tests/silence_watchdog.rs`. Wiring → add to `tests/recording_session.rs`: build timelines with `chunks_between(.., 0.2)` for sound and `utterance(..)` for speech, then `call.record(..)` and assert on `call.listener.events()`. Time is simulated by `captured_at`, so 2 minutes = 1,200 chunks of 100 ms, all synchronous (no sleeping).
- Tray: pure, `pub` modules are tested in `apps/scribe-tray/src-tauri/tests/*.rs` (for example `other_side_alert.rs`, `live_feed.rs`). Tauri-bound code (`tray_listener.rs`, `notifications.rs`) is not unit-tested. Keep logic out of it.
- Frontend: Vitest + `mockIPC` (`npm test` in `apps/scribe-tray`). Not needed unless D adds UI. Check the exit code (CLAUDE.md).
- Run: `apps/scribe-tray/scripts/stage-sidecar.sh` once, then `cargo test --workspace`, then `cargo clippy --workspace --all-targets -- -D warnings` (pedantic, `too_many_lines`/`cognitive_complexity` deny). Mutation: `cargo mutants --in-diff`. Watch threshold boundaries (`>=` vs `>`), because the watchdog test pins `>=`.

## 7. Hot paths and performance landmines
- `LaneRun::transcribe_until_closed` runs for every 100 ms chunk per lane. Do not add an O(samples) scan there if the signal can come from transcript events instead.
- `SessionWriter::handle` runs for every lane message (several per chunk). Keep the detector O(1) per message, with no allocation and no store calls.

## 8. Open uncertainties resolved during orientation
- **Signal choice:** core has no RMS, energy or level code. The only VAD is Silero inside `stt-parakeet` (`SileroVad`, `UtteranceCutter`, Windows only, adapter-private). SpeechAnalyzer exposes no VAD. The engine-agnostic signal observable in core is **transcript activity per lane**: `LaneMessage::Partial` / `LaneMessage::Final` reaching `SessionWriter::handle`. Recommended: "quiet" = no Partial or Final on any lane for N of session time, with time taken from `LaneProgress.heard_through`.
- **Where to stop:** core cannot stop itself (the writer has no access to sources). Emit an event, and let the tray call `stop_session(&id)`. That reuses the existing `stopped()` flow and the frontend needs no change.
- **Still open for the plan:** trim `ended_at` to the last speech, or keep the stop time? Does Me-lane echo of Them, or background TV, count as talking? (It will, under the transcript signal.) Is a setting needed now, or only the constant (D)?

## 8a. Design system (only relevant if slice D adds UI)
- **Source of truth:** CSS custom properties in `apps/scribe-tray/src/theme/tokens.css`, primitives in `theme/primitives.css`, and popover styles in `theme/popover.css`. There is no `design.md`.
- **Nearest screen:** `apps/scribe-tray/src/popover/SettingsPanel.tsx` (`SettingsPanel`, BEM `settings__*` classes, heading focus on mount, `isRecording` disables changes), tested in `SettingsPanel.spec.tsx` with `vitest-axe`.

---
## Changelog
- 2026-10-19: generated by code-orienteer for plan 004 (no spec) at HEAD ec0f01b.
