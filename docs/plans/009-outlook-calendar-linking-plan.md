# Plan: 009-outlook-calendar-linking

> Spec: `docs/specs/009-outlook-calendar-linking.md` · Context: `docs/specs/009-outlook-calendar-linking-context.md` (HEAD 9db3240) · Authored: 2026-10-26
> Risk: HIGH (OAuth sign-in, a token kept in the OS credential store, the first non-update outbound traffic, and recordings that start on their own) · Hard floors: slice 1 · Fabric standards: reachable (Clean Code, Incubyte Engineering Standards, Design Standards: Global Component, Security)

---

# Part 1 — Review this

*Everything a reviewer needs to approve or redirect the approach. If you only read one part, read this one.*

## What we're doing

Scribe gets a "Connect Outlook" button in Settings. Once connected, the popover shows today's meetings, every transcript records which meeting it belongs to and who was invited, and a switch on a meeting makes its Teams call record by itself. Signed out, Scribe behaves exactly as it does today.

Scribe has no server, so the desktop app signs in to Microsoft directly and calls Microsoft Graph itself. The sign-in, the token and every Graph call stay in the Rust side of the tray. The webview only ever sees the account name, the meeting list and a connection state. This is forced by two facts: the webview can read all of `settings.json` ([context §5](../specs/009-outlook-calendar-linking-context.md#5-known-dragons)), and the call watcher that decides when to record is Rust.

Every decision rule is a pure function in `scribe-core`, with the time passed in. That covers which meetings show, how overlapping meetings are laid out, which meeting a call belongs to, when an armed meeting records or counts as missed, and when to offer a switch. Graph, the credential store and window titles plug in behind three new ports. The same split already serves the call latch and the update schedule, and it keeps the risky logic under Linux CI and mutation testing, which never see the OS glue.

One deliberate exception: the Windows window-title reader lives in `audio-windows`, not `capture-windows`. That crate already owns the process table that decides whether a window belongs to Teams, so this avoids a second copy of that logic ([decision 6](#decisions-made)).

## How it breaks down

Slice 1 builds the sign-in and the Graph adapter end to end, because it carries the most risk: tenant consent, the credential store on two OSes, and the HTTP guard in CI. Each later slice adds one rule to the core and one surface to the popover, in spec order. Matching grows across slices 3, 4 and 6 by adding inputs to one function rather than adding parallel paths.

| # | Slice | Delivers | Why this tier | Tier |
|---|-------|----------|---------------|------|
| 1 | Connect an Outlook work account | PKCE sign-in, token in the keychain, Settings row, ADR 10 | Auth and secrets; floored; a mistake leaks a token or locks users out | `best` |
| 2 | See today's meetings | Agenda filter and layout rules, 5-minute refresh thread, time-grid in the popover | Layout geometry and a refresh loop; medium difficulty, contained blast radius | `excellent` |
| 3 | Save the meeting with each transcript | Candidate ranking, `meeting` front-matter block, MCP listing field | Changes the transcript file format that every reader depends on | `excellent` |
| 4 | Recognise the meeting from the Teams title | Title rule plus a macOS and a Windows title reader | Small rule, but OS glue that CI can't test | `excellent` |
| 5 | Confirm or correct the meeting | Call-start match published to the popover, radio-group selector | New popover state plus an event; standard UI work | `excellent` |
| 6 | Arm a meeting to record by itself | Armed-meeting state machine, auto-start and auto-stop, missed notice | Scribe starts recording on its own; wake, sleep and call-end edge cases | `best` |
| 7 | Switch to the next meeting | Once-per-meeting switch prompt, stop-then-start handover | Reuses slices 4–6; the risk is the 2-second gap, handled by a measured fallback | `excellent` |

Slices 3, 4 and 6 extend the same `match_meeting` function. Slice 5 reuses slice 3's selection state rather than adding a second one. Slices 6 and 7 both run inside the existing 2-second call-watch poll.

## Decisions made

**1. All OAuth, tokens and Graph traffic live in a new portable crate, `calendar-graph`, driven from Rust.** It uses the blocking `reqwest` 0.13 client the workspace already ships, on named threads, as the Parakeet downloader does. The webview never receives a token. Lands in slices 1–2. *Rejected: a JavaScript OAuth library in the webview. The webview can read `settings.json` and has no business holding a refresh token, and the call watcher needs the agenda in Rust anyway.*

**2. The refresh token goes into the OS credential store through the `keyring` crate, version 3.6, with `apple-native` and `windows-native`.** It's one small wrapper with no `unsafe`, and the workspace denies `unsafe_code`. The token is stored as UTF-8 bytes through `set_secret` to stay under the Windows Credential Manager size limit. Lands in slice 1 and is recorded in ADR 10. *Rejected: `security-framework` plus `Win32_Security_Credentials` by hand, which means two unsafe implementations for no gain. Also rejected: keyring 4.x, which splits the API across store crates and is still settling.*

**3. The account's name and email come from the `id_token` that sign-in and every refresh already return.** Nothing about the account is persisted except the refresh token. After a restart, the first refresh restores the name. This keeps the scopes exactly `openid profile offline_access Calendars.Read` (AC 5.1.4). *Rejected: calling `/me`, which needs `User.Read`, and caching the name in `settings.json`, which the webview can read.*

**4. Only work and school accounts can sign in, enforced by using the `/organizations` authority.** It's a URL choice, not code. *Rejected: `/common` with a check after sign-in, which would let a personal account reach consent first.*

**5. Graph adapters return times converted into the offset of the day bounds the tray passes in.** The core never reads a clock. The tray computes local midnight to midnight and passes it as `DayBounds`. *Rejected: giving `scribe-core` chrono's `clock` feature, which breaks the time-injection pattern that the call latch and update schedule rely on.*

**6. Teams window titles are read by adapters that sit next to the code that already knows Teams.** On macOS that's `capture-macos`, which has cidre's `sc` windows and the permission preflight; the Teams bundle prefix moves into `scribe_core::call` so the audio and capture crates share it. On Windows it's `audio-windows`, which owns the Teams process-ancestry table. Lands in slice 4. *Rejected: making `capture-windows` depend on `audio-windows`, or duplicating the ancestry walk.*

**7. The meeting a transcript belongs to is held by the tray and written into the header after the recording is final.** The tray keeps the choice in a `MeetingSelection` state. When `RecordingStopped` arrives, which the writer emits after `finalise`, the tray calls `MarkdownTranscriptStore::link_meeting`, which rewrites the header atomically. That one hook covers every way a recording stops: Stop, five minutes of quiet, sleep, and call end. Lands in slice 3. *Rejected: widening `TranscriptStore::finalise`. That would thread a mutable choice into the writer thread and change every implementation of the port.*

**8. One matching function decides every case, in a fixed order.** First a title that names exactly one candidate, then an armed candidate, then the ranking rules: Teams first, then the user's response, then nearest start. Slices 3, 4 and 6 each add one input. *Rejected: separate match paths for manual recordings, the call-start popover and auto-record, which would drift apart.*

**9. Auto-record is checked on every 2-second poll, not only when a call starts.** The check fires when a call is in progress, an armed meeting matches, nothing is recording, and that meeting hasn't already recorded. This one rule handles the computer waking mid-call (AC 5.6.8), where no new call start ever fires. Lands in slice 6. *Rejected: starting only on the call-start edge, which misses the wake case.*

**10. "Switch to B" stops A and then starts B, and we measure the gap before building anything more complex.** If the measured gap is over 2 seconds (AC 5.7.3), the fallback is already decided: `Recorder::stop_detached` stops the capture sources, hands the drain and `finalise` to a background thread, and returns at once. Lands in slice 7. *Rejected: running two captures at once, which risks device contention on both OSes.*

**11. While recording a linked transcript during a call, Teams titles are re-read every 30 seconds.** That is enough to notice a move to meeting B without enumerating windows on every 2-second poll (AC 5.4.5, [context §7](../specs/009-outlook-calendar-linking-context.md#7-hot-paths-and-performance-landmines)).

## Where this could go wrong

- **Tenant consent is unknown until a real sign-in ([spec §8.1](../specs/009-outlook-calendar-linking.md#8-open-questions)).** A tenant can block user consent even for `Calendars.Read`. The scope is `Calendars.Read`, not `ReadBasic`, because Microsoft doesn't document that `ReadBasic` returns attendees (see the Escalation log). Sapan runs every real-tenant check at the end of the build, not between slices.
- **The refresh token may be too large for Windows Credential Manager.** It caps the secret at 2,560 bytes. Slice 1 stores UTF-8 bytes and fails the connection with a named error rather than truncating; a quality check guards this. A Windows probe at build time confirms the real size.
- **Auto-record could start or stop when it shouldn't.** The pure `ArmedWatch` has a test for each transition in [spec slice 5.6](../specs/009-outlook-calendar-linking.md#56-arm-a-meeting-to-record-by-itself)'s state diagram. The bold quality check in slice 6 asserts that no start happens without a detected call (invariant 4.7).
- **Transcripts are on-disk data.** A malformed `meeting` block must never make a transcript unreadable. Slice 3 parses the block separately and drops it on error, and old files without the block are covered by a test fixture.
- **Scope creep into adjacent behaviour.** `CALL_END_GRACE` stays at 30 s ([spec §10.1](../specs/009-outlook-calendar-linking.md#10-established-behaviour--documented-not-changed)). Recent sessions stay exactly as they are while signed out. Teams-only detection is not widened ([spec §9.1](../specs/009-outlook-calendar-linking.md#9-deferred-follow-ups-and-accepted-limits)). Each has an "unchanged" quality check in the slice that touches its file.
- **Spec §10.2 gives the wrong reason.** It says macOS notifications carry no buttons, but `notifications.rs` already shows a one-button prompt for System Settings ([context §5](../specs/009-outlook-calendar-linking-context.md#5-known-dragons)). The decision to put prompts in the popover still holds; only the spec's wording needs a fix, and the plan doesn't change it.

## What's out of scope

Personal Microsoft accounts, several accounts at once, Google Calendar, days other than today, and writing anything to Outlook ([spec §1](../specs/009-outlook-calendar-linking.md#1-overview--business-context), [§9.2](../specs/009-outlook-calendar-linking.md#9-deferred-follow-ups-and-accepted-limits)). Calls outside Teams are never detected, so an armed Zoom or in-person meeting reports Missed ([§9.1](../specs/009-outlook-calendar-linking.md#9-deferred-follow-ups-and-accepted-limits)). Ad-hoc Teams calls carry no meeting ([§9.3](../specs/009-outlook-calendar-linking.md#9-deferred-follow-ups-and-accepted-limits)). Recent sessions disappear while connected ([§9.4](../specs/009-outlook-calendar-linking.md#9-deferred-follow-ups-and-accepted-limits)). A builder working near these will be tempted to:

- add a "recent" tab beside Today;
- shorten `CALL_END_GRACE`;
- refactor the 312-line `notifications.rs`.

None of those is part of this plan. Meeting notifications go in a new module that calls `show_plain`.

---

# Part 2 — Build detail

*Prescriptive detail for the build agent. Reviewers can skim or skip.*

**Confidence markers used below:** `[verified]` = the planner read the code or docs and confirmed it. `[inferred]` = pattern-matched from a sibling implementation, not confirmed — the build agent should check before assuming.

## Shared conventions (all slices)

- **Architecture:** hexagonal (ADR 1, [spec §7.2](../specs/009-outlook-calendar-linking.md#72-architectural-style)). Ports go in `crates/scribe-core/src/ports.rs`. Pure rules go in a new `scribe_core::calendar` module, with time and day bounds always passed in. Adapters go in `crates/calendar-graph`, `capture-macos` and `audio-windows`. The tray composes them in a new `src-tauri/src/calendar/` domain folder. Recording, call detection, front matter and the popover keep their behaviour while signed out (invariant 4.1).
- **Dependency direction:** tray → `calendar-graph` → `scribe-core`; tray → `scribe-core`. `scribe-mcp` → `store-markdown` → `scribe-core` only; `scribe-mcp` never depends on `calendar-graph`.
- **Module map:**
  - `crates/scribe-core/src/calendar/` (new): types and pure rules for agenda, layout, matching, titles, armed meetings and switching. Depends on chrono only. `[verified]` that the core has no chrono `clock` feature.
  - `crates/scribe-core/src/ports.rs` (modify): `CalendarSource`, `CredentialVault`, `MeetingWindowTitles`. `[verified]`
  - `crates/scribe-core/src/recording/session.rs` (modify): `SessionHeader.meeting: Option<MeetingLink>`. `[verified]` that the struct has seven public fields today.
  - `crates/calendar-graph/` (new, portable): PKCE, the loopback listener, the token client, the Graph `calendarView` client, and `OutlookAccount`. Depends on `scribe-core`, `reqwest` 0.13 (blocking, rustls), `url`, `sha2` 0.11, `base64` 0.22, `rand` 0.9, `serde`, `serde_json`, `chrono`, `thiserror`, `tracing`. `[verified]` that all of these are already in `Cargo.lock`.
  - `crates/store-markdown/src/format.rs`, `writer.rs` (modify): the `meeting` block and `link_meeting`. `[verified]`
  - `crates/capture-macos/src/teams_titles.rs` (new): macOS title reader. `crates/audio-windows/src/teams_windows.rs` (new): Windows title reader.
  - `apps/scribe-tray/src-tauri/src/calendar/` (new): `mod.rs` (`install`), `connection.rs`, `vault.rs`, `agenda.rs`, `selection.rs`, `armed.rs`, `switching.rs`, `notify.rs`, `titles.rs`, `commands.rs`.
  - `apps/scribe-tray/src-tauri/src/call_watch.rs` (modify): record the call start as a wall-clock time; run the calendar steps on each poll. `[verified]`
  - `apps/scribe-tray/src/popover/agenda/`, `src/popover/outlook/`, `src/popover/meeting/` (new): frontend components and hooks.
  - `apps/scribe-mcp/src/requests.rs` (modify): `meeting` on `TranscriptSummaryDto`. `[verified]`
- **Lexicon (from [context §4b](../specs/009-outlook-calendar-linking-context.md#4b-slice-lexicon), do not rename):**
  - Core types: `Meeting`, `EventId`, `Person`, `Response`, `CalendarEvent`, `DayBounds`, `TodaysAgenda`, `MeetingLink`, `MatchAnchor`, `MatchOutcome::{Matched, Ambiguous, Unmatched}`, `ArmedMeetings`, `ArmedState::{Armed, Recording, Recorded, Missed}`, `SwitchPrompt`, `Connection::{Disconnected, Connecting, Connected, ReconnectNeeded}`.
  - Constants: `CANDIDATE_LEAD` (10 min), `MAX_LISTED_ATTENDEES` (50), `MIN_BLOCK_MINUTES` (15), `AGENDA_REFRESH_INTERVAL` (5 min), `MISSED_AFTER` (5 min), `SIGN_IN_TIMEOUT` (5 min), `TITLE_RECHECK_INTERVAL` (30 s).
  - Commands: `outlook_connection`, `connect_outlook`, `disconnect_outlook`, `todays_agenda`, `refresh_agenda`, `call_meeting`, `choose_meeting`, `arm_meeting`, `disarm_meeting`, `record_meeting`, `switch_prompt`, `switch_meeting`, `keep_recording`.
  - Events: `outlook://connection`, `agenda://updated`, `meeting://match`, `meeting://switch-prompt`.
  - Settings key: `armed_meetings`. Keychain service `com.incubyte.scribe` (the Tauri identifier, `[verified]`), account `outlook-refresh-token`.
  - Frontend: `TodayAgenda`, `OutlookRow`, `MeetingLine`, `SwitchPromptCard`. Avoid `Meeting*Body` names, which `src/meeting-notebook/` already uses, and avoid "updater".
- **Tests:**
  - Core rules: new files under `crates/scribe-core/tests/`, one per rule (`agenda.rs`, `agenda_layout.rs`, `meeting_match.rs`, `teams_titles.rs`, `armed_meetings.rs`, `switch_prompts.rs`). Business-sentence names, times as named consts, no inline `#[cfg(test)]`.
  - `calendar-graph`: `crates/calendar-graph/tests/*.rs` against a fake `HttpTransport` and a fake `CredentialVault`. No real network; the loopback test connects to its own `127.0.0.1` port.
  - Store: extend `crates/store-markdown/tests/transcript_file.rs` and `transcript_reading.rs`.
  - MCP: extend `apps/scribe-mcp/tests/mcp_tools.rs`.
  - Frontend: `*.spec.tsx` next to each component. Every existing `Popover.spec.tsx` `backend()` must answer every new command.
  - OS glue is excluded in `.cargo/mutants.toml`: `calendar/vault.rs`, `calendar/titles.rs`, the `calendar/*` AppHandle modules, `teams_titles.rs`, `teams_windows.rs`.
- **Code limits:** 20 lines per function, 3 arguments, cognitive complexity 5, clippy pedantic with `-D warnings`, frontend ESLint complexity 5 and max depth 3. Bundle inputs into a plain struct when a rule needs more than three, as `InstallConditions` does `[verified]`. No comments in new code.
- **Privacy in logs:** log only `kind = …` and match states. Never log a subject, name, email, title, token, `code` or `state` value (invariant 4.3).

---

## Slice 1 — Connect an Outlook work account

**Key decision:** This slice builds the `calendar-graph` crate with PKCE sign-in through a single-use `127.0.0.1` listener ([decisions 1](#decisions-made), [4](#decisions-made)), keeps the refresh token in the keychain ([decision 2](#decisions-made)), and reads identity from the `id_token` ([decision 3](#decisions-made)). The tray exposes the connection state to a new Settings row. It also lands ADR 10 and the privacy text for the two new hosts. The `CalendarSource` port is declared here but used in slice 2.

**Structure**

- `crates/scribe-core/src/calendar/mod.rs` (new): `pub mod` re-exports and the consts.
- `crates/scribe-core/src/calendar/connection.rs` (new):
  - `enum Connection { Disconnected, Connecting, Connected(AccountIdentity), ReconnectNeeded }`
  - `struct AccountIdentity { display_name: String, email: String }`
  - `struct Secret(String)`, with `Secret::expose(&self) -> &str` and a `Debug` that prints `Secret(***)`.
- `crates/scribe-core/src/ports.rs` (modify):
  - `trait CredentialVault: Send + Sync { fn load(&self) -> Result<Option<Secret>, VaultError>; fn save(&self, secret: &Secret) -> Result<(), VaultError>; fn forget(&self) -> Result<(), VaultError>; }`
  - `trait CalendarSource: Send + Sync { fn events_between(&self, day: &DayBounds) -> Result<Vec<CalendarEvent>, CalendarError>; }`
  - `DayBounds` and `CalendarEvent` are declared in slice 2. Put `CalendarSource` in this slice only if `OutlookAccount` needs it to compile; otherwise add it in slice 2.
- `crates/scribe-core/src/calendar/error.rs` (new): `VaultError` (`thiserror`, `kind()`), and `CalendarError::{NotConnected, ReconnectNeeded, Unreachable, Malformed}` with `kind() -> &'static str`, mirroring `DownloadError::kind` `[verified]`.
- `crates/calendar-graph/Cargo.toml` (new): dependencies as in the module map. `[lints] workspace = true`.
- `crates/calendar-graph/src/authority.rs` (new): consts `CLIENT_ID`, `AUTHORIZE_URL = "https://login.microsoftonline.com/organizations/oauth2/v2.0/authorize"`, `TOKEN_URL = ".../organizations/oauth2/v2.0/token"`, `SCOPES = "openid profile offline_access Calendars.Read"`. `[verified]` the endpoint shape and the `organizations` tenant value against the Entra auth-code docs.
- `crates/calendar-graph/src/pkce.rs` (new):
  - `struct SignInRequest { verifier: Secret, challenge: String, state: String }`
  - `SignInRequest::generate() -> SignInRequest`, using 32 random bytes encoded as base64url without padding; the challenge is `base64url(sha256(verifier))`.
  - `SignInRequest::authorize_url(&self, redirect: &Url) -> Url`, with params `client_id`, `response_type=code`, `redirect_uri`, `response_mode=query`, `scope`, `state`, `code_challenge`, `code_challenge_method=S256`. `[verified]` against the Entra docs.
  - Random bytes come from `rand::rng().fill_bytes` (rand 0.9). `[inferred]`: check the exact rand 0.9 call name.
- `crates/calendar-graph/src/loopback.rs` (new):
  - `struct LoopbackListener`
  - `LoopbackListener::bind() -> io::Result<LoopbackListener>`, bound to `127.0.0.1:0`.
  - `redirect_uri(&self) -> Url`, in the form `http://127.0.0.1:<port>/`.
  - `wait_for_redirect(self, deadline: Duration) -> Result<RedirectParams, SignInError>`: accepts one connection, answers with a short HTML page ("You can close this tab"), and drops the listener.
  - `enum RedirectParams { Code { code: Secret, state: String }, Refused { error: String, description: String } }`
- `crates/calendar-graph/src/transport.rs` (new):
  - `trait HttpTransport: Send + Sync { fn post_form(&self, url: &str, form: &[(&str, &str)]) -> Result<HttpReply, TransportError>; fn get_json(&self, url: &str, bearer: &Secret) -> Result<HttpReply, TransportError>; }`
  - `struct HttpReply { status: u16, body: String }`
  - `ReqwestTransport` built like `HttpsFetcher::new`: `https_only(true)`, a 10 s connect timeout, and a 30 s timeout `[verified pattern]`. `get_json` sends the header `Prefer: outlook.timezone="UTC"`.
- `crates/calendar-graph/src/token_client.rs` (new):
  - `struct TokenGrant { access: Secret, expires_in: Duration, refresh: Secret, identity: AccountIdentity }`
  - `redeem_code(transport: &dyn HttpTransport, code: &Secret, proof: &CodeProof) -> Result<TokenGrant, SignInError>`, where `CodeProof { verifier, redirect_uri }`.
  - `refresh(transport: &dyn HttpTransport, refresh: &Secret) -> Result<TokenGrant, CalendarError>`
  - Form fields are as in the Entra docs `[verified]`. On refresh, an `invalid_grant` or `interaction_required` error maps to `CalendarError::ReconnectNeeded`, and a transport error maps to `Unreachable`.
- `crates/calendar-graph/src/id_token.rs` (new): `identity_from(id_token: &str) -> Result<AccountIdentity, SignInError>`. It base64url-decodes the payload segment without validating the signature, which is acceptable because the result is only displayed `[verified, Entra docs]`. It reads the claims `name` and `preferred_username` `[inferred]`.
- `crates/calendar-graph/src/sign_in_error.rs` (new): `SignInError::{Declined, AdminApprovalNeeded, TimedOut, StateMismatch, Refused, Unreachable, Vault}` with `kind()` and `plain_reason(&self) -> &'static str`, the copy behind AC 5.1.8.
- `crates/calendar-graph/src/account.rs` (new): `struct OutlookAccount`.
  - `new(transport: Arc<dyn HttpTransport>, vault: Arc<dyn CredentialVault>) -> OutlookAccount`
  - `connect(&self, open_browser: &dyn Fn(&Url)) -> Result<AccountIdentity, SignInError>`
  - `restore(&self) -> Result<Option<AccountIdentity>, CalendarError>`
  - `access_token(&self) -> Result<Secret, CalendarError>`, which refreshes when the token is missing or expires within 60 s.
  - `disconnect(&self) -> Result<(), VaultError>`
  - Holds a `Mutex<Option<CachedAccess { token: Secret, expires_at: Instant }>>`.
- `apps/scribe-tray/src-tauri/src/calendar/vault.rs` (new): `KeyringVault`, which implements `CredentialVault` over `keyring::Entry::new("com.incubyte.scribe", "outlook-refresh-token")`. It uses `set_secret` and `get_secret` with UTF-8 bytes; `Error::NoEntry` maps to `Ok(None)` on load and `Ok(())` on forget. `[verified]`: `Entry::new(&str, &str)`, `delete_credential`, the features `apple-native` and `windows-native`, and binary secrets. `[inferred]`: the exact `set_secret(&[u8])` signature.
- `apps/scribe-tray/src-tauri/src/calendar/connection.rs` (new): `OutlookConnection`, managed state.
  - `state(&self) -> Connection`, `connect_in_background<R>(app: AppHandle<R>)`, `disconnect<R>(app: &AppHandle<R>)`, `restore_in_background<R>(app: AppHandle<R>)`.
  - Single-flight `AtomicBool` like `ModelSetup` `[verified pattern]`.
  - Emits `outlook://connection` with `ConnectionPayload { state: "disconnected"|"connecting"|"connected"|"reconnect_needed", display_name?, email?, error? }`.
- `apps/scribe-tray/src-tauri/src/calendar/commands.rs` (new):
  - `outlook_connection` (sync, returns the payload)
  - `connect_outlook` (async, starts the background sign-in)
  - `disconnect_outlook` (async)
  - Registered in `lib.rs` `with_commands` `[verified]`.
- `apps/scribe-tray/src/popover/outlook/OutlookRow.tsx` and `useOutlookConnection.ts` (new), placed in `SettingsPanel.tsx` like the folder row. Command wrappers go in `commands.ts`, and `isConnectionPayload` goes in `guards.ts`.
- Docs (modify):
  - `docs/ARCHITECTURE.md`: §5 names `login.microsoftonline.com` and `graph.microsoft.com`; §9 adds ADR 10, covering Graph traffic, the keyring choice, and the reason EventKit was rejected.
  - `README.md` Privacy, and `src/privacy/PrivacyStatement.tsx` `STATEMENTS`.
- `.github/workflows/ci.yml` (modify): add `calendar-graph` to `PORTABLE_CRATES` and to the HTTP-client allow-list; add `calendar-graph` to the `scribe-mcp` ban list.
- Root `Cargo.toml`: add the workspace member and `calendar-graph = { path = … }`.

**Collaborators:** The tray constructs `ReqwestTransport`, `KeyringVault` and `OutlookAccount` in `calendar::install` and manages `Arc<OutlookAccount>` and `OutlookConnection`. `OutlookAccount` depends only on core ports and the `HttpTransport` trait. Nothing in `scribe-core` imports `calendar-graph`. The browser opens through `tauri_plugin_opener::OpenerExt::open_url` `[verified]`.

**Control flow:**

- `connect`:
  1. Generate a `SignInRequest` and bind a `LoopbackListener`.
  2. Open the browser with the authorize URL.
  3. Wait for the redirect, up to `SIGN_IN_TIMEOUT`.
  4. Guard: `Refused` maps to `Declined` (`access_denied`) or `AdminApprovalNeeded` (a description containing `AADSTS65001` or `AADSTS90094`), else `Refused`.
  5. Guard: a `state` that differs from the request is `StateMismatch`, and the code is not redeemed.
  6. Redeem the code, save the refresh token to the vault, cache the access token, and return the identity.
- `wait_for_redirect`: set the listener non-blocking and poll `accept` every 200 ms until the deadline, then `TimedOut`. Read one request line, parse its query with `url`, answer, close.
- `restore`: if the vault holds nothing, `Ok(None)`. Otherwise refresh, save the rotated refresh token, and return the identity. `ReconnectNeeded` passes through.
- `access_token`: return the cached token if it is still fresh; otherwise refresh, and on success save the new refresh token (rotation `[verified]`).
- `disconnect`: forget the vault entry, clear the cache, set the state to `Disconnected`, emit the event.
- Tray startup: `restore_in_background`. `Ok(Some)` sets `Connected`, `Ok(None)` sets `Disconnected`, `ReconnectNeeded` sets that state, and `Unreachable` keeps the vault and shows `Connecting` until the next attempt in slice 2's loop.

**Tests:**

- New `crates/calendar-graph/tests/pkce.rs`: the RFC 7636 appendix B vector, where verifier `dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk` yields challenge `E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM` `[verified RFC]`. Also the authorize URL carries the four scopes, S256 and `organizations` (AC 5.1.2, 5.1.4).
- New `crates/calendar-graph/tests/loopback.rs` (Medium: a localhost socket):
  - a matching redirect yields the code;
  - a second connection is refused after the first (AC 5.1.3);
  - a short deadline gives `TimedOut`;
  - the listener address is `127.0.0.1` (AC 5.1.3).
- New `crates/calendar-graph/tests/sign_in.rs`, using a fake transport, a fake vault and a scripted redirect:
  - a state mismatch never calls the token endpoint and saves nothing (AC 5.1.2);
  - success saves the refresh token to the vault and returns the name and email (AC 5.1.1, 5.1.5);
  - `access_denied` gives `Declined` with its plain reason (AC 5.1.8);
  - a refresh refused with `invalid_grant` gives `ReconnectNeeded` (AC 5.1.9);
  - a rotated refresh token replaces the stored one;
  - `restore` with a stored token returns the identity without a browser (AC 5.1.6);
  - `disconnect` empties the vault (AC 5.1.7).
- New `src/popover/outlook/OutlookRow.spec.tsx`:
  - the four states render "Connect Outlook", the account text with "Disconnect", "Reconnect Outlook", and "Outlook didn't connect: …" (AC 5.1.1, 5.1.8, 5.1.9);
  - `vitest-axe` reports no violations (AC 5.1.10).
- Extend each `Popover.spec.tsx` `backend()` so `outlook_connection` answers `disconnected`.

**Standards that bite here:**

- Fail fast at boundaries: the redirect, token JSON and `id_token` are each parsed into types once.
- Security: no secret in logs, files or the webview; PKCE, `state` and loopback per RFC 8252 (spec 6.1.2).
- Clean Code: SRP, so each of PKCE, listener, token client and account stays in its own module.
- Incubyte: test names in business language, ZOMBIES, complexity 5.
- Design Standards: WCAG 2.2 AA on the row.

**Quality checks**

- **No command, event, log line or `settings.json` key ever carries a token, `code`, verifier or `state`. `Secret`'s `Debug` prints `***`, and `ConnectionPayload` has no secret field (AC 5.1.5, invariant 4.3).**
- The scopes string equals `openid profile offline_access Calendars.Read` exactly, and the authority path segment is `organizations` (AC 5.1.4).
- The listener binds `127.0.0.1`, never `localhost` or `0.0.0.0`, serves one request, and closes by `SIGN_IN_TIMEOUT` (AC 5.1.3).
- A token over the Windows secret limit fails as a named `VaultError` kind, never by truncating.
- `scribe-core` gains no dependency on `reqwest`, `keyring`, Tauri or `cfg(target_os)` ([spec §7.2](../specs/009-outlook-calendar-linking.md#72-architectural-style) anti-patterns).
- CI: `calendar-graph` is in `PORTABLE_CRATES` and the HTTP allow-list, and is banned from `scribe-mcp`.
- ARCHITECTURE §5, ADR 10, the README and `PrivacyStatement` name both hosts and what each serves (AC 5.1.11).
- While signed out, Settings and the popover render as before apart from the new row (invariant 4.1).

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite: all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** In the running app: Connect Outlook, sign in with a work account, see the name and email, restart Scribe and still see them, then Disconnect.
3. **Accessibility.** Same session: Tab reaches Connect and Disconnect, focus is visible, the buttons have accessible names, and contrast holds in light and dark themes.

**Open at build time (AC 5.1.4, 5.1.5, and spec OQ 8.1–8.3):** `CLIENT_ID` comes from the Entra app registration, which needs `http://127.0.0.1` added as a redirect through the manifest and has its owner still pending (OQ 8.3). Before writing slice 3, the builder signs in once against a real tenant and checks three things:
- whether consent succeeds without an admin (OQ 8.1);
- whether a `calendarView` call with the slice 2 `$select` returns `attendees` and `organizer` (OQ 8.2);
- the refresh token's byte length on Windows.

Record the results in the Escalation log. If attendees are withheld, change `SCOPES` to `Calendars.Read`, and update ADR 10 and spec AC 5.1.4.

---

## Slice 2 — See today's meetings in the popover

**Key decision:** This slice adds a pure filter and an overlap layout to the core ([decision 5](#decisions-made)), a Graph `calendarView` client that implements `CalendarSource`, and an agenda thread in the tray that refreshes on sign-in, every 5 minutes, on wake and on popover focus. The `todays_agenda` command returns the cached, laid-out agenda. It never calls Graph on the main thread.

**Structure**

- `crates/scribe-core/src/calendar/meeting.rs` (new):
  - `struct EventId(String)`
  - `struct Person { name: String, email: String }`
  - `enum Response { Organizer, Accepted, Tentative, None, Declined }`
  - `struct Meeting { event_id, subject, starts_at: DateTime<FixedOffset>, ends_at, organizer: Person, attendees: Vec<Person>, is_teams: bool, is_recurring: bool, response: Response }`
  - `enum ShowAs { Free, Tentative, Busy, Away, WorkingElsewhere, Unknown }`
  - `struct CalendarEvent { meeting: Meeting, is_all_day: bool, is_cancelled: bool, show_as: ShowAs }`
  - `struct DayBounds { start: DateTime<FixedOffset>, end: DateTime<FixedOffset> }`
- `crates/scribe-core/src/calendar/agenda.rs` (new):
  - `struct TodaysAgenda(Vec<Meeting>)`
  - `todays_agenda(events: Vec<CalendarEvent>, day: &DayBounds) -> TodaysAgenda`: excludes all-day, cancelled, declined and `Free` events; sorts by start; keeps events that overlap the day.
  - `TodaysAgenda::meetings(&self) -> &[Meeting]`
- `crates/scribe-core/src/calendar/layout.rs` (new):
  - `struct PlacedMeeting { meeting: Meeting, top_minutes: u32, height_minutes: u32, column: u8, columns: u8 }`
  - `struct OverflowChip { top_minutes: u32, height_minutes: u32, hidden: Vec<EventId> }`
  - `struct AgendaLayout { placed: Vec<PlacedMeeting>, overflow: Vec<OverflowChip> }`
  - `lay_out(agenda: &TodaysAgenda, day: &DayBounds) -> AgendaLayout`
  - consts `MIN_BLOCK_MINUTES = 15`, `MAX_COLUMNS = 2`.
- `crates/calendar-graph/src/calendar_view.rs` (new): `calendar_view_url(day: &DayBounds) -> Url`. It sets `startDateTime` and `endDateTime` (RFC 3339), `$top=100`, and `$select=id,subject,start,end,organizer,attendees,isAllDay,isCancelled,onlineMeetingProvider,type,seriesMasterId,responseStatus,showAs`, with no `body` (AC 5.2.10). `[verified]` that these property names exist on `event`.
- `crates/calendar-graph/src/graph_events.rs` (new): private serde DTOs, plus `events_from_page(body: &str, offset: FixedOffset) -> Result<GraphPage, CalendarError>` where `GraphPage { events: Vec<CalendarEvent>, next_link: Option<String> }`.
  - Mapping rules:
    - `onlineMeetingProvider == "teamsForBusiness"` sets `is_teams`.
    - `type` of `occurrence` or `exception` sets `is_recurring`.
    - `responseStatus.response` maps `organizer`, `accepted`, `tentativelyAccepted`, `declined`, and `none` or `notResponded`.
    - `showAs` maps its six values.
    - `dateTime` is parsed as a naive UTC time (`%Y-%m-%dT%H:%M:%S%.f`) and shifted to `offset`.
  - `[verified]`: the enum values and the fact that times are UTC by default. `[inferred]`: the 7-digit fractional format.
  - A malformed event is skipped and logged with `kind` only (spec 6.1.5).
- `crates/calendar-graph/src/account.rs` (modify): `impl CalendarSource for OutlookAccount`, which follows `@odata.nextLink` for up to 5 pages and retries once after a 401 with a forced refresh.
- `apps/scribe-tray/src-tauri/src/calendar/agenda.rs` (new):
  - `AgendaCache` (managed): `Mutex<AgendaSnapshot>` with `AgendaSnapshot { meetings: Option<TodaysAgenda>, fetched_at: Option<DateTime<FixedOffset>>, failed: bool }`.
  - `install<R>(app)`: starts a named `agenda` thread with `refresh_forever` and a `tick`, mirroring `updates/mod.rs` `[verified]`.
  - `nudge(&self)`: an `AtomicBool` the loop reads, set on wake through `power::on_did_wake`, on sign-in, and by the `refresh_agenda` command.
  - `today_bounds(now: DateTime<Local>) -> DayBounds`
  - Emits `agenda://updated`.
- `apps/scribe-tray/src-tauri/src/calendar/commands.rs` (modify):
  - `todays_agenda` (sync, reads the cache only) returns `AgendaView { status: "ready"|"loading"|"failed", fetched_at?, blocks: [BlockView], overflow: [ChipView] }`, where `BlockView { event_id, subject, starts_at, ends_at, is_teams, is_recurring, top_minutes, height_minutes, column, columns, has_ended }`.
  - `refresh_agenda` (sync; it only nudges the thread).
- Frontend (new):
  - `src/popover/agenda/TodayAgenda.tsx`, `AgendaBlock.tsx`, `OverflowChip.tsx`, `useTodayAgenda.ts`
  - `Popover.tsx` (modify): render `TodayAgenda` when the connection is `connected`, else `RecentSessions` unchanged.
  - Styles in `theme/popover.css` under `agenda__*`, using the tokens `--recording` (the now line), `--text-muted` plus a "past" text cue, `--hairline` and `--focus-ring`.

**Collaborators:** The agenda thread holds `Arc<OutlookAccount>` as `Arc<dyn CalendarSource>`, the `AgendaCache` and the `OutlookConnection`. On `CalendarError::ReconnectNeeded` it sets the connection state, and on `Unreachable` it marks the snapshot `failed` but keeps its meetings (AC 5.2.9). The core receives only `DayBounds` and events. The webview receives only `AgendaView`.

**Control flow:**

- Each `tick`:
  - Guard: if not connected, skip.
  - Fetch `events_between(today_bounds(now))`.
  - Ok: build the agenda, store it with `fetched_at = now` and `failed = false`, and emit.
  - Err: set `failed = true`, keep the previous meetings, emit, and log the `kind`.
- The loop waits `AGENDA_REFRESH_INTERVAL` or until a nudge, whichever comes first, checking the flag every second like the update loop `[inferred]`.
- `lay_out`:
  1. Group meetings into clusters with overlapping time ranges.
  2. Within a cluster, give each meeting the lowest free column.
  3. Meetings past column `MAX_COLUMNS` go into the cluster's `OverflowChip`.
  4. Height is `max(duration, MIN_BLOCK_MINUTES)`, and top is minutes since `day.start`, clamped to the day.
- Frontend:
  - On mount and on window `focus`, call `refresh_agenda` and then `todays_agenda`; also listen for `agenda://updated`.
  - On first render, scroll so the now-minute sits at a third of the viewport height (AC 5.2.6).
  - Re-render the now line every minute using the existing fake-`Date`-friendly timer pattern.

**Tests:**

- New `crates/scribe-core/tests/agenda.rs`:
  - all-day, cancelled, declined and free events are left out, and a tentative one stays (AC 5.2.2);
  - recurring occurrences stay (AC 5.2.2);
  - an empty input gives an empty agenda (AC 5.2.8).
- New `crates/scribe-core/tests/agenda_layout.rs`:
  - a 10-minute meeting gets a 15-minute height (AC 5.2.3);
  - two overlapping meetings get two columns (AC 5.2.4);
  - a third overlapping meeting goes into a chip (AC 5.2.4);
  - top follows the start in minutes since midnight (AC 5.2.3).
- New `crates/calendar-graph/tests/calendar_view.rs`:
  - `$select` lists exactly the fields and never `body` (AC 5.2.10);
  - JSON fixture events map to `CalendarEvent` with the Teams and recurring flags and the local offset;
  - a malformed event is skipped and the rest survive (spec 6.1.5);
  - `nextLink` paging is followed;
  - a 401 triggers one refresh.
- New `src/popover/agenda/TodayAgenda.spec.tsx`:
  - the heading reads "Today · Mon 26" (AC 5.2.1);
  - "No meetings today" (AC 5.2.8);
  - a failed fetch with no data shows "Couldn't load your calendar" and Retry calls `refresh_agenda` (AC 5.2.9);
  - stale data shows "Updated <time>" (AC 5.2.9);
  - blocks carry accessible names in the form "<subject>, <start>–<end>, Teams meeting, recurring", in time order (AC 5.2.5, 5.2.11);
  - a past block is labelled as past, not only dimmed (AC 5.2.11);
  - the chip lists its hidden meetings (AC 5.2.4);
  - `vitest-axe` reports no violations.
- Extend `Popover.spec.tsx`: Recent sessions while disconnected, Today while connected (AC 5.2.1). Every `backend()` answers `todays_agenda` and `refresh_agenda`.

**Standards that bite here:**

- Separation of concerns: filter and layout in the core (spec 6.3.1).
- Fail fast at boundaries: Graph DTOs.
- Complexity 5: split `lay_out` into a cluster step and a column step.
- Design Standards: contrast-in-context on `--surface-sunken`; colour is never the only cue.
- `.spec.tsx` naming.

**Quality checks**

- **The `todays_agenda` command performs no network or keychain I/O; it reads `AgendaCache` only ([context §7](../specs/009-outlook-calendar-linking-context.md#7-hot-paths-and-performance-landmines)).**
- `scribe-core` reads no clock. Every rule takes `DayBounds` or a `DateTime` argument.
- The `$select` string has no `body`, `bodyPreview` or `attachments` (AC 5.2.10, invariant 4.2).
- A failed refresh never clears the last good agenda (AC 5.2.9, invariant 4.4).
- Signed out, the popover renders `RecentSessions` with unchanged markup (invariant 4.1, [spec §9.4](../specs/009-outlook-calendar-linking.md#9-deferred-follow-ups-and-accepted-limits)).
- No subject appears in any log line.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite: all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** While connected, open the popover: today's meetings show on the time-grid with the now line in the top third and overlapping meetings side by side, and an edit in Outlook appears within 5 minutes.
3. **Accessibility.** Same session: Tab moves through blocks in time order, focus is visible on blocks and the chip, the announced names include the times, and contrast holds in both themes.

---

## Slice 3 — Save the meeting's name and invitees with each transcript

**Key decision:** This slice adds the core candidate and ranking rule ([decision 8](#decisions-made)) and an optional `meeting` field on `SessionHeader`, written and read as an optional front-matter block. The tray records the call's wall-clock start at the rising edge, matches when a recording starts, keeps the choice in `MeetingSelection`, and writes it with `link_meeting` when `RecordingStopped` arrives ([decision 7](#decisions-made)). The MCP listing gains a `meeting` field.

**Structure**

- `crates/scribe-core/src/calendar/link.rs` (new):
  - `struct MeetingLink { event_id: EventId, subject: String, starts_at, ends_at, organizer: Person, attendees: Vec<Person>, attendee_count: usize }`
  - `MeetingLink::from_meeting(meeting: &Meeting) -> MeetingLink`, which caps `attendees` at `MAX_LISTED_ATTENDEES` and sets `attendee_count` to the full count (AC 5.3.5).
- `crates/scribe-core/src/calendar/matching.rs` (new):
  - `struct MatchAnchor(DateTime<FixedOffset>)`
  - `struct MatchInputs<'a> { agenda: &'a TodaysAgenda, anchor: MatchAnchor }`. Slice 4 adds `titles` and slice 6 adds `armed`.
  - `enum MatchOutcome { Matched(Meeting), Ambiguous(Vec<Meeting>), Unmatched }`, where `Ambiguous` is ranked best first.
  - `meeting_candidates(agenda: &TodaysAgenda, anchor: &MatchAnchor) -> Vec<&Meeting>`: a meeting qualifies when `start - CANDIDATE_LEAD <= anchor < end`.
  - `match_meeting(inputs: &MatchInputs) -> MatchOutcome`
  - `fn outranks(a: &Meeting, b: &Meeting) -> bool`, using rules 1–2 only.
  - Rank order: rule 1 is `is_teams`; rule 2 is `Organizer|Accepted > Tentative > None`; rule 3 is the smallest gap between start and anchor.
- `crates/scribe-core/src/recording/session.rs` (modify):
  - `SessionHeader.meeting: Option<MeetingLink>`
  - `SessionHeader::recording` sets `None`.
  - `with_meeting(self, link: MeetingLink) -> SessionHeader`, which also sets `title = link.subject`.
  - `SessionSummary.meeting: Option<MeetingLink>`, carried by `summary()`.
  - Update the five struct-literal sites listed in [context §1](../specs/009-outlook-calendar-linking-context.md#1-affected-modules) with `meeting: None`.
- `crates/scribe-core/src/library/summary_view.rs` (modify): `SessionSummaryView.meeting: Option<MeetingSummaryView { subject, organizer, attendees, attendee_count }>` behind the existing `serde` feature `[verified]`.
- `crates/store-markdown/src/format.rs` (modify):
  - `render_header` appends the `meeting:` block in the shape of [spec §3.2](../specs/009-outlook-calendar-linking.md#32-kinds-of-data), with every string through `quoted` and times through `timestamp` `[verified]`.
  - `parse_header` reads `meeting` as an optional `serde_saphyr::Value`, which `meeting_from(value) -> Option<MeetingLink>` maps leniently: any missing or mistyped field gives `None` and a `warn!(kind = "malformed_meeting")`.
  - `[inferred]`: check that `serde_saphyr` exposes a generic `Value`. If it doesn't, deserialize into `Option<MeetingFields>` with every field `Option` and `#[serde(default)]`.
- `crates/store-markdown/src/writer.rs` (modify): `MarkdownTranscriptStore::link_meeting(&self, id: &SessionId, link: MeetingLink) -> Result<(), StoreError>` uses `rewrite_header(path, |header| header.with_meeting(link))` `[verified]`. It is an inherent method, not a port method ([decision 7](#decisions-made)).
- `apps/scribe-tray/src-tauri/src/call_watch.rs` (modify): `CallPresence` gains `call_started_at: Mutex<Option<DateTime<FixedOffset>>>`, set to `Local::now()` on the rising edge and cleared when the call ends, plus `call_started_at(&self) -> Option<DateTime<FixedOffset>>`.
- `apps/scribe-tray/src-tauri/src/calendar/selection.rs` (new): `MeetingSelection` (managed).
  - `Mutex<Selection { session: Option<SessionId>, chosen: Option<Meeting>, auto_started: bool }>`
  - Methods: `on_recording_started<R>(app, id)`, `take_for(&self, id: &SessionId) -> Option<Meeting>`, `choose(&self, meeting: Option<Meeting>)`.
  - `anchor_now<R>(app) -> MatchAnchor` uses the call start while in a call, else now.
- `apps/scribe-tray/src-tauri/src/tray_listener.rs` (modify):
  - `started` calls `MeetingSelection::on_recording_started` `[inferred]`; check that the `RecordingStarted` handler is named `started`.
  - `stopped` calls `calendar::link_finished_session(app, id)` after `stop_session`.
- `apps/scribe-mcp/src/requests.rs` (modify): `TranscriptSummaryDto.meeting: Option<MeetingDto>` from `SessionSummaryView` (AC 5.3.7). Update the `list_transcripts` tool description in `server.rs`.

**Collaborators:** `MeetingSelection` reads `AgendaCache` and `CallPresence` and calls the core `match_meeting`. `link_finished_session` reads the selection and calls `MarkdownTranscriptStore::link_meeting` from the tray-listener thread, never under the `RecordingController` mutex. `scribe-mcp` gains nothing beyond what `store-markdown` and `scribe-core` already give it.

**Control flow:**

- `on_recording_started`:
  - Guard: if not connected, or no agenda is cached, record `chosen = None` (AC 5.3.8).
  - Otherwise compute the anchor and `match_meeting`. `Matched` sets `chosen`; `Ambiguous` and `Unmatched` set `None` (AC 5.3.4).
  - Log `match = matched|ambiguous|unmatched` only (AC 5.3.9).
- `link_finished_session(id)`: take the selection for `id`. If a meeting was chosen, call `link_meeting` and log a failure by `kind`; the transcript stays valid without the block (invariant 4.4). Clear the selection.
- `match_meeting`: get the candidates; none gives `Unmatched`, one gives `Matched`; otherwise sort by rank, and if the first `outranks` the second, `Matched`, else `Ambiguous(ranked)`.

**Tests:**

- New `crates/scribe-core/tests/meeting_match.rs`:
  - a call at 10:58 matches the 11:00 Teams meeting (AC 5.3.1);
  - an anchor at 11:05 still matches the 11:00 meeting at an evaluation time of 11:40, because only the anchor is passed (AC 5.3.2);
  - no candidates gives Unmatched (AC 5.3.3);
  - two Teams meetings both accepted, separated only by start gap, give Ambiguous (AC 5.3.4);
  - Teams outranks non-Teams, and accepted outranks tentative (ranking rules);
  - the boundary: an anchor exactly 10 minutes before start qualifies, and at the meeting's end it doesn't.
- New `crates/scribe-core/tests/meeting_link.rs`: 60 attendees list 50 with a count of 60 (AC 5.3.5).
- Extend `crates/store-markdown/tests/transcript_file.rs`:
  - a linked transcript renders a `meeting` block that parses back equal (AC 5.3.5);
  - a subject containing `:` and `"` round-trips;
  - `link_meeting` sets the title to the subject (AC 5.3.1).
- Extend `crates/store-markdown/tests/transcript_reading.rs`:
  - a fixture written before this spec parses unchanged (AC 5.3.6);
  - a malformed `meeting` block parses as a transcript without a meeting (spec 6.1.5).
- Extend `apps/scribe-mcp/tests/mcp_tools.rs`: `list_transcripts` includes `meeting` for a linked transcript and omits it for an old one (AC 5.3.6, 5.3.7).

**Standards that bite here:**

- Fail fast at boundaries: lenient parsing at the front-matter edge.
- Clean Code: command-query separation, so `match_meeting` is a pure query.
- DRY: reuse `quoted`, `timestamp` and `rewrite_header`.
- Engineering standards: ZOMBIES boundaries on the 10-minute lead.

**Quality checks**

- **Old transcripts and transcripts without `meeting` parse byte-for-byte as before; `render_header` emits no `meeting` key when the field is `None` (AC 5.3.6, invariant 4.6).**
- The match uses only `MatchAnchor`; neither `link_finished_session` nor `match_meeting` reads "now" (invariant 4.5).
- A store failure in `link_meeting` leaves the finalised transcript in place and readable (invariant 4.4).
- Log lines in `selection.rs` contain only the match state and error kinds (AC 5.3.9).
- `TranscriptStore` and `InMemoryTranscriptStore` signatures are unchanged.
- `CALL_END_GRACE` and the `CallWatch` behaviour are unchanged ([spec §10.1](../specs/009-outlook-calendar-linking.md#10-established-behaviour--documented-not-changed)).

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite: all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** While connected, start a recording during a calendar meeting and stop it. The transcript file is titled with the subject and has the `meeting` block, and Claude's `list_transcripts` shows it.

---

## Slice 4 — Recognise the meeting from the Teams window title

**Key decision:** This slice adds a pure title rule to the core and a `titles` input to `MatchInputs`, which takes precedence over ranking ([decision 8](#decisions-made)). Two OS readers implement `MeetingWindowTitles` next to the code that already identifies Teams ([decision 6](#decisions-made)). Titles are read once, at the call-start edge, and passed into the match the next recording uses.

**Structure**

- `crates/scribe-core/src/calendar/titles.rs` (new):
  - `meeting_named_in_titles<'a>(candidates: &[&'a Meeting], titles: &[String]) -> Option<&'a Meeting>`
  - `fn normalized(text: &str) -> String`, which trims and lowercases.
  - A title names a candidate when `normalized(title)` contains `normalized(subject)` and the subject isn't empty. A title that names two or more candidates is ignored. A result is returned only when exactly one candidate is named across all titles (AC 5.4.2).
  - Teams suffixes need no stripping because the check is containment.
- `crates/scribe-core/src/calendar/matching.rs` (modify): `MatchInputs.titles: &'a [String]`. `match_meeting` checks the title rule before ranking.
- `crates/scribe-core/src/call/mod.rs` (modify): `pub const TEAMS_BUNDLE_ID_PREFIX: &str = "com.microsoft.teams"`, moved from `audio-macos` `[verified]`, which now imports it.
- `crates/scribe-core/src/ports.rs` (modify): `trait MeetingWindowTitles: Send + Sync { fn teams_window_titles(&self) -> Vec<String>; }`
- `crates/capture-macos/src/teams_titles.rs` (new): `CaptureTeamsTitles`, which implements the port.
  - Guard: `cg::screen_capture_access::preflight()`. If false, return empty; never call `request` (AC 5.4.3).
  - Read the shareable windows, keep those whose owning app bundle id starts with `TEAMS_BUNDLE_ID_PREFIX` and that are on screen, and return their non-empty `title()`.
  - `[inferred]`: the cidre names `sc::Window::title`, `RunningApp::bundle_id`, and how `outlines.rs` obtains `ShareableContent`. Mirror that call exactly.
- `crates/audio-windows/src/teams_windows.rs` (new): `WasapiTeamsTitles`, which implements the port. `EnumWindows` collects visible top-level windows with `GetWindowThreadProcessId` and `GetWindowTextW`, and keeps those for which `ProcessTable::snapshot()?.belongs_to_teams(pid)` holds `[verified]` (the table is in the same crate, so it can become `pub(crate)`). Add the `Win32_UI_WindowsAndMessaging` feature to the crate's `windows` dependency. The crate already has `#![allow(unsafe_code)]` `[verified]`.
- `apps/scribe-tray/src-tauri/src/calendar/titles.rs` (new): `teams_titles() -> Box<dyn MeetingWindowTitles>`, with a per-OS `mod` and an `unsupported` fallback that returns an empty list, like `call_probes.rs` `[verified pattern]`.
- `apps/scribe-tray/src-tauri/src/call_watch.rs` (modify): on the rising edge, while connected, read the titles once and store them in `CallPresence.call_titles: Mutex<Vec<String>>`, cleared when the call ends. `MeetingSelection` passes them into `MatchInputs`.

**Collaborators:** The core rule has no OS knowledge. The adapters depend on `scribe-core` for the port and the bundle const. The tray picks the adapter per OS. Titles live in memory only.

**Control flow:**

- Rising edge:
  - Guard: if not connected, read nothing.
  - Read the titles once and store them.
- `match_meeting`: candidates, then `meeting_named_in_titles`. If it returns `Some`, `Matched` it. Otherwise the existing ranking.
- Windows reader: the snapshot fails → empty list; a window with an empty title → skip; a pid outside Teams → skip.

**Tests:**

- New `crates/scribe-core/tests/teams_titles.rs`:
  - "Anthara Standup | Microsoft Teams" picks Anthara Standup over the overlapping DXC Review (AC 5.4.1);
  - the match is case- and whitespace-insensitive (AC 5.4.2);
  - one title naming both candidates decides nothing (AC 5.4.2);
  - no titles falls back to ranking (AC 5.4.3);
  - an empty subject never matches.
- Extend `meeting_match.rs`: a title outranks the ranking rules.
- The OS readers have no automated tests; Windows is clippy-only in CI ([context §5](../specs/009-outlook-calendar-linking-context.md#5-known-dragons)). They are covered by the manual verification below and excluded from mutants.

**Standards that bite here:**

- Security: read only windows owned by Teams (spec §7.5).
- Clean Code: one owner for the Teams identity (DRY), shared through the core const.
- Separation of concerns.

**Quality checks**

- **The macOS reader never calls a permission `request`, and no notification or prompt appears without Screen Recording access (AC 5.4.3).**
- Titles are read at the rising edge only in this slice (slice 7 adds the 30-second recheck), never on every poll, and never logged or persisted (AC 5.4.5).
- Only windows whose owner matches the Teams bundle prefix or the Teams process ancestry are returned; browsers are excluded (AC 5.4.4).
- `audio-macos` call detection behaves identically after the const moves.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite: all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** On real hardware, with two overlapping calendar meetings, join one in Teams and record. The transcript links to the meeting Teams is showing. Repeat on macOS without Screen Recording access: the link falls back silently, with no prompt.

**Open at build time (AC 5.4.1, spec OQ 8.4):** the real format of new Teams meeting-window titles. Probe: during a Teams meeting on macOS and Windows, log the Teams window titles once to the console in a scratch build, never committed. Confirm the subject appears. If it doesn't, the rule still stands and simply never fires; record the finding in the Escalation log and in spec §8.4.

---

## Slice 5 — Confirm or correct the meeting when a call starts

**Key decision:** At the call-start edge, the tray runs the same `match_meeting` with the call start as anchor and publishes the result. It sends a `meeting://match` event and answers a `call_meeting` snapshot command, so a popover opened later catches up ([context §4c](../specs/009-outlook-calendar-linking-context.md#4c-slice-traps)). `choose_meeting` writes into slice 3's `MeetingSelection` ([decision 7](#decisions-made)). The pending call choice is adopted when recording starts, and while recording it replaces the session's choice.

**Structure**

- `apps/scribe-tray/src-tauri/src/calendar/selection.rs` (modify):
  - `Selection` gains `pending_call: Option<CallMatch>`, where `CallMatch { outcome: MatchOutcome, chosen: Option<Meeting> }`.
  - `on_call_started<R>(app)` and `on_call_ended(&self)`.
  - `choose_by_id(&self, id: Option<&EventId>) -> Result<(), ChooseError>`: the id must be one of the current candidates; `None` means "Not a calendar meeting".
  - `on_recording_started` adopts `pending_call.chosen` when present instead of re-matching (AC 5.5.2).
- `apps/scribe-tray/src-tauri/src/calendar/commands.rs` (modify):
  - `call_meeting` returns `CallMeetingView { state: "matched"|"ambiguous"|"unmatched", selected_event_id?, candidates: [{ event_id, subject, starts_at, ends_at }] }`.
  - `choose_meeting(event_id: Option<String>)`.
  - Emits `meeting://match` with the same payload.
- `apps/scribe-tray/src-tauri/src/call_watch.rs` (modify): the rising edge calls `on_call_started` before `open_unless_recording`; the call end calls `on_call_ended`.
- Frontend (new): `src/popover/meeting/MeetingLine.tsx` and `useCallMeeting.ts`, rendered under `StatusBar` in the idle call-start view and in `LiveTranscript`'s header.
  - Matched: "DXC Review · 11:00–11:45 ▾".
  - Ambiguous: "Which meeting is this?".
  - Unmatched: renders nothing.
  - Opening the line shows a `<fieldset>` radio group with the candidates plus "Not a calendar meeting".
  - Wrappers go in `commands.ts`, `isCallMeetingView` in `guards.ts`.

**Collaborators:** `call_watch.rs` calls into `MeetingSelection`. The commands read and write `MeetingSelection` only. The `RecordingController` API is unchanged, and `start_recording` keeps taking no arguments.

**Control flow:**

- `on_call_started`:
  - Guard: if not connected, `pending_call = None`.
  - Otherwise match with the call-start anchor and titles. `Matched` sets `chosen`; `Ambiguous` sets `chosen` to the top-ranked candidate (AC 5.5.1); `Unmatched` sets `None`.
  - Emit.
- `choose_meeting`:
  - While recording, replace the session's `chosen`; the latest choice wins (AC 5.5.3).
  - Otherwise replace `pending_call.chosen`.
  - Emit.
- `on_recording_started`: if `pending_call` exists, adopt its `chosen` (`None` stays `None`); otherwise run slice 3's matching.

**Tests:**

- New `src/popover/meeting/MeetingLine.spec.tsx`:
  - Matched shows the subject and time range (AC 5.5.1);
  - Ambiguous shows "Which meeting is this?" with the top candidate checked (AC 5.5.1);
  - choosing a candidate calls `choose_meeting` with its id, and "Not a calendar meeting" sends null (AC 5.5.2);
  - Unmatched renders nothing (AC 5.5.4);
  - the line appears in the live view and a change there calls `choose_meeting` (AC 5.5.3);
  - it is a radio group with a legend, the arrow keys move the selection, and `vitest-axe` is clean (AC 5.5.5).
- Extend `Popover.spec.tsx`: every `backend()` answers `call_meeting` with `unmatched`, and the call-start popover is otherwise unchanged (AC 5.5.4).
- The selection behaviour is logic in the tray shell. Extract `fn adopt(pending: Option<CallMatch>, fallback: impl FnOnce() -> Option<Meeting>) -> Option<Meeting>` as a pure function in `selection.rs` and test it from `src-tauri/tests/meeting_selection.rs`, like `start_failure.rs` `[verified pattern]` (AC 5.5.2, 5.5.3).

**Standards that bite here:**

- Design Standards: native radio inputs, a visible `--focus-ring`, and accessible names.
- Clean Code: no flag arguments, so `choose(None)` is an explicit "not a meeting" value.
- Map handlers and aria attributes when inserting into `StatusBar` and `LiveTranscript`.

**Quality checks**

- **With an Unmatched or signed-out call, the call-start popover's DOM is unchanged (AC 5.5.4, invariant 4.1).**
- `choose_meeting` rejects an event id that isn't a current candidate.
- The candidate payload carries the subject and times only, never attendees, and is never logged.
- The `open_for_call` and `open_unless_recording` behaviour is unchanged when signed out.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite: all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** Start a Teams call during two overlapping meetings. The popover asks "Which meeting is this?". Pick one, record, switch to the other in the live view, stop, and the transcript carries the last choice.
3. **Accessibility.** Same session: arrow keys move through the radio group, focus is visible, and the selected meeting is announced.

---

## Slice 6 — Arm a meeting to record by itself

**Key decision:** This slice adds a pure `ArmedWatch` state machine to the core that the 2-second call-watch poll consults on every tick ([decision 9](#decisions-made)). Armed ids persist in `settings.json` under `armed_meetings`, pruned by a pure function. An auto-start goes through `RecordingController::start` and marks `auto_started` in `MeetingSelection`. The call end then stops only auto-started recordings. An armed candidate wins the match when no title decides ([decision 8](#decisions-made)).

**Structure**

- `crates/scribe-core/src/calendar/armed.rs` (new):
  - `struct ArmedEntry { event_id: EventId, date: NaiveDate }`
  - `ArmedMeetings::for_day(entries: Vec<ArmedEntry>, today: NaiveDate) -> ArmedMeetings`, which prunes earlier days.
  - `is_armed(&self, id: &EventId) -> bool`, `arm(&mut self, id: EventId, today: NaiveDate)`, `disarm(&mut self, id: &EventId)`, `entries(&self) -> &[ArmedEntry]`.
  - `enum ArmedState { Armed, Recording, Recorded, Missed }`
  - `ArmedWatch` holds `HashMap<EventId, ArmedState>`.
  - `struct WatchInputs<'a> { agenda: &'a TodaysAgenda, armed: &'a ArmedMeetings, matched: Option<&'a Meeting>, in_call: bool, recording: bool, now: DateTime<FixedOffset> }`
  - `enum ArmedAction { StartRecording(Meeting), ReportMissed(Meeting) }`
  - `next_actions(&mut self, inputs: &WatchInputs) -> Vec<ArmedAction>`
  - `note_recording(&mut self, id: &EventId)`, `note_recorded(&mut self, id: &EventId)`, `note_start_failed(&mut self, id: &EventId)`.
  - `const MISSED_AFTER: Duration = 5 min`.
- `crates/scribe-core/src/calendar/matching.rs` (modify): `MatchInputs.armed: &'a ArmedMeetings`; an armed candidate is matched after the title rule and before ranking (AC 5.6.3).
- `apps/scribe-tray/src-tauri/src/calendar/armed.rs` (new): `ArmedStore` (managed).
  - `load<R>(app) -> ArmedMeetings` reads `armed_meetings`, prunes, and writes back if anything was pruned (AC 5.6.2).
  - `arm<R>(app, id)` and `disarm<R>(app, id)`, with the key shape following `privacy.rs` `[verified pattern]`.
  - Holds `Mutex<ArmedWatch>`.
- `apps/scribe-tray/src-tauri/src/calendar/notify.rs` (new): `notify_meeting_recording(app, subject)`, `notify_meeting_saved(app, subject)`, `notify_meeting_missed(app, subject)`, `notify_meeting_not_started(app, subject)`, all through `notifications::show_plain` `[verified]`.
- `apps/scribe-tray/src-tauri/src/calendar/commands.rs` (modify):
  - `arm_meeting(event_id)` and `disarm_meeting(event_id)`.
  - `record_meeting(event_id)`, the Missed block's Record button: it starts a recording and sets the session's choice to that meeting.
  - `BlockView` gains `armed: bool` and `armed_state?: "missed"`.
- `apps/scribe-tray/src-tauri/src/call_watch.rs` (modify):
  - Each poll calls `calendar::on_poll(app, now)`, which builds `WatchInputs` from the caches and performs the actions.
  - The call end also calls `calendar::stop_if_auto_started(app)`.
- `apps/scribe-tray/src-tauri/src/tray_listener.rs` (modify): `stopped` calls `notify_meeting_saved` when the stopped session was auto-started and linked (AC 5.6.4), and `ArmedWatch::note_recorded`.
- `apps/scribe-tray/src-tauri/src/recording.rs` (modify): `report_start_failure` stays the single failure path. Add a `start_for_meeting<R>(app, meeting, auto_started: AutoStart)` that wraps `start` and records the choice. `AutoStart` is a two-variant enum, not a bool flag.
- Frontend (modify): `AgendaBlock.tsx` gains a switch (`<input type="checkbox" role="switch">` named "Record <subject> automatically") on blocks that haven't ended, and on Missed blocks the text "No call detected" plus a Record button.

**Collaborators:** `on_poll` reads `CallPresence`, `AgendaCache`, `ArmedStore`, `MeetingSelection` and `RecordingController::state`, and writes through `RecordingController`, `MeetingSelection` and `notify`. The core `ArmedWatch` sees only `WatchInputs`. Never hold the `RecordingController` mutex while notifying ([context §7](../specs/009-outlook-calendar-linking-context.md#7-hot-paths-and-performance-landmines)).

**Control flow:**

- `next_actions`, for each armed meeting in the agenda:
  - Guard: skip a meeting that has ended or is already `Recording` or `Recorded`.
  - If `in_call && !recording && matched == this meeting && now >= start - CANDIDATE_LEAD`, emit `StartRecording` and move to `Recording`.
  - Otherwise, if the state is `Armed && now >= start + MISSED_AFTER && !(in_call && matched == this)`, emit `ReportMissed` and move to `Missed`.
- `on_poll`:
  - Guard: if not connected, do nothing.
  - Match with the call-start anchor when in a call.
  - Run `next_actions`.
  - `StartRecording` calls `start_for_meeting(.., AutoStart::Yes)`. On success, notify "Recording <subject>" (AC 5.6.3). On failure, `report_start_failure` names the meeting, `note_start_failed` moves it to `Missed` (AC 5.6.7), and the UI shows Record.
  - `ReportMissed` calls `notify_meeting_missed` (AC 5.6.5).
- When the call ends: if the active session is `auto_started`, stop it; the existing `RecordingStopped` path then links it and posts "Saved" (AC 5.6.4). Manual recordings are untouched.
- A manual recording, a `record_meeting` recording, or an adopted call match linked to an armed meeting calls `note_recording`, so auto-start never fires after it.

**Tests:**

- New `crates/scribe-core/tests/armed_meetings.rs`:
  - arming stores today's date, and entries from yesterday are pruned on read (AC 5.6.2);
  - a call in the armed meeting's window starts recording (AC 5.6.3);
  - no call means no start, even at the meeting's start (AC 5.6.6, invariant 4.7);
  - five minutes after the start with no call reports Missed once (AC 5.6.5);
  - a call still in progress at the first poll after wake starts recording when the meeting hasn't ended (AC 5.6.8);
  - an ended meeting never starts;
  - after a recording finishes, a continuing call doesn't restart it;
  - a failed start moves to Missed (AC 5.6.7);
  - a meeting already recording manually isn't started again.
- Extend `meeting_match.rs`: an armed candidate wins over ranking when no title decides; a title still beats armed (AC 5.6.3).
- Extend `src/popover/agenda/TodayAgenda.spec.tsx`:
  - the switch appears only on meetings that haven't ended, and is named "Record <subject> automatically" (AC 5.6.1);
  - toggling calls `arm_meeting` and `disarm_meeting`;
  - a Missed block shows "No call detected" and its Record button calls `record_meeting` (AC 5.6.5);
  - the switch has role `switch` with `aria-checked`, a target of at least 24×24 px, and is clean under `vitest-axe` (AC 5.6.9).

**Standards that bite here:**

- Clean Code: no flag arguments (`AutoStart` enum); categorical states over booleans.
- Complexity 5: split `next_actions` into per-transition predicates.
- Security: notify on every auto-start.
- Design Standards: switch semantics and 3:1 boundary contrast.

**Quality checks**

- **There is no code path from `ArmedWatch` to `RecordingController::start` unless `in_call` is true and the match resolved to that meeting. Every auto-start is followed by a "Recording <subject>" notification (AC 5.6.3, 5.6.6, invariant 4.7).**
- Manually started recordings never stop when the call ends (AC 5.6.4).
- `armed_meetings` holds only event ids and dates, never subjects (invariant 4.3, 4.9).
- `stop_before_sleep`, `went_quiet` and `CALL_END_GRACE` behave as before.
- Notifications live in `calendar/notify.rs`; `notifications.rs` isn't restructured.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite: all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** Arm a meeting and join its Teams call. Recording starts with no popover and a "Recording <subject>" notification. Leave the call, and after the grace the transcript saves with "Saved <subject>". Arm a second meeting and don't join: five minutes after the start, the missed notice appears and the block offers Record.
3. **Accessibility.** Same session: the switch is reachable by Tab, toggles with Space, announces on and off, and its focus is visible.

---

## Slice 7 — Switch to the next meeting without leaving the call

**Key decision:** A pure `SwitchPrompts` in the core decides when to offer "Switch to B?". That happens when B is armed and has started, or when re-read Teams titles now name B, and only once per B. The tray re-reads titles every 30 s while recording a linked transcript in a call ([decision 11](#decisions-made)), opens the popover with the prompt, and on Switch stops A and starts B linked ([decision 10](#decisions-made)). Doing nothing keeps A (invariant 4.8).

**Structure**

- `crates/scribe-core/src/calendar/switching.rs` (new):
  - `struct SwitchPrompt { from: Meeting, to: Meeting }`
  - `SwitchPrompts` holds the `HashSet<EventId>` already offered.
  - `struct SwitchInputs<'a> { linked: &'a Meeting, agenda: &'a TodaysAgenda, armed: &'a ArmedMeetings, titled: Option<&'a Meeting>, now: DateTime<FixedOffset> }`
  - `due(&mut self, inputs: &SwitchInputs) -> Option<SwitchPrompt>`
  - `const TITLE_RECHECK_INTERVAL: Duration = 30 s`.
- `apps/scribe-tray/src-tauri/src/calendar/switching.rs` (new): `SwitchState` (managed).
  - `Mutex<{ prompts: SwitchPrompts, open: Option<SwitchPrompt>, last_title_check: Option<Instant> }>`
  - `on_poll<R>(app, now)`, called from `calendar::on_poll` while recording a linked session in a call.
  - `switch<R>(app)` and `keep<R>(app)`.
- `apps/scribe-tray/src-tauri/src/calendar/commands.rs` (modify): `switch_prompt` (snapshot: `{ from_subject, to_subject } | null`), `switch_meeting`, `keep_recording`. Emits `meeting://switch-prompt`.
- Frontend (new): `src/popover/meeting/SwitchPromptCard.tsx` with "Still recording <A>. Switch to <B>?", a **Switch** button and a **Keep recording** button. Focus moves to the card's heading on open, as in `SettingsPanel` `[verified pattern]`. It renders above `LiveTranscript`.

**Collaborators:** The tray `SwitchState` reads `MeetingSelection` (the linked A), `AgendaCache`, `ArmedStore`, `CallPresence` and the titles adapter. `switch` calls `RecordingController::stop`, then `start_for_meeting(app, B, AutoStart::No)`, which carries B's armed status forward through `note_recording`. The popover opens through `popover::open_for_call`. While the popover is already visible, the event is what updates it ([context §4c](../specs/009-outlook-calendar-linking-context.md#4c-slice-traps)).

**Control flow:**

- `on_poll`:
  - Guard: only when in a call, recording, and the session is linked to A.
  - When `TITLE_RECHECK_INTERVAL` has elapsed, re-read the titles and compute `titled = meeting_named_in_titles(candidates at now, titles)`.
  - `due(...)`: B must differ from A, must not have been offered before, and must be either armed with `now >= B.start` or equal to `titled` (AC 5.7.1, 5.7.2).
  - On `Some`, record it as offered, store it as `open`, emit, and open the popover.
- `switch`: take `open`, stop A (its link to A is written by slice 3's `RecordingStopped` path), start B linked, clear `open`, emit `null`.
- `keep`: clear `open` and emit `null`. B stays offered, so the prompt never returns (AC 5.7.4).

**Tests:**

- New `crates/scribe-core/tests/switch_prompts.rs`:
  - an armed B that has started prompts while A is linked (AC 5.7.1);
  - a title naming B prompts (AC 5.7.2);
  - B is offered only once, even after Keep (AC 5.7.2, 5.7.4);
  - a B that is neither armed nor titled never prompts (AC 5.7.5);
  - A itself never prompts;
  - an armed B before its start doesn't prompt.
- New `src/popover/meeting/SwitchPromptCard.spec.tsx`:
  - the text names both meetings (AC 5.7.1);
  - Switch calls `switch_meeting` and Keep calls `keep_recording` (AC 5.7.3, 5.7.4);
  - focus moves to the prompt on open, and both buttons work by keyboard (AC 5.7.6);
  - `vitest-axe` is clean.
- Extend `Popover.spec.tsx`: every `backend()` answers `switch_prompt` with null.

**Standards that bite here:**

- Clean Code: command-query separation (`due` mutates only the offered set, which its name and signature state).
- Design Standards: focus management and keyboard operation.
- Engineering standards: business-language test names.

**Quality checks**

- **There is no split without a click on Switch. A timeout, closing the popover, or Keep leaves A recording and linked to A (AC 5.7.4, invariant 4.8).**
- A's transcript keeps A's link after a switch; B's transcript is linked to B (AC 5.7.3, invariant 4.5).
- Titles are re-read at most once per `TITLE_RECHECK_INTERVAL` and only while in a call (AC 5.4.5).
- `CALL_END_GRACE` is unchanged; back-to-back meetings are handled by the prompt, not by a shorter grace ([spec §10.1](../specs/009-outlook-calendar-linking.md#10-established-behaviour--documented-not-changed)).
- The prompt lives in the popover on both OSes; no notification buttons are added ([spec §10.2](../specs/009-outlook-calendar-linking.md#10-established-behaviour--documented-not-changed)).

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite: all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** Record a call linked to A and arm a meeting B that starts while the call continues. The popover offers the switch. Press Switch: A's transcript is saved linked to A, and a new transcript linked to B is recording. Repeat and press Keep recording: the prompt doesn't come back.
3. **Accessibility.** Same session: focus lands on the prompt when it opens, both buttons are reachable by Tab, and focus is visible.

**Open at build time (AC 5.7.3):** the real gap between A's capture stopping and B's capture starting, which depends on how long the transcription drain takes on the machine. Probe: in the running app, switch during speech and compare the timestamp of A's last segment with B's first. If it is 2 s or less, ship as planned. If it is more, implement the fallback from [decision 10](#decisions-made):
- add `Recorder::stop_detached(&mut self) -> JoinHandle<Result<(), RecorderError>>`, which stops the sources, sets the state to idle, and moves the worker and writer joins onto a background thread;
- `switch` calls it before `start_for_meeting`;
- log the outcome when the join completes.

Record the measured gap in the Escalation log.

---

## Hard floors

- Slice 1: categories **auth** (OAuth sign-in, token issuance and refresh) and **security** (secrets in the OS credential store, PKCE and `state`). Floor tier `≥ excellent`; planned at `best`.

No other slice touches a designated category. Slice 3 changes the transcript front-matter format, but that is an optional additive field with no schema or data migration (ARCHITECTURE §5 "Migrations: none").

## Escalation log

- 2026-10-26 — slice 1 — scope `Calendars.ReadBasic` → `Calendars.Read`. Reason: Microsoft documents only what `ReadBasic` withholds ("properties such as body, attachments, and extensions"), not that it returns `attendees` and `organizer`, which slice 3 needs. `Calendars.Read` is still read-only and needs no admin consent for delegated use, so invariant 4.2 holds. Spec OQ 8.2 closed; changed `authority.rs` `SCOPES`, its tests, ADR 10 and spec 4.2, 5.1.4, 6.1.4. Decided by Sapan.
- 2026-10-26 — all slices — real-tenant probes (OQ 8.1, refresh-token size on Windows, `id_token` claim names, OQ 8.4 Teams titles) and the slice 7 gap measurement move to one manual test pass after slice 7. Builders use fakes and the placeholder `CLIENT_ID`. Decided by Sapan.
