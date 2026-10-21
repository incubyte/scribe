# Plan: 006-teams-call-auto-open

> Spec: none — planned from prompt (standard tier, spike) · Context: [`docs/specs/006-teams-call-auto-open-context.md`](../specs/006-teams-call-auto-open-context.md) · Authored: 2026-10-21
> Risk: LOW (spike; reads process names and mic-in-use flags only, no audio content; easy to revert) · Hard floors: none · Fabric standards: reachable

---

# Part 1 — Review this

*Everything a reviewer needs to approve or redirect the approach. If you only read one part, read this one.*

## What we're doing

When a Microsoft Teams call starts, the Scribe popover opens by itself so you can start recording, unless Scribe is already recording. It opens once per call and stays quiet for the rest of the call. This covers the desktop Teams app on macOS and Windows.

A call is detected by **Teams taking the microphone**, not by Teams playing sound. Teams plays ringtones and notification sounds outside calls, but it only opens the mic for a call or its pre-join screen. Both OSes report which processes are using the mic without opening any audio stream, and the libraries Scribe already uses (`cidre`, `wasapi`) expose this. So no new permission and no new crate is needed.

"No flicker" comes from a small latch: it reports a call once, when Teams goes from not using the mic to using it. It then stays latched until Teams has released the mic for 30 seconds. A brief mic drop (device switch, Teams reconnecting) does not end the call or trigger a second open.

The popover appears **without taking focus**, so you can keep typing in Teams. It ignores focus changes until you first click into it, then behaves as it does today. One deliberate exception: on Windows, Tauri's `show()` always activates the window, so the popover will briefly take focus from Teams there. It still stays open when you click back into Teams.

## How it breaks down

Slice 1 builds the latch in core, where portable CI and mutation testing cover it. Slices 2 and 3 are the two OS probes that answer "is Teams holding the mic?". Slice 4 teaches the popover to open without focus. Slice 5 connects everything in the tray with a poll every 2 seconds. Mutation testing runs once over the whole diff after slice 5, not per slice.

| # | Slice | Delivers | Why this tier | Tier |
|---|-------|----------|---------------|------|
| 1 | Latch calls in core | `CallWatch` latch and the `CallProbe` port, fully tested | Small, but it is the whole "once per call, no flicker" rule | `excellent` |
| 2 | Probe Teams on macOS | `CoreAudioCallProbe` reading Core Audio process objects | Platform calls whose signatures are verified; must skip unreadable processes | `excellent` |
| 3 | Probe Teams on Windows | `WasapiCallProbe` reading capture sessions and walking parent processes | COM and Win32 calls that CI never runs, so they must be right first time | `excellent` |
| 4 | Open the popover without focus | `popover::open_for_call` and `BlurGuard` waiting for the first real focus | Tested focus rule, plus the tray-position fallback | `excellent` |
| 5 | Watch for calls in the tray | Per-OS probe selection and a background poll thread that opens the popover | Thin glue that follows the plan exactly | `fast` |

Slices 2 and 3 have the same shape: list who holds the mic, then match Teams. Slice 3 adds a parent-process walk because Teams on Windows may run its audio in a WebView2 child process.

## Decisions made

**1. A call starts when Teams takes the microphone, not when it plays sound.** Ringtones and notifications use the speakers outside calls; only calls and the pre-join screen open the mic. This lands in slices 2 and 3. *Rejected: watching audio output, which would open the popover for every chat ping.*

**2. The "once per call" rule lives in a pure latch in core, and time is passed in.** `CallWatch` is fed "does a call app hold the mic?" plus the current `Instant`. It reports once on the rising edge and stays latched until the mic has been released for `CALL_END_GRACE` (30 s). It mirrors `QuietWatch` (`observe`, `is_start_due`, `mark_reported`). This lands in slice 1. *Rejected: putting the rule in the tray thread, which is Tauri-bound and untested by convention, and is where flicker bugs would hide.*

**3. Core knows "call app", never "Teams".** The port is `CallProbe::call_app_holds_microphone`. The Teams bundle id and exe name live only in the adapters, as ARCHITECTURE §7 requires (no OS or vendor names in core). Adding Zoom later only touches the adapters. This lands in slices 1 to 3.

**4. Teams is matched precisely.**
- macOS: bundle id prefix `com.microsoft.teams`. This covers new Teams (`com.microsoft.teams2`) and any helper whose id extends it, but not Edge or Outlook (`com.microsoft.edgemac`, `com.microsoft.Outlook`).
- Windows: the process holding an active capture session is `ms-teams.exe`, or has `ms-teams.exe` as a parent or grandparent. A `msedgewebview2.exe` only counts when Teams owns it.

