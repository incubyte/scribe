# Task context: 008-chrome-like-auto-update

> Generated for spec 72fde7d0-4a8a-48f8-a9ea-28b402819342 at HEAD b15aebc288355a51c623fffb61518659f22fd848 · 2026-10-23
> Read first by every agent in /anthara:plan-implementer's slice loop.

Style: **hexagonal** (ADR 1, `docs/ARCHITECTURE.md` §1, §3, §7). Pure rules live in `crates/scribe-core`; Tauri types never go below `apps/`. The app shell is a set of small `install(app)` modules wired by `lib.rs` (`start_scribe`).

## 1. Affected modules
| Module / file | Role | Notes |
|---|---|---|
| `scripts/release.sh` | Local release pipeline (ADR 4, 8) | `set -euo pipefail`; top-level vars, then one `snake_case` function per step, then a flat call list at the bottom |
| `apps/scribe-tray/src-tauri/tauri.conf.json` | Base config | `signingIdentity: "-"` so keyless builds work; no `plugins` block yet |
| `src-tauri/tauri.ci.conf.json` | Overlay passed via `--config` by `release.sh` **and** `.github/workflows/release.yml` | Overrides only `beforeBuildCommand` |
| `src-tauri/tauri.windows.conf.json` | Auto-merged platform overlay | VC++ resource and `installerHooks` |
| `src-tauri/capabilities/default.json` | Webview permissions | Must NOT gain `updater:*` (AC 5.2.8) |
| `src-tauri/Cargo.toml` | Shell dependencies | Tauri plugins pinned as `"2.x"` strings |
| `src-tauri/src/lib.rs` | Composition root | `with_plugins`, `start_scribe` order |
| `src-tauri/src/tray.rs`, `menu_action.rs` | Tray menu, dispatch, `quit` | 271 lines; largest shell file touched |
| `src-tauri/src/call_watch.rs` | `call-watch` thread polling `CallProbe` every 2 s | Owns `CallWatch` as a thread-local |
| `src-tauri/src/popover.rs` | Popover window | No public "is open" query exists |
| `crates/scribe-core/src/call/call_watch.rs` | `CallWatch` state machine | Has `in_call` field, no public getter |
| `src-tauri/windows/hooks.nsh` | NSIS hooks | Only `NSIS_HOOK_POSTINSTALL` today |
| `README.md`, `docs/ARCHITECTURE.md`, `CLAUDE.md` | Docs | Network sentence in README "Privacy" paragraph; ADR list in ARCHITECTURE §9 |

## 2. Patterns to follow
- **Module `install(app)` entry point** — each shell concern exposes `pub fn install<R: Runtime>(app: &AppHandle<R>)` that `app.manage(...)`s its state and spawns its thread. Defined in `call_watch.rs` (`install`), `model_setup.rs` (`install`).
- **Named background thread** — `thread::Builder::new().name(THREAD_NAME.to_owned()).spawn(...)`, failure logged with `tracing::error!`. Defined in `call_watch.rs` (`install`, `watch_forever`). No async runtime is used anywhere in the shell yet; the updater plugin is async, so `tauri::async_runtime::block_on`/`spawn` is the only sanctioned bridge (do not add `tokio`).
- **Managed state behind a Mutex, poison-tolerant** — `lock().unwrap_or_else(PoisonError::into_inner)`. `recording.rs` (`RecordingController::recorder`), `popover.rs` (`with_guard`).
- **Single-flight with an `AtomicBool` claim** — `compare_exchange(false, true, AcqRel, Acquire)`. `model_setup.rs` (`ModelSetup::begin_attempt`), `recording.rs` (`StartClaim`). Fits AC 5.2.6.
- **Logging** — `tracing` with Display field then a lowercase sentence: `tracing::error!(%error, "could not start watching for calls")`; enum-ish context as `kind = ...`. Warn for recoverable UI glitches (`log_failure` helpers in `tray.rs`, `popover.rs`), error for lost work. Never log user data (ARCHITECTURE §5).
- **Errors** — `thiserror` enums with `#[error(transparent)] #[from]` for adapters, a `try_*` fn returning `Result` and a public wrapper that logs. `tray.rs` (`try_open_transcripts_folder`, `OpenFolderError`).
- **Pure policy in core, free function over plain data** — `crates/scribe-core/src/recording/readiness.rs` (`recorder_state(&Readiness, bool) -> RecorderState`), `#[must_use]`, exhaustive `match` on a tuple.
- **Shell-pure helpers as `pub mod`** so `tests/` can import them: `blur_guard`, `tray_anchor`, `menu_action` in `lib.rs`. AppHandle glue stays private (`mod popover;`) and is excluded from mutation in `.cargo/mutants.toml`.
- **Naming** — `install` (wire), `publish_*` (emit state to tray + webview), `show_*` (tray look), `notify_*` (OS notifications), `*Controller` (managed state wrapper), `*Watch` (time-based observer), `*Guard` (pure UI gate), `MENU_*` id consts.
- **Clippy limits are hard errors** (`clippy.toml`): 20 lines per fn, max 3 args, cognitive complexity 5, pedantic warn with `-D warnings`.
- **Advisory (ai-ergonomics):** `tray.rs` at 271 lines mixes menu build, look, and dispatch; put update menu logic in the updates module rather than growing `tray.rs`.

