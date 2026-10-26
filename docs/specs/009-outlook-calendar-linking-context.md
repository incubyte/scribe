# Task context: 009-outlook-calendar-linking

> Generated for spec 39fbd1fa-158e-4b57-8678-588067443927 at HEAD 9db324061d66b7b2ac0968f861baa7dada65b582 · 2026-10-26
> Read first by every agent in /anthara:plan-implementer's slice loop.

Style: **hexagonal** (ADR 1, `docs/ARCHITECTURE.md` §1, §3, §7; [spec §7.2](009-outlook-calendar-linking.md#72-architectural-style)). Ports live in `crates/scribe-core/src/ports.rs`. Pure rules sit in `scribe-core` modules (`call/`, `update/`, `recording/readiness.rs`). The shell is a set of small `install(app)` modules wired by `lib.rs` (`start_scribe`). Core `chrono` has no `clock` feature, so the core cannot read the time and must be given it.

## 1. Affected modules
| Module / file | Role | Notes |
|---|---|---|
| `crates/scribe-core/src/ports.rs` | All ports | New `CalendarSource`, `CredentialVault`, `MeetingWindowTitles` go here. `TranscriptStore::finalise(id, ended_at)` has no slot for a meeting |
| `crates/scribe-core/src/call/call_watch.rs` | `CallWatch` latch | Tracks only `Instant`. It does not record when the call started |
| `crates/scribe-core/src/recording/session.rs` | `SessionHeader`, `SessionSummary`, `default_title` | Built with struct literals in `format.rs`, `session.rs`, `tests/session_id.rs`, and in store-markdown `tests/month_folders.rs` and `tests/transcript_file.rs` |
| `crates/scribe-core/src/recording/writer.rs` | `SessionWriter::finish` calls `store.finalise` | Runs on the writer thread. `Recorder::stop` joins it, so stop blocks until the file is final |
| `crates/scribe-core/src/library/summary_view.rs` | `SessionSummaryView` (serde behind the `serde` feature) | Shared by the tray `recent_sessions` command and the MCP DTOs |
| `crates/store-markdown/src/format.rs` | `render_header`, `parse_header`, `HeaderFields` | Hand-written YAML out, `serde_saphyr` in. Strings go through `quoted` |
| `crates/store-markdown/src/writer.rs` | `finalise` → `rewrite_header` → `replace_header` | Atomic header rewrite through `private_file::replace_atomically` |
| `crates/store-markdown/src/settings_file.rs` | `SETTINGS_FILE_NAME`, `TRANSCRIPTS_FOLDER_KEY` | Read-only helper. The tray writes `settings.json` through `tauri-plugin-store` |
| `apps/scribe-tray/src-tauri/src/call_watch.rs` | `call-watch` thread, 2 s poll, `CallPresence` | Opens the popover on a rising edge unless recording (`open_unless_recording`) |
| `src-tauri/src/recording.rs` | `RecordingController` (`start`, `stop`, `stop_if_recording`), `start_in_background`, `stop_before_sleep` | `start()` builds `SessionHeader::recording(Local::now()…)`, so the title and start time are fixed here |
| `src-tauri/src/tray_listener.rs` | `TrayUpdater` handles `RecordingEvent`s | `stopped` is where a "Saved <subject>" notification can go. `went_quiet` auto-stops after 5 min of quiet |
| `src-tauri/src/notifications.rs` | `show_plain`; button prompts for macOS (`system_settings`) and Windows (`privacy_settings`) | 312 lines; see dragons |
| `src-tauri/src/lib.rs` | `with_commands` (`generate_handler!`), `start_scribe` | New commands and `install` calls are registered here |
| `src-tauri/src/updates/` | Background thread, wake hook, `UpdateController` | Template for the agenda refresh loop |
| `src-tauri/capabilities/default.json` | Webview permissions | Has `store:default`, so the **webview can read `settings.json`** |
| `crates/audio-macos/src/call_probe.rs`, `crates/audio-windows/src/call_probe.rs` | How Teams is identified | `TEAMS_BUNDLE_ID_PREFIX`; `TEAMS_EXE` plus `ProcessTable::belongs_to_teams` (private) |
| `crates/capture-macos/src/outlines.rs`, `crates/capture-windows/src/outlines.rs` | Window enumeration already in use | cidre `sc`/`cg` on macOS; `xcap::Window` on Windows |
| `apps/scribe-mcp/src/server.rs`, `requests.rs` | `list_transcripts`, `get_transcript`, `TranscriptSummaryDto` | `get_transcript` returns `document.raw`, so the front matter already passes through |
| `apps/scribe-tray/src/popover/` | `Popover.tsx`, `RecentSessions.tsx`, `SettingsView.tsx`/`SettingsPanel.tsx`, `commands.ts`, `guards.ts`, `types.ts` | Popover window is 360×440 (`tauri.conf.json`) |
| `.github/workflows/ci.yml`, `.cargo/mutants.toml`, `clippy.toml` | CI guards and limits | See dragons |
| `docs/ARCHITECTURE.md` §5, §9; `README.md` Privacy; `src/privacy/PrivacyStatement.tsx` (`STATEMENTS`) | Privacy promise, ADR list | ADR 10 and the two new hosts go in all three (AC 5.1.11) |

## 2. Patterns to follow
- **Pure policy over a plain-data struct**: inputs are bundled in a struct so a function stays within the 3-argument limit. Defined in `scribe-core/src/update/install_moment.rs` (`InstallConditions`, `install_moment`). Re-exports and named `Duration` consts live in `mod.rs`, as in `update/mod.rs` (`CHECK_INTERVAL`, `CHECK_DELAY_AFTER_WAKE`).
- **Time-injected state machines**: `CallWatch::observe(bool, Instant)` and `CheckSchedule::is_check_due(now)`. Wall-clock policy takes `DateTime<FixedOffset>` as an argument.
- **Shell `install(app)` + named thread + managed state**: `call_watch.rs` (`install`, `CallPresence`) and `updates/mod.rs` (`install`, `check_forever`, `tick`). A wake hook goes through `power::on_did_wake` and an `AtomicBool` that the loop reads (`UpdateController::note_wake`/`take_wake`).
- **Poison-tolerant Mutex**: `lock().unwrap_or_else(PoisonError::into_inner)`, as in `recording.rs` (`RecordingController::recorder`) and `updates/controller.rs` (`lifecycle`).
- **Commands**: `#[tauri::command]` plus `#[allow(clippy::needless_pass_by_value)]`, `AppHandle<R>` first, errors as `Result<T, String>` through `error.to_string()` (`commands.rs` (`recent_sessions`, `open_session`)). Slow work uses `pub async fn` (`change_transcripts_folder`).
- **Webview events**: `webview::emit(app, "<area>://<event>", payload)` with a `Serialize + Clone` payload, as in `recording.rs` (`STATE_EVENT = "recorder://state"`). Frontend: `listenFor(event, guard, handle)` in `useRecordingFeed.ts`.
- **Front matter**: a field is written by `render_header` and read through a private `#[derive(Deserialize)]` struct, then mapped to a core type (`HeaderFields::into_header`). Core types stay free of serde, which sits behind the optional `serde` feature.
- **Settings keys**: `app.store(SETTINGS_FILE_NAME)?`, then `store.set(KEY, json!(…))`, then `store.save()`, with a `const …_KEY` per module (`privacy.rs` (`ACKNOWLEDGED_KEY`, `acknowledge`)).
- **HTTP**: a `reqwest::blocking::Client` with `https_only(true)`, a connect timeout and a timeout, behind a trait seam. A `thiserror` error with `kind() -> &'static str` is logged as `kind = …`. Defined in `crates/stt-parakeet/src/fetcher.rs` (`HttpsFetcher::new`) and `download_error.rs` (`DownloadError::kind`).
- **Frontend IPC**: one wrapper per command in `commands.ts`, `invoke<unknown>(…).then(expecting(guard, "what"))`. Guards go in `guards.ts` (`hasFields`, `isOneOf`, `isListOf`, `isNullable`). Each component has a `use*` hook that refreshes on window `focus` (`useClaudeDesktopStatus.ts`).
- **Logging**: `tracing::warn!(%error, "lowercase sentence")`, with enums as `kind = …`. Never log user data (ARCHITECTURE §5). Meeting subjects, names and emails count as user data.
- **Clippy limits are hard errors** (`clippy.toml`): 20 lines per fn, 3 arguments, cognitive complexity 5; pedantic with `-D warnings`.
- **Advisory (ai-ergonomics):** `notifications.rs` is 312 lines and mixes four concerns. Put meeting notifications in their own module that calls `show_plain`. Also, the frontend `src/meeting-notebook/` already has `MeetingBody`/`MeetingSummaryBody`. Name the calendar types so they don't collide (e.g. under `src/popover/agenda/`).

## 3. Reusable utilities
- `scribe-core/src/call/mod.rs`: `CALL_END_GRACE`, `CallWatch` (`is_start_due`, `mark_reported`, `is_in_call`)
- `scribe-core/src/recording/session.rs`: `default_title`, `SessionHeader::recording`/`finalised`
- `store-markdown/src/format.rs`: `quoted` (YAML string escaping), `timestamp` (RFC 3339 seconds), `split_front_matter`
- `store-markdown/src/writer.rs`: `rewrite_header` (read, change, atomic replace)
- `src-tauri/src/call_watch.rs`: `CallPresence::is_in_call`. `popover.rs`: `open_for_call`, `is_open`, `keep_open_during`
- `src-tauri/src/recording.rs`: `is_recording`, `start_in_background`, `report_start_failure`. `start_failure.rs`: `StartFailureKind`
- `src-tauri/src/notifications.rs`: `show_plain`, `notify_recording_not_started`
- `src-tauri/src/power/mod.rs`: `on_did_wake`, `on_will_sleep` (both macOS and Windows accept several listeners)
- `tauri_plugin_opener::OpenerExt::open_url` opens the system browser from Rust (`notifications.rs` (`open_settings`))
- Frontend: `shared/guards.ts` (`expecting`, `hasFields`); `popover/format.ts`; `useAnnouncement.ts`; `usePrefersReducedMotion.ts`
- **Dependencies already in the tree:** HTTP is `reqwest` 0.13 (blocking, rustls) in `stt-parakeet`; `url` 2.5 and `rand` 0.9 come in transitively. `sha2` 0.11 is a direct dependency of `stt-parakeet`, and `base64` 0.22 of `scribe-mcp`. **Keychain:** no `keyring` crate. `security-framework` 3.7 arrives on macOS only through `rustls-platform-verifier`. On Windows, `windows` 0.62 needs the `Win32_Security_Credentials` feature added. **Window titles:** macOS has cidre 0.29 (`sc::Window` title and owning app bundle id; `cg::screen_capture_access::preflight`) in `capture-macos`. Windows has `xcap` 0.9 (`Window::title`, `pid`) in `capture-windows`, and the `windows` crate with `Win32_UI_WindowsAndMessaging`. **Loopback listener:** no HTTP server crate; `std::net::TcpListener` is enough.

## 4. Adjacent code that must coordinate (per slice)
- **Slice 5.1:** `lib.rs` (`with_commands`, `start_scribe`); `SettingsView.tsx`/`SettingsPanel.tsx`; `ci.yml` reqwest guard and `PORTABLE_CRATES`; ARCHITECTURE §5 and §9 (ADR 10); README Privacy; `PrivacyStatement.tsx` (`STATEMENTS`); `Cargo.toml` (workspace deps).
- **Slice 5.2:** `Popover.tsx` (the `RecentSessions` branch); `useRecordingFeed.ts` (`recent`); `updates/mod.rs` (wake + loop shape); `popover.css` (`recent__*`, `session*`).
- **Slice 5.3:** `recording.rs` (`RecordingController::start`); `writer.rs` (`SessionWriter::finish`); `ports.rs` (`TranscriptStore`); `testing.rs` (`InMemoryTranscriptStore::finalise`); `format.rs`; `summary_view.rs`; `scribe-mcp` `requests.rs` (`TranscriptSummaryDto`) and the tool descriptions in `server.rs`; `apps/scribe-mcp/tests/mcp_tools.rs`.
- **Slice 5.4:** `call_watch.rs` (`open_if_call_started`); the Teams identity consts in `audio-macos`/`audio-windows` `call_probe.rs`; `capture-*/outlines.rs`.
- **Slice 5.5:** `call_watch.rs` (`open_unless_recording`); `popover.rs` (`open_for_call`); `StatusBar.tsx`, `LiveTranscript.tsx`; `commands.rs` (`start_recording` takes no arguments today).
- **Slice 5.6:** `call_watch.rs`; `tray_listener.rs` (`stopped`, `went_quiet`); `recording.rs` (`report_start_failure`, `stop_before_sleep`); `notifications.rs`; `privacy.rs` (settings-key shape).
- **Slice 5.7:** `RecordingController` (`stop`, `start`); `popover.rs` (`open_for_call`, which does nothing if the popover is already visible); `tray_listener.rs`.

## 4a. Slice imitation targets
- **Slice 5.1:** the HTTP adapter mirrors `stt-parakeet/src/fetcher.rs` (`HttpsFetcher`, `RangeFetcher` seam) and `download_error.rs`. The shell mirrors `model_setup.rs` (`install`, `ModelSetup`, `AtomicBool` single-flight) for "connect in progress". Commands mirror `claude_desktop_status`/`connect_claude_desktop` in `commands.rs`. The hook mirrors `useClaudeDesktopStatus.ts`.
- **Slice 5.2:** core agenda filter and overlap layout mirror `update/install_moment.rs` with tests like `tests/install_moment.rs`. The refresh loop mirrors `updates/mod.rs` (`install`, `check_forever`, `tick`, `check_after_wake`) with `CheckSchedule`. The component mirrors `RecentSessions.tsx` (`useId` heading, `<ul aria-labelledby>`).
- **Slice 5.3:** the `meeting` block mirrors `format.rs` (`render_header`, `HeaderFields::into_header`). The MCP field mirrors `TranscriptSummaryDto`'s `From<SessionSummaryView>`.
- **Slice 5.4:** the adapter mirrors `capture-macos/src/outlines.rs` (`window_candidates`, `owner_pid`) and `capture-windows/src/outlines.rs`. Selection lives in `call_probes.rs` (per-OS `mod` with an `unsupported` fallback).
- **Slice 5.5:** the selector is greenfield. Visually it is closest to `IdleNotice.tsx` (native `<input>` inside a label). State and event mirror `recorder://state` (`StatePayload`, `isStatePayload`).
- **Slice 5.6:** armed-state rules mirror `update/schedule.rs` (`CheckSchedule`). Persistence mirrors `privacy.rs` (`is_acknowledged`, `acknowledge`). Notifications mirror `notify_recording_auto_stopped`.
- **Slice 5.7:** greenfield prompt. Focus-on-open mirrors `SettingsPanel.tsx` (`heading.current?.focus()`).

## 4b. Slice lexicon
Names fixed by the spec are in quotes. The rest are proposals; pin them in slice 5.1/5.2 and reuse them after that.
- **Slice 5.1:** core module `scribe_core::calendar`; ports `CalendarSource`, `CredentialVault`, `MeetingWindowTitles` ([spec §7.3](009-outlook-calendar-linking.md#73-module-decomposition)); `Connection::{Disconnected, Connecting, Connected, ReconnectNeeded}`; scopes `"openid profile offline_access Calendars.ReadBasic"`; authority `login.microsoftonline.com/organizations`; `SIGN_IN_TIMEOUT` (5 min); commands `outlook_connection`, `connect_outlook`, `disconnect_outlook`; event `outlook://connection`; credential service `com.incubyte.scribe`; adapter crate `calendar-graph`; UI copy "Connect Outlook", "Disconnect", "Reconnect Outlook", "Outlook didn't connect: …".
- **Slice 5.2:** `Meeting` (core), `TodaysAgenda`; `AGENDA_REFRESH_INTERVAL` (5 min); `MIN_BLOCK_MINUTES` (15); command `todays_agenda`; event `agenda://updated`; component `TodayAgenda`; copy "Today · <weekday> <day>", "No meetings today", "Couldn't load your calendar", "Retry", "Updated <time>".
- **Slice 5.3:** front-matter key `meeting` with `event_id`, `subject`, `starts_at`, `ends_at`, `organizer {name, email}`, `attendees`, `attendee_count`; `MeetingLink`; `MatchAnchor`; `MeetingCandidate`; `MatchOutcome::{Matched, Ambiguous, Unmatched}`; `CANDIDATE_LEAD` (10 min); `MAX_LISTED_ATTENDEES` (50).
- **Slice 5.4:** `MeetingWindowTitles::teams_window_titles`; `meeting_named_in_titles`; reuse `TEAMS_BUNDLE_ID_PREFIX` / `TEAMS_EXE`.
- **Slice 5.5:** command `choose_meeting` (event id, or null for "Not a calendar meeting"); event `meeting://match`; component `MeetingLine`; copy "Which meeting is this?".
- **Slice 5.6:** settings key `armed_meetings` (event id + date); `ArmedMeetings`; `ArmedState::{Armed, Recording, Recorded, Missed}`; `MISSED_AFTER` (5 min); commands `arm_meeting`, `disarm_meeting`; `notify_meeting_recording` / `_saved` / `_missed`; copy "Record <subject> automatically", "No call detected", "Recording <subject>", "Saved <subject>".
- **Slice 5.7:** `SwitchPrompt`; event `meeting://switch-prompt`; commands `switch_meeting`, `keep_recording`; copy "Still recording <A>. Switch to <B>?", "Switch", "Keep recording".

## 4c. Slice traps
- **Slice 5.1:** (1) Do NOT put tokens in `settings.json` or the store plugin, and do NOT return them from any command. The webview has `store:default` and can read the whole file (invariant [4.3](009-outlook-calendar-linking.md#4-invariants)). (2) Adding `reqwest` to a new crate fails `ci.yml`'s "Only the Parakeet adapter and the updater may depend on an HTTP client" step. Extend that allow-list deliberately and add the crate to `PORTABLE_CRATES` if it is platform-neutral. (3) Do NOT pull in `tokio` or async OAuth crates. The shell has no runtime except `tauri::async_runtime`; blocking `reqwest` on a named thread is the house style. (4) Use the `/organizations` authority so only work and school accounts can sign in, bind to `127.0.0.1:0` (not `localhost`), take one request, and stop after 5 min. (5) The credential store is cfg-gated in the shell or adapter, never `#[cfg(target_os)]` in the core.
- **Slice 5.2:** (1) Overlap layout and filtering are core rules ([spec §6.3](009-outlook-calendar-linking.md#63-clean-code--incubyte-engineering-standards)). The core computes column and column-count, and the webview only places blocks. (2) Local midnight needs the local offset, which only the tray can read (`chrono` `clock` is tray-only). Pass the day bounds into the core. (3) Do NOT fetch Graph inside a synchronous `#[tauri::command]`, which blocks the main thread. Return the cached agenda and refresh on the agenda thread. (4) Every `Popover.spec.tsx` `backend()` must answer the new commands. An unanswered `invoke` makes `expecting(...)` reject, which fails `npm test` with no failing test.
- **Slice 5.3:** (1) A malformed `meeting` block must not make `parse_header` fail ([spec §6.1](009-outlook-calendar-linking.md#61-security-and-privacy-architecture-5-extended) 6.1.5). Deserialize it leniently (an optional, separately validated value) so the transcript still parses. (2) The meeting choice can change until Stop, but `Recorder::stop` joins the writer, which calls `finalise` with only `(id, ended_at)`. Either extend the port, so `InMemoryTranscriptStore` changes too, or rewrite the header with `rewrite_header` after the stop. Do not stash it in a global the writer reads. (3) The anchor is the call's rising edge in **wall-clock** time. `CallWatch` keeps only `Instant`, so record a `DateTime<FixedOffset>` at the edge in `call_watch.rs`. (4) Adding a `SessionHeader` field breaks every struct literal listed in §1. Prefer `Option` with a builder (`with_meeting`). (5) Escape subjects and names with `quoted`; subjects contain `:` and `"`.
- **Slice 5.4:** (1) On macOS, check `cg::screen_capture_access::preflight()` before touching `sc::ShareableContent`. Never call `request()`; AC 5.4.3 forbids a prompt. (2) Read titles only once, at the rising edge, not every 2 s poll, and never log them. (3) On Windows, the Teams window may belong to a child process. Mirror the parent walk in `audio-windows` `ProcessTable::belongs_to_teams` (currently private) instead of matching the exe name only.
- **Slice 5.5:** (1) Today the popover has no "opened for a call" state: `open_for_call` only shows the window. Publish the match through an event and a snapshot command so a popover opened later can catch up. (2) `open_for_call` does nothing when the popover is already visible, so the meeting line must update through the event. (3) The selector is a real radio group (`<fieldset>`/`<input type="radio">`), checked with `vitest-axe`.
- **Slice 5.6:** (1) Auto-start goes through `RecordingController::start` and on failure through `report_start_failure`, never around it. (2) `CallWatch` reports a call once (`mark_reported`), and `stop_before_sleep` ends recordings at sleep. After wake the mic is still held, so there is no new rising edge (and macOS `Instant` pauses in sleep). AC 5.6.8 needs an explicit "in call, armed, not recording" check on each poll. (3) Only auto-started recordings stop on call end, so keep an `auto_started` flag next to the recording, not in `CallWatch`. (4) Prune entries from earlier days on read, as a pure function over (entries, today).
- **Slice 5.7:** (1) `RecordingController::stop` blocks until transcription drains and `finalise` runs, and `start` needs `ensure_idle`. Measure the 2 s gap (AC 5.7.3) rather than assuming it. (2) The "once per meeting B" memory is core state with time passed in, not a frontend flag. (3) Never split without the user's click (invariant [4.8](009-outlook-calendar-linking.md#4-invariants)); timing out means keep recording.

## 5. Known dragons
- `ci.yml` HTTP-client guard (see 5.1 trap 2). The `scribe-mcp` guard bans `capture-`, `audio-`, `stt-`, `cidre` and `xcap`. Keep `scribe-mcp` off the calendar crates (invariant [4.6](009-outlook-calendar-linking.md#4-invariants), [spec §7.2](009-outlook-calendar-linking.md#72-architectural-style)).
- The `windows` CI job only runs clippy. Windows credential-store and window-title code is never tested in CI.
- `.cargo/mutants.toml` excludes `call_watch.rs`, `popover.rs`, `updates/**` and `notifications.rs`. Exclude new AppHandle and OS glue the same way, and keep the logic in tested core code.
- `tray_listener.rs` has a struct named `TrayUpdater`, and `updates/` uses "updater". Avoid "updater" in agenda names.
- [Spec §10](009-outlook-calendar-linking.md#10-established-behaviour--documented-not-changed) 10.2 says macOS notifications have no buttons, but `notifications.rs` (`system_settings::user_chose_to_open_settings`) shows a single-button `mac_notification_sys` prompt that blocks its thread. The spec's popover-only decision still holds.
- No TODO, FIXME or LEGACY markers in the touched areas.

## 6. Test conventions
- Core: `crates/scribe-core/tests/<topic>.rs`; no inline `#[cfg(test)]`. Fakes are in `src/testing.rs` behind the `testing` feature (`InMemoryTranscriptStore`, `RecordingListenerSpy`). Names are business-language sentences (`a_release_one_millisecond_short_of_the_grace_keeps_the_same_call`), with times as named consts and arrange/act/assert separated by blank lines. Run `cargo test -p scribe-core`.
- Store: `crates/store-markdown/tests/*.rs` with `tempfile` and a `moment("…rfc3339")` helper (`transcript_reading.rs`). Add old-file fixtures that have no `meeting` block (AC 5.3.6).
- MCP: `apps/scribe-mcp/tests/mcp_tools.rs` (async rmcp client, `connect_to(folder)`).
- Shell: pure `pub mod`s tested from `src-tauri/tests/<module>.rs` (`start_failure.rs`, `live_feed.rs`). Stage the sidecar first.
- Frontend: vitest + jsdom, `*.spec.ts(x)` next to the source, `mockIPC(handler, { shouldMockEvents: true })` with a `backend()` response map, fake `Date` only (`vi.useFakeTimers({ toFake: ["Date"] })`), and `vitest-axe`. Check the exit code of `npm test`. Stryker runs through `npm run mutate`.

## 7. Hot paths and performance landmines
- `call_watch.rs` (`watch_forever`) polls every 2 s. Matching there may read only the cached agenda, with no Graph call and no window enumeration except once at the rising edge.
- Synchronous Tauri commands run on the main thread. Anything that touches the network, keychain or loopback must be `async` or handed to a thread.
- `RecordingController`'s mutex is held during start and stop. Never hold it while calling Graph or showing a notification.
- `recent_sessions` and agenda fetches run on every popover `focus`. Keep the agenda command a cheap cache read.

## 8. Open uncertainties resolved during orientation
- HTTP: the workspace already ships blocking `reqwest` 0.13 with rustls, so no new HTTP stack is needed. Only the CI guard changes.
- Keychain: there is no existing wrapper. Choose between adding `keyring` (apple-native and windows-native) and using `security-framework` plus `windows` `Win32_Security_Credentials` directly. Record the choice in ADR 10.
- Window titles: cidre and xcap are already linked in `capture-macos` and `capture-windows`, so those crates are the natural adapter homes, and both are already excluded from `scribe-mcp`. [Spec §8](009-outlook-calendar-linking.md#8-open-questions) OQ 8.4 (whether titles carry the subject) remains open.
- MCP: `get_transcript` already returns the raw front matter, so AC 5.3.7 needs real work only in `list_transcripts`. Serde ignores unknown fields by default, so older MCP builds will tolerate the new block.
- OQ 8.1–8.3 (tenant consent, whether `ReadBasic` returns attendees, app ownership) are external and remain open. Settle them before slice 5.1.

## 8a. Design system (UI slices only)
- **Source of truth:** CSS custom properties in `apps/scribe-tray/src/theme/tokens.css`, with light and dark themes through `prefers-color-scheme`. There is no `design.md` and no Tailwind.
- **Tokens:** spacing `--space-1…5` (4–24 px); `--radius-control`; text `--text-sm/md/lg`; colours `--accent`, `--accent-soft`, `--recording` (use it for the red "now" line), `--text-muted` (for past blocks, together with a non-colour cue), `--hairline`, `--surface-sunken`, `--focus-ring`.
- **Components:** none shared. Reuse the BEM classes in `theme/popover.css` (`recent__heading`, `session`, `session__title`/`__meta`, `actions__primary`/`__link`, `settings__*`) and `theme/primitives.css` (`.button`, `.visually-hidden`).
- **Conventions:** dense layout at 360×440. Sentence-case labels with no trailing full stop. Every interactive element gets a `:focus-visible` ring from `--focus-ring`. Motion respects `usePrefersReducedMotion`. The target is WCAG 2.2 AA, enforced by `eslint-plugin-jsx-a11y` and `vitest-axe`.
- **Slice 5.1:** imitate the folder row in `SettingsPanel.tsx`. **5.2:** `RecentSessions.tsx` + `.recent*`. **5.5/5.7:** sit under `StatusBar.tsx`; focus handling as in `SettingsPanel`. **5.6:** no switch exists yet; base it on `<input type="checkbox" role="switch">` like the checkbox in `IdleNotice.tsx`.

---
## Changelog
- 2026-10-26 — generated by code-orienteer from spec 39fbd1fa-158e-4b57-8678-588067443927 at HEAD 9db3240.