This lands in slices 2 and 3. *Rejected: a bare `com.microsoft.` prefix, or any `msedgewebview2.exe`; both would catch Outlook, Edge and Widgets.*

**5. Poll every 2 seconds instead of subscribing to OS events.** Polling is identical on both OSes and costs one cheap process listing every 2 s. Event listeners on macOS (per-process property listeners) and Windows (per-session notifications) are two different, fiddly mechanisms. This lands in slice 5. *Rejected: Core Audio and WASAPI listeners, a possible follow-up if battery use shows up.*

**6. The popover opens without taking focus and ignores focus loss until you first click into it.** The new `BlurGuard::shown_awaiting_focus` keeps blur-hide off until the popover gains focus at least 300 ms (`SETTLE`) after it opened. That way the activation Windows forces at show time does not count, and your first real click does. After that click, clicking away hides it as today. This lands in slice 4. *Rejected: reusing `popover::toggle`, which hides a visible popover and takes focus; and pinning the popover until a tray click, which ignores your choice that clicking away after interacting should close it.*

**7. The auto-open falls back to a screen corner when the tray position is unknown.** `tauri-plugin-positioner` only learns the tray position from a tray click or hover. Its `TrayIconEvent` is `#[non_exhaustive]`, so we cannot seed it ourselves. So `move_window_constrained(Position::TrayCenter)` fails until you have touched the tray icon since launch. Falling back to `TopRight` on macOS and `BottomRight` on Windows means a call right after launch still opens the popover. This lands in slice 4.

**8. Open unless Scribe is `Recording`.** `Idle` and `Locked` (model still downloading, privacy not yet acknowledged) both open, because the popover is where you resolve those states. The recorder state is read only on the rising edge, because its mutex is held for seconds while a recording stops. This lands in slice 5.

## Where this could go wrong

- **Teams may hold the mic from a process we do not match.** For example, on macOS the mic may be attributed to a WebKit or WebView helper with a non-Teams bundle id. This is the spike's main unknown, and it can only be answered during a real call. Slice 2 ships an ignored smoke test that lists every bundle id holding the mic, so a failed match shows what to match instead (slice 2, "Open at build time").
- **macOS focus behaviour is unverified on hardware.** `show()` calls `makeKeyAndOrderFront` without activating Scribe. We expect Teams to keep focus and no blur event to arrive, but the guard rule in Decision 6 holds either way (slice 4 quality checks).
- **Probe errors could end a call by mistake.** If a failed probe counted as "mic released", one Core Audio hiccup would re-arm the latch and reopen the popover mid-call. Slice 5 skips the tick on error (slice 5 quality checks).
- **Windows code never runs in CI.** The Windows job is clippy only (ADR 2). Slice 3 keeps all decisions in tiny functions and ships an ignored smoke test to run by hand on a Windows machine.
- **Pre-join opens the popover.** Teams opens the mic on its pre-join screen, so the popover appears just before you join. This is accepted for the spike.

## What's out of scope

- Teams in a browser, Google Meet, Zoom and other meeting apps. The adapters are where they would be added later (Decision 3).
- Classic Teams (`com.microsoft.teams` without the `2`) is matched by the same prefix but is not tested; Microsoft retired it.
- A setting to turn the feature off. It is always on for the spike; add a toggle if it proves annoying.
- Starting a recording automatically. The popover only opens; you still press Record.
- Event-driven detection (Decision 5).
- Avoiding the brief focus steal on Windows (Decision 6). Fixing it needs `SW_SHOWNOACTIVATE`, which Tauri only uses for windows created unfocused.

---

# Part 2 — Build detail

*Prescriptive detail for the build agent. Reviewers can skim or skip.*

**Confidence markers used below:** `[verified]` = the planner read the code or docs and confirmed it. `[inferred]` = pattern-matched from a sibling implementation, not confirmed — the build agent should check before assuming.

## Shared conventions (all slices)