## 3. Reusable utilities
- `crates/scribe-core/src/recording/readiness.rs` — recorder states (`RecorderState::{Locked, Idle, Recording}`)
- `src-tauri/src/recording.rs` — current recorder state; stop-and-save (`RecordingController::state`, `RecordingController::stop`)
- `crates/scribe-core/src/call/mod.rs` — grace constant and watch (`CALL_END_GRACE`, `CallWatch`)
- `src-tauri/src/tray.rs` — menu item helper and tray id (`enabled_item`, `TRAY_ID`, `log_failure`)
- `src-tauri/src/popover.rs` — window lookup (`popover_window`, `WINDOW_LABEL`)
- `scripts/release.sh` — prerequisite gate shape (`require_notary_credentials`), upload (`publish_assets`), Gatekeeper check (`notarize_macos` uses `spctl --assess`)

## 4. Adjacent code that must coordinate (per slice)
- **Slice 5.1 (manifest):** `.github/workflows/release.yml` (uses `tauri.ci.conf.json`); `scripts/release-notes.md` (draft notes; `latest.json` `notes` can reuse it); ARCHITECTURE §5 Privacy and §6 Integration table.
- **Slice 5.2 (check/download):** `lib.rs` (`with_plugins`, `start_scribe`); `.github/workflows/ci.yml` reqwest guard step; `README.md` Privacy paragraph; `PrivacyStatement.tsx` sentence must stay true.
- **Slice 5.3 (install on quit):** `tray.rs` (`quit`, `dispatch`); `recording.rs` (`RecordingController::stop`).
- **Slice 5.4 (sidecar unlock):** `hooks.nsh`; `tauri.windows.conf.json` (`installerHooks`); README Claude Desktop setup section (AC 5.4.4).
- **Slice 5.5 (idle install):** `call_watch.rs` (`install`, `watch_forever`); `popover.rs`; `crates/scribe-core/src/lib.rs` module list.
- **Slice 5.6 (restart item):** `tray.rs` (`build_menu`, `RecordingMenuItem`, `show_recorder_state`); `menu_action.rs` (`ACTIONS_BY_ID`); `recording.rs` (`publish_state`, which already calls into the tray on every state change).

## 4a. Slice imitation targets
- **Slice 5.1:** mirror `scripts/release.sh` (`require_notary_credentials`) for key/password gates (stderr message naming the setup command, `exit 1`), and (`build_macos`) for passing env + `--config`.
- **Slice 5.2:** mirror `model_setup.rs` (`install`, `spawn_attempt`, `ModelSetup`): managed state struct, `AtomicBool` single-flight, thread with outcome `match` that calls `show_*`.
- **Slice 5.3:** extend `tray.rs` (`quit`); keep the stop-then-exit order.
- **Slice 5.4:** mirror `hooks.nsh` (`NSIS_HOOK_POSTINSTALL`) macro shape with `!define` constants on top.
- **Slice 5.5 (core policy):** mirror `readiness.rs` (`recorder_state`) + `tests/recorder_readiness.rs`; new `crates/scribe-core/src/update/` module with `mod.rs` re-exports like `call/mod.rs`. In-call query: add a `#[must_use] pub fn` on `CallWatch` next to `is_start_due`.
- **Slice 5.6:** mirror `tray.rs` (`RecordingMenuItem` newtype in managed state, `enable_toggle`/`disable_toggle`) and `menu_action.rs` (const id + `ACTIONS_BY_ID` entry + test in `tests/menu_action.rs`).

