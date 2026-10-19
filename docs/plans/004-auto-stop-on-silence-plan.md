# Plan: 004-auto-stop-on-silence

> Spec: none — planned from prompt (standard tier) · Context: [`docs/specs/004-auto-stop-on-silence-context.md`](../specs/004-auto-stop-on-silence-context.md) · Authored: 2026-10-19
> Risk: MODERATE (a false stop cuts a meeting short; single-user local app, easy to revert) · Hard floors: none · Fabric standards: reachable

---

# Part 1 — Review this

*Everything a reviewer needs to approve or redirect the approach. If you only read one part, read this one.*

## What we're doing

Scribe stops a recording on its own when nobody has spoken on either lane (Me or Them) for 5 minutes, and shows a notification saying the recording was stopped automatically. The timer runs from the moment recording starts, so a recording started by mistake also ends itself. The 5 minutes is a fixed value, not a setting.

"Nobody has spoken" means **no transcript activity**: no partial and no finished segment on any lane. Core has no audio-level code, and the only voice detector is private to the Windows engine ([context §8](../specs/004-auto-stop-on-silence-context.md#8-open-uncertainties-resolved-during-orientation)). Transcript activity works the same for both engines and ignores fan hum or a quiet room. The existing `SilenceWatchdog` is not reused, because it detects a dead device (exact zeros), not a quiet conversation.

Core cannot stop a session itself: the writer that sees every lane has no access to the audio sources. So core only **announces** that the conversation went quiet, with the session id, and the tray stops that session by id and shows the notification. That reuses the existing stop path, so the popover and frontend need no change.

## How it breaks down

Pure detector first, then core wiring, then the tray reaction. Each slice builds and passes tests on its own; the feature becomes visible in slice 3.

| # | Slice | Delivers | Why this tier | Tier |
|---|-------|----------|---------------|------|
| 1 | Detect a quiet conversation | `QuietWatch`: says once when 5 minutes pass with no speech | Small pure type with boundary cases | `fast` |
| 2 | Announce it from the session | The session emits `ConversationWentQuiet { id }` once, using time from every lane | Threads and lane timing traps (silent loopback, single lane, Windows has no partials) | `excellent` |
| 3 | Stop the session and notify | Tray stops that session by id and shows "Recording auto-stopped" | Thin Tauri glue with a stop/stop race already handled by `stop_session` | `fast` |

## Decisions made

**1. "Quiet" means no transcript activity on any lane, not low audio level.** A `Partial` or `Final` lane message from either lane counts as speech. *Rejected: an RMS level threshold, because core has no level code, a threshold would need tuning per device, and background noise would keep recordings alive.* Consequence: a TV or other speech in the room counts as talking.

**2. Time comes from the lanes' `heard_through`, never the wall clock.** The watch keeps the latest `heard_through` it has seen from any lane, so a silent Windows loopback that delivers no chunks does not stall the timer; the microphone advances it every 100 ms. Speech is stamped at the latest time heard when it arrives. *Rejected: driving time from one lane's chunks, which stalls when that lane goes quiet ([context §4c](../specs/004-auto-stop-on-silence-context.md#4c-slice-traps)).*

**3. The timer starts at session start (time zero).** As the user chose, a recording where nobody ever speaks stops after 5 minutes too.

**4. The limit is a fixed constant, `AUTO_STOP_AFTER_QUIET` = 5 minutes, in core.** The user chose fixed. *Rejected: a popover setting (YAGNI).* The tray's notification text reads the minutes from the same constant, so the copy cannot drift.

**5. Core announces, the tray stops by id.** A new `RecordingEvent::ConversationWentQuiet { id }` is emitted once. The tray calls `RecordingController::stop_session(&id)`, which ignores an outdated id and is serialised with the user's own Stop by the recorder mutex. *Rejected: the writer stopping the session, because it has no handle on the sources and would create a cycle.*

**6. `QuietWatch` follows command-query separation instead of copying `SilenceWatchdog::observe`.** Commands record what was heard and that the report was sent; one query says whether a report is due. The org Clean Code standard forbids a method that both mutates and returns. *Rejected: copying `observe() -> Verdict` for symmetry.*

## Where this could go wrong

- **A real meeting cut short** because transcript activity stalls (for example the engine stops producing partials while someone talks). Guarded by slice 2's checks that both `Partial` and `Final` on either lane reset the timer.
- **The timer never firing when the other side's loopback goes silent.** Guarded by slice 2's check that a Me-only timeline still reaches the report.
- **A late event stopping a newer recording.** Guarded by slice 3's check that only `stop_session(&id)` is used, never `stop()`.
- **The event firing on every progress message after 5 minutes.** Guarded by slice 1's report-once check and slice 2's "exactly one event" test.
- **The transcript's end time includes the 5 quiet minutes.** Accepted; see out of scope.

## What's out of scope

A user setting for the limit or an on/off switch. Trimming the transcript's end time back to the last speech. Ignoring echo or a TV in the room. Stopping at a scheduled end time (comes with MCP scheduling later). Any popover or frontend change.

---

# Part 2 — Build detail

*Prescriptive detail for the build agent. Reviewers can skim or skip.*

**Confidence markers used below:** `[verified]` = the planner read the code or docs and confirmed it. `[inferred]` = pattern-matched from a sibling implementation, not confirmed — the build agent should check before assuming.

## Shared conventions (all slices)

- **Architecture:** hexagonal (ADR 1). Detection and the event live in `scribe-core::recording`; stopping and notifying live in `apps/scribe-tray`. No `#[cfg(target_os)]` in core, no Tauri types in core, no statics or singletons for the limit.
- **Dependency direction:** `apps/scribe-tray` → `scribe-core`. Core never calls the tray or `Recorder` from the writer.
- **Lexicon (from [context §4b](../specs/004-auto-stop-on-silence-context.md#4b-slice-lexicon) — do not rename):** `Speaker::Me` / `Speaker::Them`, lane, chunk, session, segment, partial, `heard_through`. New names, used verbatim across slices: `QuietWatch`, `AUTO_STOP_AFTER_QUIET`, `RecordingEvent::ConversationWentQuiet { id }`, `notify_recording_auto_stopped`. "Quiet" never means "silenced" (exact zeros).
- **Tests:** core tests live in `crates/scribe-core/tests/*.rs` (no inline `#[cfg(test)]`), simulated time via chunk `captured_at`, no sleeping, behaviour-sentence names ([context §6](../specs/004-auto-stop-on-silence-context.md#6-test-conventions)). Tray Tauri-bound code is not unit-tested by convention. Run `apps/scribe-tray/scripts/stage-sidecar.sh` once, then `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.

---

## Slice 1 — Detect a quiet conversation

**Key decision:** A pure `QuietWatch` holds the latest time heard, the time of the last speech (initially zero, per decision 3) and whether it has reported. It answers one query: is a report due? It follows decision 6 (CQS) and decision 2 (time from `heard_through`).

**Structure**

- `crates/scribe-core/src/recording/quiet_watch.rs` (new):
  - `pub const AUTO_STOP_AFTER_QUIET: Duration` = 5 minutes.
  - `pub struct QuietWatch { limit: Duration, heard_through: Duration, last_speech: Duration, reported: bool }`.
  - `pub fn new(limit: Duration) -> QuietWatch`
  - `pub fn hear_through(&mut self, heard_through: Duration)` — keeps the maximum seen, never moves backwards.
  - `pub fn hear_speech(&mut self)` — stamps speech at the current `heard_through` and re-arms the report.
  - `pub fn mark_reported(&mut self)`
  - `#[must_use] pub fn is_report_due(&self) -> bool`
- `crates/scribe-core/src/recording/mod.rs` (modify) — `mod quiet_watch;` and `pub use quiet_watch::{AUTO_STOP_AFTER_QUIET, QuietWatch};` beside the `SilenceWatchdog` export `[verified]`.

**Collaborators:** none. Depends only on `std::time::Duration`.

**Control flow:** `is_report_due` is true when not yet reported AND `heard_through - last_speech >= limit` (saturating). `hear_through` takes the max of stored and given. `hear_speech` sets `last_speech = heard_through` and `reported = false`.

**Tests:** new file `crates/scribe-core/tests/quiet_watch.rs`, unit level, mirroring `tests/silence_watchdog.rs` `[verified exists per context]`. Use a short limit (for example 10 s) in tests. Cases (ZOMBIES):
- a new watch has no report due at time zero;
- a report is due once quiet lasts exactly the limit (pins `>=`);
- a report is not due one millisecond before the limit;
- speech resets the quiet period, so the report comes a full limit after the last speech;
- after `mark_reported`, no report is due while quiet continues;
- speech after a report re-arms it;
- an earlier `heard_through` from a lagging lane does not move time backwards.

**Standards that bite here:** Clean Code: command-query separation, intent-revealing names, searchable named constant (no magic `300`), small functions, YAGNI (no `Speaker` parameter: any lane counts the same). Incubyte: test names in business language, ZOMBIES boundaries.

**Quality checks**

- **Boundary pinned: a report is due at exactly `limit` of quiet and not before.**
- No method both mutates and returns a value.
- `QuietWatch` uses no `Instant`, `SessionClock` or wall clock.
- The 5-minute value appears once, as `AUTO_STOP_AFTER_QUIET`.

---

## Slice 2 — Announce it from the session

**Key decision:** `SessionWriter`, the one place that sees every lane, owns a `QuietWatch`. `Partial` and `Final` messages count as speech when they arrive (before the sequencer can hold a segment back); `Progress` advances time and, when a report is due, emits `ConversationWentQuiet { id }` once. Decisions 1, 2 and 5.

**Structure**

- `crates/scribe-core/src/recording/events.rs` (modify) — add `ConversationWentQuiet { id: SessionId }` to `RecordingEvent` `[verified]`.
- `crates/scribe-core/src/recording/writer.rs` (modify) — `SessionWriter` gains `pub(crate) quiet_watch: QuietWatch`. New private methods:
  - `fn hear_speech(&mut self)` — forwards to the watch.
  - `fn watch_for_quiet(&mut self, heard_through: Duration)` — advances the watch and announces when due.
  - `fn announce_quiet(&mut self)` — emits the event and marks it reported.
- `crates/scribe-core/src/recording/recorder.rs` (modify) — `Recorder::session_writer` sets `quiet_watch: QuietWatch::new(AUTO_STOP_AFTER_QUIET)` `[verified: the struct literal is built there]`.
- `apps/scribe-tray/src-tauri/src/tray_listener.rs` (modify) — add a temporary arm for the new variant to keep the exhaustive `match` in `TrayUpdater::apply` compiling; slice 3 gives it behaviour. The arm logs at info level and does nothing else.

**Collaborators:** `SessionWriter` calls `QuietWatch` and `self.listener.on_event`. The writer never touches `Recorder` or sources.

**Control flow:** in `SessionWriter::handle`:
- `Final(segment)` → `hear_speech`, then `sequencer.accept(segment)`.
- `Partial(speaker, text)` → `hear_speech`, then emit `PartialSegmentUpdated` as today.
- `Progress(progress)` → `advance(&progress)` as today, then `watch_for_quiet(progress.heard_through)`.
- other arms unchanged.

`watch_for_quiet`: `quiet_watch.hear_through(..)`; guard: if `quiet_watch.is_report_due()` then `announce_quiet`. Keep `handle` under the clippy `too_many_lines` and complexity limits by keeping each arm a single call.

**Tests:** extend `crates/scribe-core/tests/recording_session.rs`, integration level, using `Call`, `lane_hearing`, `chunks_between`, `utterance`, and a new filter helper `quiet_announcements(events)` copied from `silence_warnings` `[verified]`. Five minutes = 3,000 chunks of 100 ms. `ScriptedTranscriber::new` takes one reply per push `[inferred — check `ScriptedTranscriber::push` in `testing.rs`]`; build long reply lists with a small named helper, not loops in the test body. Cases:
- a call where nobody speaks for five minutes is announced as gone quiet once;
- a call shorter than five minutes is never announced;
- a final segment on the other side's lane resets the quiet period, so no announcement happens within five minutes of it;
- a partial on the microphone lane resets the quiet period;
- a call with only the microphone lane still gets announced (Them lane ended or unavailable);
- the announcement carries the session's id.

**Standards that bite here:** Clean Code: separation of concerns (detection in the watch, emission in the writer), small functions, no side effects beyond the name. ARCHITECTURE §7: no singletons, no `cfg(target_os)`. Hot path ([context §7](../specs/004-auto-stop-on-silence-context.md#7-hot-paths-and-performance-landmines)): O(1) per message, no allocation.

**Quality checks**

- **Speech is counted on arrival in `handle`, before `sequencer.release_ready`.**
- Exactly one `ConversationWentQuiet` per session, however long the quiet lasts.
- `LaneMessage` and `LaneProgress` are unchanged.
- No new field on `Recorder`; `Recorder::new` signature unchanged.
- Existing `recording_session.rs` tests pass unmodified.

---

## Slice 3 — Stop the session and notify

**Key decision:** The tray handles `ConversationWentQuiet { id }` by stopping that session by id and then showing a plain notification. The existing `RecordingStopped` → `stopped()` path publishes state and tells the frontend. Decision 5; copy uses decision 4's constant.

**Structure**

- `apps/scribe-tray/src-tauri/src/tray_listener.rs` (modify) — replace slice 2's arm with `RecordingEvent::ConversationWentQuiet { id } => self.went_quiet(&id)`; add `fn went_quiet(&self, id: &SessionId)`.
- `apps/scribe-tray/src-tauri/src/notifications.rs` (modify) — `pub fn notify_recording_auto_stopped<R: Runtime>(app: &AppHandle<R>)` calling `show_plain` `[verified: show_plain(app, title, body)]`. Title const `RECORDING_AUTO_STOPPED_TITLE` = "Recording auto-stopped". Body built from `AUTO_STOP_AFTER_QUIET` in minutes: "No one spoke for 5 minutes, so Scribe stopped recording. The transcript is saved."
- `README.md` (modify) — one bullet under "What it does": recordings stop automatically after 5 minutes without speech.

**Collaborators:** `TrayUpdater` → `RecordingController::stop_session` `[verified: returns (), logs unclean finish]` → `notifications::notify_recording_auto_stopped`.

**Control flow:** `went_quiet`: `controller.stop_session(id)`, then `notify_recording_auto_stopped`. Do not call `publish_state` or emit `recording://stopped` here; `stopped()` does that when `RecordingStopped` arrives, and its own `stop_session` call is then a no-op `[verified]`. Hold no other lock across `stop_session`.

**Tests:** none new. `tray_listener.rs` and `notifications.rs` are Tauri-bound and untested by convention ([context §6](../specs/004-auto-stop-on-silence-context.md#6-test-conventions)); keep them logic-free. Manual check: start a recording, stay silent for 5 minutes, confirm the recording stops, the popover returns to idle and the notification appears; then press Stop yourself at 4 minutes and confirm no notification appears.

**Standards that bite here:** Clean Code: no side effects (the handler only stops and notifies), searchable constants for the copy. Incubyte: Conventional Commits.

**Quality checks**

- **Only `stop_session(&id)` is used, never `stop()`.**
- The notification minutes come from `AUTO_STOP_AFTER_QUIET`, not a literal `5`.
- `went_quiet` calls neither `publish_state` nor `emit`.
- No frontend file changes.

---

## Hard floors

None. No slice touches auth, authorization, payment, security or migrations.

## Escalation log

- 2026-10-19, review fix (slices 2 and 3, no tier change): the end-of-spec review found the auto-stop notice could show when `stop_session` did nothing (the user's Stop won the race, or the stop failed). Added `Recorder::is_recording_session(&SessionId) -> bool`, which `stop_session` now reuses, and `RecordingController::stop_if_recording(&SessionId) -> bool`; `TrayUpdater::went_quiet` notifies only when it returns true. This replaces slice 3's plain `stop_session` call. Also fixed a racy restart test by keeping the mute Me lane shorter than the limit.