- **Architecture:** Hexagonal (ADR 1). The domain rule (`CallWatch`) and the port (`CallProbe`) live in `scribe-core`, adapters in `crates/audio-macos` and `crates/audio-windows`, and the composition root plus polling in `apps/scribe-tray`. No `#[cfg(target_os)]`, Teams names or Tauri types in core ([context §4c](../specs/006-teams-call-auto-open-context.md#4c-slice-traps), slice A trap 3).
- **Dependency direction:** tray → adapters → core; tray → core. Core depends on nothing new.
- **Module map:**
  - `crates/scribe-core/src/call/mod.rs` (new) · re-exports `CallWatch`, `CALL_END_GRACE` · std only
  - `crates/scribe-core/src/call/call_watch.rs` (new) · the latch · std only
  - `crates/scribe-core/src/ports.rs` (modify) · `CallProbe` trait · `CaptureError` `[verified]`
  - `crates/audio-macos/src/call_probe.rs` (new) · `CoreAudioCallProbe` · `cidre` `core_audio` (already enabled) `[verified]`
  - `crates/audio-windows/src/call_probe.rs` (new) · `WasapiCallProbe` · `wasapi` 0.26 `[verified]`, `windows` 0.62 (new dependency of this crate) `[verified]`
  - `apps/scribe-tray/src-tauri/src/blur_guard.rs` (modify) · focus-awaiting rule · std only
  - `apps/scribe-tray/src-tauri/src/popover.rs` (modify) · `open_for_call`, Focused(true) hook · `tauri`, `tauri_plugin_positioner`
  - `apps/scribe-tray/src-tauri/src/call_probes.rs` (new) · per-OS probe choice · adapters
  - `apps/scribe-tray/src-tauri/src/call_watch.rs` (new) · poll thread · core, `popover`, `recording`
- **Lexicon (from [context §4b](../specs/006-teams-call-auto-open-context.md#4b-slice-lexicon) — do not rename):**
  - Existing names: `RecorderState::{Locked, Idle, Recording}`, `RecordingController::state`, `BlurGuard::{shown, blur_should_hide, click_may_show}`, `SETTLE`, `start_scribe`, `install`, `CaptureError::StreamFailed`, `failure(action, status)`, `stream_failed(&error)`.
  - New names, used verbatim across slices: `CallWatch`, `CALL_END_GRACE`, `CallProbe`, `call_app_holds_microphone`, `CoreAudioCallProbe`, `WasapiCallProbe`, `TEAMS_BUNDLE_ID_PREFIX`, `TEAMS_EXE`, `BlurGuard::{shown_awaiting_focus, focused}`, `popover::open_for_call`, `call_probes::call_probe`, `call_watch::install`, `POLL_INTERVAL`.
  - British spelling.
- **Tests:**
  - Core: a new file, `crates/scribe-core/tests/call_watch.rs`, modelled on `tests/quiet_watch.rs`.
  - Tray: extend `apps/scribe-tray/src-tauri/tests/blur_guard.rs`.
  - Adapters: `#[ignore]` hardware smoke tests in the `tap_smoke.rs` shape.
  - Tauri-bound code (`call_watch.rs`, `call_probes.rs`, `popover.rs`) gets no unit tests, by convention ([context §6](../specs/006-teams-call-auto-open-context.md#6-test-conventions)).
  - Test names are business-language behaviour sentences.
- **Org standards that bite in every slice:**
  - Functions under 20 lines, cyclomatic complexity at most 5, no blank lines inside function bodies.
  - Guard clauses, command-query separation, no dead code, no comments.
  - Conventional Commits, one commit per slice.
  - ADR 7 (work lands directly on `main`) overrides the org branch-naming standard for this repo.
- **Commands:** `apps/scribe-tray/scripts/stage-sidecar.sh` once, then `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- **Mutation testing runs once, at end-of-spec only.** At the developer's request, no slice-builder runs `cargo mutants` or `npm run mutate`.
  - After the last slice, the orchestrator runs `cargo mutants --in-diff <diff of all slices>` a single time.
  - Surviving mutants in `call_watch.rs` (core) or `blur_guard.rs` are fixed with targeted tests in one follow-up pass.
  - There are no frontend changes, so Stryker does not run.

---

## Slice 1 — Latch calls in core

**Key decision:** Adds the pure rule from Decision 2 and the port from Decision 3. `CallWatch` is fed one observation per poll and answers "is a call start due?". The tray marks it reported. No clock, no OS, no Teams.

**Structure**

- `crates/scribe-core/src/call/call_watch.rs` (new):
  - `pub const CALL_END_GRACE: Duration` = 30 s.
  - `pub struct CallWatch { grace: Duration, in_call: bool, released_at: Option<Instant>, reported: bool }`.
  - `#[must_use] pub fn new(grace: Duration) -> CallWatch`
  - `pub fn observe(&mut self, call_app_holds_microphone: bool, now: Instant)`
  - `pub fn mark_reported(&mut self)`
  - `#[must_use] pub fn is_start_due(&self) -> bool`
  - Private helpers for the two observe branches, such as `hold(&mut self)` and `release(&mut self, now: Instant)`.
- `crates/scribe-core/src/call/mod.rs` (new) — `mod call_watch; pub use call_watch::{CallWatch, CALL_END_GRACE};`
- `crates/scribe-core/src/lib.rs` (modify) — `pub mod call;`
- `crates/scribe-core/src/ports.rs` (modify) — `pub trait CallProbe: Send { fn call_app_holds_microphone(&self) -> Result<bool, CaptureError>; }`, placed after `ScreenCapturer`.

**Collaborators:** none. The port has no fake in `testing.rs` because no core code calls it (YAGNI).

**Control flow:**
- `observe(true, now)`: clear `released_at`. If not already `in_call`, set `in_call` and clear `reported`.
- `observe(false, now)`:
  - Guard: not `in_call` → nothing.
  - Otherwise keep the first release moment (`released_at` set only if `None`).
  - When `now - released_at` (saturating) reaches `grace`, end the call: `in_call = false`, `released_at = None`.
- `is_start_due`: `in_call && !reported`.

**Tests:** new `crates/scribe-core/tests/call_watch.rs` (unit, Small). Build times as `started + Duration`; never sleep. Cases (ZOMBIES):
- nothing is due before any call app takes the microphone (AC 1.1)
- a call app taking the microphone makes a call start due (AC 1.2)
- a reported call is not due again while the microphone stays in use (AC 1.3)
- a microphone release one millisecond short of the grace keeps the same call, with nothing due on resume (AC 1.4)
- a release lasting exactly the grace ends the call (AC 1.5)
- taking the microphone again after a call ended makes a new call start due (AC 1.6)
- an app already holding the microphone at the first observation makes a call start due (AC 1.7)

**Standards that bite here:** `Clean Code: command-query separation` (`observe`/`mark_reported` are commands, `is_start_due` is a query), `Clean Code: guard clauses`, `Clean Code: dependency direction`, Incubyte: test names in business language, ZOMBIES boundaries.

**Quality checks**

- **The latch reports exactly once per call: a `true` observation while latched never makes `is_start_due` true again until the call has ended** (AC 1.3, 1.4)
- No `Instant::now()`, `SystemTime` or sleep anywhere in `call_watch.rs`; time arrives only through `observe`.
- No "teams", "microsoft", `cfg(target_os)` or OS crate in `scribe-core` (ARCHITECTURE §7).
- The grace boundary is pinned by an exact-limit test and a one-ms-short test (AC 1.4, 1.5).
- `CallProbe` takes no collaborators and returns `Result<bool, CaptureError>`; no new error enum.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.

---

## Slice 2 — Probe Teams on macOS

**Key decision:** Implements `CallProbe` over Core Audio's process list (Decisions 1, 3, 4). Reading process objects opens no stream and needs no permission `[inferred]` ([context §8](../specs/006-teams-call-auto-open-context.md#8-open-uncertainties-resolved-during-orientation)). A public diagnostic, `bundle_ids_using_microphone`, lets the smoke test show what actually holds the mic.

**Structure**

- `crates/audio-macos/src/call_probe.rs` (new):
  - `const TEAMS_BUNDLE_ID_PREFIX: &str = "com.microsoft.teams";`
  - `#[derive(Default)] pub struct CoreAudioCallProbe;`
  - `impl CoreAudioCallProbe { pub fn bundle_ids_using_microphone(&self) -> Result<Vec<String>, CaptureError> }`
  - `impl CallProbe for CoreAudioCallProbe { fn call_app_holds_microphone(&self) -> Result<bool, CaptureError> }`
  - Private helpers `uses_microphone(process: &ca::Process) -> bool` and `bundle_id_of(process: &ca::Process) -> Option<String>`.
  - Error mapping mirrors `tap.rs` `failure(action, status)` → `CaptureError::StreamFailed` `[verified]`. Copy the helper locally; it is private to `tap.rs`.
- `crates/audio-macos/src/lib.rs` (modify) — `#[cfg(target_os = "macos")] mod call_probe;` and `#[cfg(target_os = "macos")] pub use call_probe::CoreAudioCallProbe;`
- APIs `[verified]` in `cidre-0.29.0/src/core_audio/hardware.rs`:
  - `ca::System::processes() -> os::Result<Vec<ca::Process>>`
  - `Process::is_running_input(&self) -> os::Result<bool>`
  - `Process::bundle_id(&self) -> os::Result<arc::R<cf::String>>` (`.to_string()` gives a Rust `String`)
- `crates/audio-macos/tests/call_probe_smoke.rs` (new) — `#![cfg(target_os = "macos")]`.

**Collaborators:** `cidre` only. Depends on `scribe-core` for `CallProbe` and `CaptureError`.

**Control flow:**
- `bundle_ids_using_microphone`:
  1. `System::processes()`; on error, return `failure("list audio processes", status)`.
  2. Keep processes where `uses_microphone` holds (`is_running_input` treats an error as false).
  3. Map through `bundle_id_of` (an error or empty string gives `None`, skipped) and collect.
- `call_app_holds_microphone`: `bundle_ids_using_microphone()?` then `any(starts_with(TEAMS_BUNDLE_ID_PREFIX))`.
- Check `is_running_input` before `bundle_id`, so bundle ids are read only for mic users ([context §7](../specs/006-teams-call-auto-open-context.md#7-hot-paths-and-performance-landmines)).

**Tests:** new `crates/audio-macos/tests/call_probe_smoke.rs`, one `#[ignore = "needs a live Microsoft Teams call"]` test: "a teams call in progress is detected". It asserts `call_app_holds_microphone()` is `Ok(true)`, and its failure message includes `bundle_ids_using_microphone()`. Hardware-bound, so it is not part of the automated suite, matching `tap_smoke.rs` ([context §6](../specs/006-teams-call-auto-open-context.md#6-test-conventions)).

**Standards that bite here:** `Clean Code: fail fast at boundaries` (one error mapping at the adapter edge), `Clean Code: small functions`, `Clean Code: intent-revealing names`.

**Quality checks**

- **One unreadable process (bundle id or input flag errors) is skipped and never fails the whole probe**; only a failed process listing returns `Err` ([context §4c](../specs/006-teams-call-auto-open-context.md#4c-slice-traps), slice B trap 2).
- The match uses `is_running_input`, never `is_running` or `is_running_output` (Decision 1).
- The prefix is exactly `com.microsoft.teams`; no bare `com.microsoft.` match (Decision 4).
- No `unsafe` and no `#![allow(unsafe_code)]`; the `cidre` calls used are safe `[verified]`.
- Every `mod`/`pub use` added to `lib.rs` is gated `#[cfg(target_os = "macos")]`, so the crate still compiles empty on Linux CI.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.

**Open at build time:** Which bundle id Core Audio reports for Teams' mic use (main app or a helper) can only be seen during a live call. The builder writes the smoke test. The developer runs `cargo test -p audio-macos --test call_probe_smoke -- --ignored` during a Teams call. If the listed bundle id does not start with `com.microsoft.teams`, record it in the Escalation log and widen `TEAMS_BUNDLE_ID_PREFIX` to an explicit list of Teams ids.

---

## Slice 3 — Probe Teams on Windows

**Key decision:** Implements `CallProbe` over WASAPI capture sessions (Decisions 1, 3, 4). It lists the pids of every `Active` session on every active capture endpoint. Only when that list is non-empty does it take one ToolHelp process snapshot and check whether any pid is `ms-teams.exe` or descends from it. Each probe call initialises and releases COM itself, so the caller's thread carries no COM contract.

**Structure**

- `crates/audio-windows/src/call_probe.rs` (new), starting with `#![allow(unsafe_code)]` (Win32 FFI):
  - `const TEAMS_EXE: &str = "ms-teams.exe";`
  - `const MAX_ANCESTRY: usize = 4;`
  - `#[derive(Default)] pub struct WasapiCallProbe;`
  - `impl CallProbe for WasapiCallProbe { fn call_app_holds_microphone(&self) -> Result<bool, CaptureError> }`
  - Private helpers:
    - `fn active_capture_pids() -> Result<Vec<u32>, CaptureError>`
    - `fn active_session_pids(device: &Device) -> Result<Vec<u32>, WasapiError>`
    - `struct ProcessTable(HashMap<u32, (u32, String)>)` (pid → parent pid, lower-cased exe name)
    - `fn ProcessTable::snapshot() -> Result<ProcessTable, CaptureError>`
    - `fn ProcessTable::belongs_to_teams(&self, pid: u32) -> bool`
  - Errors map through a local `stream_failed(&error)` mirroring `stream.rs` `[verified]`.
- APIs `[verified]` in `wasapi-0.26.0/src/api.rs`, via [context §3](../specs/006-teams-call-auto-open-context.md#3-reusable-utilities):
  - `wasapi::initialize_mta()` and `wasapi::deinitialize()`
  - `DeviceEnumerator::new()?.get_device_collection(&Direction::Capture)?`, iterated by index: `get_nbr_devices()`, `get_device_at_index(i)`. Never `for d in &collection` ([context §4c](../specs/006-teams-call-auto-open-context.md#4c-slice-traps), slice C trap 3).
  - `Device::get_iaudiosessionmanager()?.get_audiosessionenumerator()?`
  - `get_count() -> i32` and `get_session(i)`, then `AudioSessionControl::get_state()? == SessionState::Active` and `get_process_id()?`. Skip pid `0`.
- APIs `[verified]` via context §3, in `windows` 0.62:
  - `Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0), Process32FirstW, Process32NextW, PROCESSENTRY32W}`; fields `th32ProcessID`, `th32ParentProcessID`, `szExeFile`.
  - `Win32::Foundation::CloseHandle`.
  - Set `PROCESSENTRY32W.dwSize` before the first call `[inferred]` (the standard ToolHelp contract).
- `crates/audio-windows/Cargo.toml` (modify) — under `[target.'cfg(windows)'.dependencies]` add `windows = { version = "0.62", features = ["Win32_Foundation", "Win32_System_Diagnostics_ToolHelp"] }`.
- `crates/audio-windows/src/lib.rs` (modify) — `#[cfg(windows)] mod call_probe;` and `#[cfg(windows)] pub use call_probe::WasapiCallProbe;`
- `crates/audio-windows/tests/call_probe_smoke.rs` (new) — `#![cfg(windows)]`.

**Collaborators:** `wasapi`, `windows`. Depends on `scribe-core` for `CallProbe` and `CaptureError`.

**Control flow:**
- `call_app_holds_microphone`:
  1. `initialize_mta()`.
  2. `active_capture_pids()`.
  3. `deinitialize()`, always, including on the error path.
  4. Guard: an empty pid list returns `Ok(false)` without a snapshot.
  5. `ProcessTable::snapshot()?`, then `any(belongs_to_teams)`.
- `belongs_to_teams`: walk from `pid` through parents for at most `MAX_ANCESTRY` steps. True when any visited exe equals `TEAMS_EXE`. The step limit guards against pid-reuse cycles.
- `active_capture_pids`: across every active capture device, flatten `active_session_pids`.

**Tests:** new `crates/audio-windows/tests/call_probe_smoke.rs`, one `#[ignore = "needs a live Microsoft Teams call on Windows"]` test: "a teams call in progress is detected". It asserts `Ok(true)`. CI does not run Windows tests (ADR 2), so this is run by hand on a Windows machine.

**Standards that bite here:** `Clean Code: small functions` and complexity at most 5 (split the snapshot loop from the ancestry walk), `Clean Code: fail fast at boundaries`, `Clean Code: no side effects` (`deinitialize` paired with every `initialize_mta`).

**Quality checks**

- **`msedgewebview2.exe` and any other exe only count when `ms-teams.exe` is the process itself or one of its first `MAX_ANCESTRY` ancestors** (Decision 4).
- Exe names compare case-insensitively (lower-cased once when the snapshot is built).
- Only `SessionState::Active` sessions count; `Inactive` and `Expired` are ignored ([context §4c](../specs/006-teams-call-auto-open-context.md#4c-slice-traps), slice C trap 5).
- All active capture endpoints are enumerated, not just the default device (slice C trap 2).
- The snapshot handle is closed on every path, and `deinitialize` runs on every path.
- `cargo clippy --workspace --all-targets -- -D warnings` passes for the Windows target (the CI `windows` job).

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.

**Open at build time:** Whether new Teams holds the capture session in `ms-teams.exe` itself or in a WebView2 child can only be seen on a Windows machine during a call. The ancestry walk covers both. If the smoke test fails, record the session's exe chain in the Escalation log.

---

## Slice 4 — Open the popover without focus

**Key decision:** Adds `popover::open_for_call`, which shows the popover only if it is hidden, never toggles, and never calls `set_focus` (Decision 6). It adds a focus-awaiting state to `BlurGuard` so blur is ignored until the popover first gains focus at least `SETTLE` after opening. It also falls back to a corner when the tray position is unknown (Decision 7).

**Structure**

- `apps/scribe-tray/src-tauri/src/blur_guard.rs` (modify):
  - New field `awaiting_focus: bool`.
  - `pub fn shown_awaiting_focus(&mut self, now: Instant)` — sets `shown_at` and `awaiting_focus`.
  - `pub fn focused(&mut self, now: Instant)` — clears `awaiting_focus` only when settled since `shown_at`.
  - `shown(now)` also clears `awaiting_focus`.
  - `blur_should_hide` returns `false` while `awaiting_focus`.
- `apps/scribe-tray/src-tauri/src/popover.rs` (modify):
  - `pub fn open_for_call<R: Runtime>(app: &AppHandle<R>)`
  - Private `fn place_at_tray<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()>`: `move_window_constrained(Position::TrayCenter)`, falling back to `move_window(FALLBACK_POSITION)` on error.
  - `const FALLBACK_POSITION: Position` is `Position::TopRight` under `#[cfg(target_os = "macos")]` and `Position::BottomRight` under `#[cfg(not(target_os = "macos"))]`. Both variants exist in `tauri_plugin_positioner::Position` `[inferred]`; check the enum.
  - Existing `show` reuses `place_at_tray` instead of the bare `move_window_constrained` call, so behaviour on the click path is unchanged when the tray position is known.
  - `install_blur_hide` handles `WindowEvent::Focused(true)` by calling `with_guard(app, |guard| guard.focused(Instant::now()))`, alongside the existing `Focused(false)` branch.
- Facts `[verified]`:
  - `WebviewWindow::show()` on macOS is `makeKeyAndOrderFront` without app activation (`tao` `set_visible`). On Windows it is `ShowWindow(SW_SHOW)`, which activates, unless the window was created unfocused (`tao` `window_state.rs`).
  - `TrayIconEvent` is `#[non_exhaustive]`, so the positioner cannot be seeded.

**Collaborators:** `tauri`, `tauri_plugin_positioner`, `BlurGuard` through the existing `with_guard`/`popover_window`/`log_failure` reusables ([context §3](../specs/006-teams-call-auto-open-context.md#3-reusable-utilities)).

**Control flow:**
- `open_for_call`:
  1. Get the window via `popover_window`; log and return on error.
  2. Guard: already visible → return.
  3. `place_at_tray` (log failure, continue).
  4. `window.show()` (log failure, return on error).
  5. `with_guard(shown_awaiting_focus(Instant::now()))`.
- `focused(now)`: guard `!awaiting_focus` → nothing. If settled since `shown_at`, clear `awaiting_focus`.

**Tests:** extend `apps/scribe-tray/src-tauri/tests/blur_guard.rs` (unit). Cases:
- a popover opened for a call stays open when focus goes elsewhere (AC 4.1)
- focus arriving straight after a call opened the popover does not make it hide on blur (AC 4.2; the Windows forced activation)
- once you click into a popover opened for a call, clicking away hides it (AC 4.3)
- opening the popover from the tray after a call-opened one hides on blur as before (AC 4.4)

The existing tests stay unchanged and green (AC 4.5).

**Standards that bite here:** `Clean Code: command-query separation` (`focused` is a command; `blur_should_hide` keeps its existing shape), `Clean Code: small functions`, no blank lines in function bodies.

**Quality checks**

- **`open_for_call` never hides a visible popover and never calls `set_focus`** (Decision 6).
- `popover::toggle` and the tray-click path behave exactly as before; existing `blur_guard.rs` tests pass unmodified (AC 4.5).
- `open_for_call` calls `shown_awaiting_focus`, never `shown`.
- When `TrayCenter` placement fails, the popover is still shown at `FALLBACK_POSITION`, and the failure is logged at `warn` (Decision 7).
- No change to the frontend, so no new IPC command and no `mockIPC` update.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
3. **User-facing flow.** Checked by hand together with slice 5, in the running tray app on macOS: the popover appears near the tray icon (or the top-right corner if the tray was never touched), Teams keeps keyboard focus, clicking into Teams leaves the popover open, and clicking into the popover then elsewhere hides it.
4. **Accessibility.** Same session: the popover opens with no control auto-focused, and once clicked, Tab moves through its controls with visible focus as today.

---

## Slice 5 — Watch for calls in the tray

**Key decision:** Connects slices 1 to 4. `call_probes::call_probe()` picks the OS adapter, mirroring `screenshots/capturers.rs` `[verified]`. `call_watch::install(app)` spawns one app-lifetime thread that polls every `POLL_INTERVAL` (2 s, Decision 5). The thread feeds `CallWatch`, and on a due start opens the popover unless the recorder is `Recording` (Decision 8).

**Structure**

- `apps/scribe-tray/src-tauri/src/call_probes.rs` (new) — `pub fn call_probe() -> Box<dyn CallProbe>` with three cfg modules:
  - macOS: `audio_macos::CoreAudioCallProbe`.
  - Windows: `audio_windows::WasapiCallProbe`.
  - `#[cfg(not(any(target_os = "macos", windows)))]`: `NoCallProbe`, returning `Ok(false)`.
- `apps/scribe-tray/src-tauri/src/call_watch.rs` (new):
  - `const POLL_INTERVAL: Duration` = 2 s.
  - `pub fn install<R: Runtime>(app: &AppHandle<R>)` — spawns a named thread (`"call-watch"`) owning `call_probe()`, `CallWatch::new(CALL_END_GRACE)` and a cloned `AppHandle`.
  - Private `fn watch_forever<R: Runtime>(app: AppHandle<R>, probe: Box<dyn CallProbe>, watch: CallWatch)`.
  - Private `fn check_once<R: Runtime>(app: &AppHandle<R>, probe: &dyn CallProbe, watch: &mut CallWatch, now: Instant)`.
  - Private `fn open_unless_recording<R: Runtime>(app: &AppHandle<R>)`.
- `apps/scribe-tray/src-tauri/src/lib.rs` (modify) — `mod call_probes;`, `mod call_watch;`, and `call_watch::install(app);` in `start_scribe` **after** `popover::install_blur_hide(app)?` ([context §4c](../specs/006-teams-call-auto-open-context.md#4c-slice-traps), slice D trap 6).
- `.cargo/mutants.toml` (modify) — add `call_watch.rs`, `call_probes.rs`, `crates/audio-macos/src/call_probe.rs` and `crates/audio-windows/src/call_probe.rs` to `exclude_globs`, following the screenshots precedent ([context §5](../specs/006-teams-call-auto-open-context.md#5-known-dragons)).
- The tray's `Cargo.toml` already depends on `audio-macos` and `audio-windows` `[inferred]`; confirm. If not, add them under the same `cfg` target sections used for the capture crates.

**Collaborators:** `scribe_core::{call::{CallWatch, CALL_END_GRACE}, ports::CallProbe}`, `call_probes`, `popover::open_for_call`, `RecordingController::state` via `app.state::<RecordingController>()` `[verified]`. Dependencies point inward only. Nothing in core or the adapters learns about the tray.

**Control flow:**
- `watch_forever`: loop forever: `thread::sleep(POLL_INTERVAL)`, then `check_once(.., Instant::now())`. The thread is never joined, consistent with the other app-lifetime threads ([context §5](../specs/006-teams-call-auto-open-context.md#5-known-dragons)).
- `check_once`:
  1. The probe returns `Err` → `tracing::debug!` and return. The error does not count as "released".
  2. Otherwise `watch.observe(holds, now)`.
  3. Guard: `!watch.is_start_due()` → return.
  4. `watch.mark_reported()`, then `open_unless_recording(app)`.
- `open_unless_recording`:
  1. Read `RecordingController::state()` once.
  2. Guard: `Recording` → `tracing::info!` that a call started while recording, then return.
  3. Otherwise `tracing::info!("a call started; opening the popover")` and `popover::open_for_call(app)`.

**Tests:** none new. `call_watch.rs` and `call_probes.rs` are Tauri-bound glue, untested by convention and excluded from mutation ([context §6](../specs/006-teams-call-auto-open-context.md#6-test-conventions)). The rules they rely on are covered by slices 1 and 4.

**Standards that bite here:** `Clean Code: separation of concerns` (the probe answers, the watch decides, the tray only shows), `Clean Code: small functions`, `Clean Code: no side effects` (`check_once` reads the recorder only on a due start).

**Quality checks**

- **A probe error leaves `CallWatch` untouched (no `observe(false, ..)`), so a transient failure can never end a call and reopen the popover** ([context §4c](../specs/006-teams-call-auto-open-context.md#4c-slice-traps), slice A trap 4).
- `RecordingController::state()` is called only after `is_start_due()` is true, never on every poll ([context §7](../specs/006-teams-call-auto-open-context.md#7-hot-paths-and-performance-landmines)).
- No new `RecordingEvent` variant and no use of `TrayRecordingListener` (slice D trap 5).
- `call_watch::install` runs after `popover::install_blur_hide` in `start_scribe`.
- Logs carry no bundle ids, exe names or window titles beyond the fixed messages.
- `POLL_INTERVAL` is at least 1 s.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
3. **User-facing flow.** By hand on macOS with `npm run tauri dev` (or the built app), with Scribe idle:
   - Start a Teams call: the popover opens once within about 2 s.
   - Stay in the call for over a minute, mute and unmute, and switch the mic: it does not reopen.
   - Close it mid-call (tray click): it stays closed.
   - Leave the call, wait over 30 s, and start a new call: it opens again.
   - Start a Scribe recording, then join a call: it does not open.
   - Repeat the first and last checks on Windows when a machine is available.
4. **Accessibility.** Same session: the popover opens with no control auto-focused, and once clicked, Tab moves through its controls with visible focus.

---

## Hard floors

None. No slice touches auth, authorization, payment, security or migrations. The probes read process names and mic-in-use flags; no audio, credentials or user content.

## Escalation log

- 2026-10-21 — slice 5 — supersedes Structure, Control flow, Standards — `fn check_once<R: Runtime>(app: &AppHandle<R>, probe: &dyn CallProbe, watch: &mut CallWatch, now: Instant)` called from `watch_forever` → no `check_once`. `watch_forever(app, probe, watch)` calls `probe.call_app_holds_microphone()`; on Err it logs at debug and skips the tick; on Ok it calls `watch.observe(holds, Instant::now())` inline, then `open_if_call_started<R: Runtime>(app: &AppHandle<R>, watch: &mut CallWatch)`, which checks `is_start_due`, calls `mark_reported`, then `open_unless_recording(app)`. Reason: the workspace clippy config denies `too_many_arguments` above 3 parameters, so the planned 4-parameter `check_once` fails `cargo clippy --workspace --all-targets -- -D warnings`. Behaviour and ordering are unchanged. Floor: unchanged. Already implemented in commit f7220cf.
- 2026-10-21 — planner lesson — the planner authored a 4-parameter `check_once` signature without checking the workspace clippy `too_many_arguments` limit (3), so the planned signature was not confidence-marked against the lint config; check lint argument limits when a plan fixes a signature.