## 4b. Slice lexicon
- **Slice 5.1:** overlay=`tauri.release.conf.json`; env=`TAURI_SIGNING_PRIVATE_KEY_PATH`/`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; fns=`require_update_signing_key`, `write_update_manifest`; asset=`latest.json`; platform keys=`darwin-aarch64`, `windows-x86_64`.
- **Slice 5.2:** module=`updates` (shell); fn=`updates::install`; consts=`CHECK_INTERVAL` (1 h), `THREAD_NAME = "update-check"`; endpoint=`https://github.com/incubyte/scribe/releases/latest/download/latest.json`; states=`UpToDate`, `Downloading`, `UpdateReady`, `Installing`.
- **Slice 5.3:** fn=`updates::install_on_quit` called from `tray::quit`.
- **Slice 5.4:** macro=`NSIS_HOOK_PREINSTALL`; process=`scribe-mcp.exe`.
- **Slice 5.5:** core module=`scribe_core::update`; type=`InstallMoment` (`Now` / `Wait(WaitReason)`); fn=`install_moment(...)`; reasons=`Recording`, `InCall`, `PopoverOpen`, `NothingReady`; `CallWatch::is_in_call`; `popover::is_open`.
- **Slice 5.6:** const=`MENU_RESTART_TO_UPDATE = "restart-to-update"`; variant=`MenuAction::RestartToUpdate`; label=`"Restart to update"`.

## 4c. Slice traps
- **Slice 5.1:** (1) do NOT add `createUpdaterArtifacts` or a `plugins.updater` signing need to `tauri.ci.conf.json` or the base config in a way that requires the key — `release.yml` and CI build with that overlay and have no key (invariant 4.7, [spec §9.4](008-chrome-like-auto-update.md#94-the-windows-release-fallback)). (2) never `echo`/`cat` the key, password or `.sig` contents; read the password with `security find-generic-password -w` into an env var only. (3) build the updater archive in the same `build_macos` run as the notarized `.dmg`, and re-run `spctl` on the extracted `.app` (AC 5.1.4). (4) the `.sig` filename and `.app.tar.gz` name come from Tauri; glob them from `target/release/bundle/macos/` instead of hardcoding. (5) gates run before `build_macos`, next to `require_notary_credentials`.
- **Slice 5.2:** (1) adding `tauri-plugin-updater` pulls in `reqwest`, which trips CI's "Only the Parakeet adapter may depend on an HTTP client" step in `ci.yml`; update that guard deliberately (allow `tauri-plugin-updater`) and note it in ARCHITECTURE §5 Privacy. (2) call `updates::install` in `start_scribe` before `recording::install`, which returns early with `?` on store errors (invariant 4.5); do not make it fallible with `?`. (3) do not add `updater:default` to `capabilities/default.json`. (4) use `check()` + `download()` and keep the bytes; `download_and_install` would install immediately. (5) one log line per failure, no retry loop inside the hour.
- **Slice 5.3:** (1) `RecordingController::stop` must finish before install. (2) on Windows the plugin's `install` runs the NSIS installer and exits the process itself, so nothing after it runs; on macOS call `app.exit(0)` afterwards. (3) an install error is logged and Scribe still exits (AC 5.3.4).
- **Slice 5.4:** (1) match processes by full image path under `$INSTDIR`, not by name alone (AC 5.4.2). (2) keep `NSIS_HOOK_POSTINSTALL` and its VC++ step untouched. (3) a missing process is not an error.
- **Slice 5.5:** (1) `CallWatch` lives only inside the `call-watch` thread (`call_watch.rs` (`install`)); share "in call" through managed state such as an `AtomicBool`, not by moving `CallWatch` into a Mutex polled from another thread. (2) "in call" must include the 30 s grace, so read `CallWatch`'s `in_call`, not the raw probe result. (3) treat `RecorderState::Locked` as not recording. (4) no `#[cfg(target_os)]` or Tauri types in `scribe-core` (ARCHITECTURE §7). (5) relaunch with `AppHandle::restart`, and show no notification or window.
- **Slice 5.6:** (1) Tauri `MenuItem` has no visibility toggle; insert and remove the item from the `Menu`, which therefore has to be kept in managed state (today only the toggle item is). (2) disabled-while-recording follows recorder changes, so hook it from the same path as `tray::show_recorder_state`. (3) pressing it goes through the same install path as idle install, not a second copy.

## 5. Known dragons
- `.github/workflows/ci.yml` reqwest guard — see 5.2 trap (1). The `scribe-mcp` dependency guard is unaffected.
- `ci.yml` `windows` job only runs clippy on `scribe-tray`; Windows-only updater code is never tested in CI.
- `.cargo/mutants.toml` — PR mutation threshold is 75%. Add the updates module's AppHandle glue to `exclude_globs` like `call_watch.rs`/`popover.rs`, and keep the logic in tested pure code.
- `tray_listener.rs` already has a struct named `TrayUpdater`, which refreshes the tray. Avoid the bare word "updater" for new shell types.
- No TODO/FIXME/LEGACY markers in the touched areas.
- ADR 6 Windows install is still "pending a first install on real Windows hardware"; slices 5.3 and 5.4 also need Windows hardware.

## 6. Test conventions
- Core: integration-style tests in `crates/scribe-core/tests/<topic>.rs` that import `scribe_core::...`. No inline `#[cfg(test)]` modules anywhere. Run: `cargo test -p scribe-core`.
- Shell: pure `pub mod` units tested from `apps/scribe-tray/src-tauri/tests/<module>.rs` (`blur_guard.rs`, `tray_anchor.rs`, `menu_action.rs`). Code that needs `AppHandle` is not unit-tested; keep it thin. Run: `cargo test -p scribe-tray` (stage the sidecar first).
- Names are business-language snake_case sentences (`a_release_lasting_exactly_the_grace_ends_the_call`), with arrange / act / assert separated by blank lines and named consts for times (`GRACE`, `ONE_MILLISECOND_SHORT_OF_GRACE`). Time is injected as `Instant` arguments, never read inside the policy.
- Frontend untouched; `npm test` must still exit 0.

## 7. Hot paths and performance landmines
- `call_watch.rs` (`watch_forever`) polls every 2 s; publishing the in-call flag must stay a cheap atomic store.
- `tray.rs` (`dispatch`) runs on the main/event thread; do not block it on network download (install on quit is the only synchronous step).
- `RecordingController` mutex is held during start/stop; the idle check reads `state()` only and never holds the lock across install.

## 8. Open uncertainties resolved during orientation
- [Spec §9.2](008-chrome-like-auto-update.md#92-does-tauri-build-fail-or-skip-signing-when-createupdaterartifacts-is-on-and-no-private-key-is-set): still unconfirmed, but `release.sh` already passes `--config src-tauri/tauri.ci.conf.json`, and Tauri accepts more than one `--config`. A new release-only overlay can be added without touching the CI overlay.
- [Spec §9.4](008-chrome-like-auto-update.md#94-the-windows-release-fallback): confirmed. `release.yml` builds with `tauri.ci.conf.json`, so updater artifacts must not live there.
- Popover visibility: there is no query today (`toggle`/`open_for_call` call `window.is_visible()` inline), so 5.5 adds `popover::is_open`.
- In-call state is not shared outside the `call-watch` thread today, so 5.5 has to publish it.
- [Spec §9.1](008-chrome-like-auto-update.md#91-does-claude-desktop-restart-a-stopped-scribe-mcp-server-on-its-own): unresolved. The README already tells users to restart Claude Desktop after connecting.

## 8a. Design system (UI slices only)
- **Source of truth:** the only UI change (5.6) is the native tray menu, built with `tauri::menu` in `tray.rs` (`build_menu`), so web design tokens do not apply. The popover theme lives in `apps/scribe-tray/src/theme` and is not touched.
- **Conventions:** sentence-case labels with no trailing punctuation ("Quit Scribe", "Copy MCP config"). A disabled state is set with `set_enabled(false)`, as for the recording toggle while `Locked`.
- **Slice 5.6:** imitate `tray.rs` (`recording_toggle_item`, `enabled_item`). The item goes just above `quit` in the `Menu::with_items` order.

---
## Changelog
- 2026-10-23 — generated by code-orienteer from spec 72fde7d0-4a8a-48f8-a9ea-28b402819342 at HEAD b15aebc.
