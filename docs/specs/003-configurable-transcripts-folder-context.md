# Task context: 003-configurable-transcripts-folder

> Generated for plan 003 "configurable-transcripts-folder" (standard tier, no spec) at HEAD 8855cf9a10cb37ee25c313d39f7a3895d52007a7 · 2026-10-19T12:17:47Z
> Read first by every agent in /anthara:plan-implementer's slice loop.

**No spec file exists**, so this file cites no spec anchors. The slices below follow the natural seams of the brief. If the plan slices the work differently, map onto these.
- **A** resolve the configured folder from `settings.json`, shared by the tray and the MCP binary.
- **B** month subfolders `YYYY-MM` in `MarkdownTranscriptStore`, with old flat files still readable.
- **C** tray menu item, native folder picker, persist the choice, and swap the folder at runtime.
- **D** replace the hardcoded "Documents/Scribe" copy.

**De-facto style:** Hexagonal, per `docs/ARCHITECTURE.md` (ADR 1). ARCHITECTURE §3 limits `scribe-mcp` to `scribe-core` + `store-markdown`. So the logic that reads `settings.json` to find the folder **must live in `store-markdown`**, beside `TranscriptsFolder`. It cannot live in the tray, and it cannot use Tauri types (§7). `scribe-core` stays filesystem-free: deriving the month from a `SessionId` is pure and may live in core, but creating folders stays in store-markdown.

## 1. Affected modules
| Module / file | Role | Notes |
|---|---|---|
| `crates/store-markdown/src/transcripts_folder.rs` | `TranscriptsFolder` (`default_location`, `at`, `ensure_exists`, `path`), `private_dir_builder` (0o700, recursive) | Home for the shared settings resolver (A) and for creating the month folder (B). Deps already include `dirs`, `serde`. `serde_json` is **not** a dep yet; the workspace pins `serde_json` with `preserve_order` |
| `crates/store-markdown/src/writer.rs` | `MarkdownTranscriptStore` (`new`, `pub path_for`, `try_create`, `transcript_paths`, the `TranscriptStore` impl), `is_transcript` | 186 lines. Today it writes `folder/<id>.md` and lists with a flat `read_dir`. Every port method goes through `path_for` or `transcript_paths` |
| `crates/store-markdown/src/reader.rs` | `read_document` (maps `NotFound` → `StoreError::NotFound`), `list_summaries`, `summary_or_skip` (warns and skips) | Unaware of paths. Takes the paths it is given |
| `crates/store-markdown/src/private_file.rs` | `create_new` (0o600, `create_new`), `append`, `replace_atomically` (`md.tmp` sibling) | The staging file sits beside the target, so it lands inside the month folder. No change expected |
| `crates/scribe-core/src/recording/session.rs` | `SessionId` (`parse`, `from_start`, `collision_candidates`, `as_str`), `STAMP_FORMAT` | The id's first 7 chars are `YYYY-MM`. `parse` checks digit counts only, not real dates |
| `crates/scribe-core/src/ports.rs` | `TranscriptStore`, `StoreError` (`DocumentsFolderUnavailable`, `CreateFolder`, `Io`, `NotFound`, `Malformed`, `NoFreeSessionId`) | Change the port only if a variant is truly needed. A settings-read failure should fall back, not error (see 4c) |
| `crates/scribe-core/src/recording/recorder.rs` | `Recorder { store: Arc<dyn TranscriptStore>, .. }` | The store is fixed in `Recorder::new`, with **no setter**. `session_writer` clones the `Arc` into each active `SessionWriter` |
| `crates/scribe-core/src/library/mod.rs` | `TranscriptLibrary` (`list`, `get`, `search`), `Arc<dyn TranscriptStore>` | `get` calls `SessionId::parse`, which is the MCP path-traversal guard |
| `apps/scribe-mcp/src/main.rs` | Composition root: `TranscriptsFolder::default_location()?` → store → library → `ScribeServer` | Resolves the folder **once at startup**. One-line swap to the shared resolver |
| `apps/scribe-tray/src-tauri/src/recording.rs` | `install` builds the one `Arc<MarkdownTranscriptStore>` from `default_location`, runs `mark_abandoned_sessions`, gives it to `Recorder` **and** `app.manage(store)` | The only place the store is built. `RecordingController` holds `Mutex<Recorder>` |
| `apps/scribe-tray/src-tauri/src/commands.rs` | `recent_sessions`, `open_session` read `app.state::<Arc<MarkdownTranscriptStore>>()` | `open_session` checks `path_for(&id).is_file()`. That breaks for month or flat files unless it gets a resolved path |
| `apps/scribe-tray/src-tauri/src/tray.rs` | `build_menu`, `dispatch`, `open_transcripts_folder` / `try_open_transcripts_folder` / `OpenFolderError` | 259 lines (clippy `too_many_lines = deny`). `try_open_transcripts_folder` calls `default_location()` **on its own**, as a third copy of the folder |
| `apps/scribe-tray/src-tauri/src/menu_action.rs` | `MENU_*` consts, `ACTIONS_BY_ID: [(&str, MenuAction); 6]`, `MenuAction` | `pub`, tested in `tests/menu_action.rs` |
| `apps/scribe-tray/src-tauri/src/privacy.rs` | `SETTINGS_FILE = "settings.json"`, `ACKNOWLEDGED_KEY`, `is_acknowledged` / `acknowledge` via `app.store(..)` | The pattern for reading and writing settings. `SETTINGS_FILE` is private |
| `apps/scribe-tray/src-tauri/src/notifications.rs` | `show_plain` + `notify_*` | 212 lines. Add outcome notifications here (optional per plan) |
| `apps/scribe-tray/src-tauri/src/start_failure.rs` | `StartFailureKind::advice() -> &'static str` | The `TranscriptsFolder` advice hardcodes "Documents/Scribe". Shown via `notify_recording_not_started` |
| `apps/scribe-tray/src-tauri/src/lib.rs` | `with_plugins`, `with_commands`, `start_scribe` | Add `.plugin(tauri_plugin_dialog::init())` here |
| `apps/scribe-tray/src-tauri/Cargo.toml` | Deps | `tauri-plugin-store = "2.4"` (locked 2.4.5), `dirs`, `serde_json`. **No `tauri-plugin-dialog` yet** |
| `apps/scribe-tray/src-tauri/capabilities/default.json` | `store:default`, `opener:allow-open-path` scoped to `$DOCUMENT/Scribe[/**]` | See dragons. Both opener calls happen in Rust, not JS |
| `apps/scribe-tray/src/privacy/PrivacyStatement.tsx` + `.spec.tsx` | `STATEMENTS` const, with the 4th line naming "Documents/Scribe" | The spec asserts the exact string twice (`getByText`, `toHaveAccessibleDescription`) |
| `docs/ARCHITECTURE.md` | §2 Storage row says "One Markdown file per session in `Documents/Scribe`" | Update it when the layout changes (docs drift) |

## 2. Patterns to follow
- **Locator with a default and an injectable path.** `TranscriptsFolder::default_location` (`dirs::*` + `.ok_or(StoreError::..)`) pairs with `TranscriptsFolder::at(PathBuf)`. The same shape appears in `claude_desktop/location.rs` (`ClaudeDesktopConfig::default_location` / `in_folder`). Do the same for the settings file: one fn takes an explicit settings path (testable), and one computes the real path.
- **Settings via the tauri-plugin-store in the tray.** `app.store(SETTINGS_FILE)?` → `store.set(KEY, json!(..))` → `store.save()`. Reads use `.ok().and_then(|s| s.get(KEY)).and_then(|v| v.as_..())`, with a default (`privacy.rs` (`is_acknowledged`)). Call `save()` explicitly so the MCP process sees the change on disk.
- **Tray action = pure core + try/notify shell.** Mirror `mcp_clipboard.rs` (`copy_config` / `try_copy_config` / `CopyError` with `#[error(transparent)] #[from]`), or `claude_desktop_connect.rs` (an outcome enum → `notifications::notify_*`). `dispatch` in `tray.rs` makes **one call** into the module.
- **Menu items: one const, one table row, one enum variant.** Follow `menu_action.rs`, then `enabled_item(app, MENU_X, "<Label>")` in `tray.rs` (`build_menu`), placed in the `Menu::with_items` slice, plus a `dispatch` arm.
- **Typed errors with `thiserror`.** Never return a sentinel. A missing or unreadable settings file is **not** an error for the resolver: it falls back to `Documents/Scribe` (brief).
- **Private filesystem modes.** Create folders only through `private_dir_builder` (0o700, recursive) and files through `private_file` (0o600).
- **Skip, don't fail, on bad files.** `reader::summary_or_skip` logs `file = %file_name(path)` at warn and goes on. Treat month folders the same way.
- **Logging (ARCHITECTURE §5):** session ids and file names only. Logging the chosen folder path is new, so the plan should decide whether to log it. Never log transcript text.
- **AI-ergonomics (advisory):** `tray.rs` is close to the line limit, so put the picker and the folder swap in a new module (e.g. `transcripts_location.rs`), not in `tray.rs`. Name `"transcripts_folder"`, `"com.incubyte.scribe"` and `"settings.json"` as `const`s. Today `SETTINGS_FILE` lives only privately in `privacy.rs`. Share one const (DRY) rather than adding a second literal.

## 3. Reusable utilities
- `store-markdown/src/transcripts_folder.rs` — `TranscriptsFolder::at` / `ensure_exists` / `path`; `private_dir_builder` (private; reuse it for month dirs inside the crate).
- `store-markdown/src/store_error.rs` — `file_name`, `malformed`, `io_error` (crate-private) for `StoreError` mapping.
- `store-markdown/src/reader.rs` — `list_summaries` / `list_all_summaries` take any iterator of paths. Feed them flat and month paths together.
- `scribe-core` `SessionId::as_str` — its first 7 chars are `YYYY-MM` by construction (`STAMP_FORMAT`). Prefer a named method (e.g. `SessionId::month`) over slicing in several places.
- `privacy.rs` — `SETTINGS_FILE`, `is_acknowledged` (the read/default idiom), `acknowledge` (the set+save idiom).
- `notifications.rs` — `show_plain(app, title, body)`.
- `dirs::data_dir()` (dirs 6) gives `~/Library/Application Support` on macOS and `{FOLDERID_RoamingAppData}` (`%APPDATA%`) on Windows. Tauri 2's `app_data_dir()` is `data_dir()/<identifier>`, and tauri-plugin-store resolves the relative `"settings.json"` under `BaseDirectory::AppData`. So the MCP binary computes `dirs::data_dir()/com.incubyte.scribe/settings.json`. Use `data_dir`, not `config_dir`: they match on macOS and Windows but differ on Linux (CI).
- `tempfile::tempdir()` — dev-dep in store-markdown, scribe-mcp and the tray.

## 4. Adjacent code that must coordinate (per slice)
- **Slice A (resolve):** `apps/scribe-mcp/src/main.rs`; `recording.rs` (`install`); `tray.rs` (`try_open_transcripts_folder`). All three call `default_location()` today, and all must use the same resolver. `privacy.rs` (`SETTINGS_FILE`).
- **Slice B (month folders):** `writer.rs` (`path_for`, `try_create`, `transcript_paths`, `mark_abandoned_incomplete`); `commands.rs` (`open_session` uses `path_for`); `TranscriptLibrary::get` / `search` (both go through `read`). Tests: `store-markdown/tests/*.rs`, `scribe-mcp/tests/mcp_tools.rs` (`folder_contents` does a flat `read_dir` + `fs::read` and will panic on a subdirectory).
- **Slice C (picker/persist/swap):** `menu_action.rs`, `tray.rs` (`build_menu`, `dispatch`), `lib.rs` (`with_plugins`), `recording.rs` (`RecordingController`, `install`), `commands.rs` (`recent_sessions`, `open_session`), the popover's `recent_sessions` refresh (`src/popover/commands.ts`), `notifications.rs`, Cargo.toml.
- **Slice D (copy):** `PrivacyStatement.tsx` + spec; `start_failure.rs` (`advice`) + `tests/start_failure.rs`; `docs/ARCHITECTURE.md` §2.

## 4a. Slice imitation targets
- **Slice A:** mirror `TranscriptsFolder::default_location` / `at` for shape, and `privacy.rs` (`is_acknowledged`) for "missing or wrong type → default". Expose an injectable form (settings path in, `TranscriptsFolder` out) so tests never touch the real app data dir.
- **Slice B:** mirror `writer.rs` (`transcript_paths`). Keep the `NotFound → empty` handling and add one level of `YYYY-MM` folders. For the read fallback, mirror `reader::read_document`'s `ErrorKind::NotFound → StoreError::NotFound { id }` mapping.
- **Slice C:** mirror `mcp_clipboard.rs` (`copy_config` / `try_copy_config` / `CopyError`) for the shell and `privacy.rs` (`acknowledge`) for persistence. The menu item follows `MENU_COPY_MCP_CONFIG`.
- **Slice D:** for the frontend, mirror `PrivacyStatement.spec.tsx` (testing-library + `vitest-axe`). For Rust, follow `tests/start_failure.rs` (`a_speech_engine_that_cannot_start_is_reported_with_advice`).

## 4b. Slice lexicon
- **Slice A:** key=`"transcripts_folder"`; file=`"settings.json"`; identifier=`"com.incubyte.scribe"` (from `tauri.conf.json`); base=`dirs::data_dir()`; fallback=`Documents/Scribe` (`FOLDER_NAME = "Scribe"`). Keep the type name `TranscriptsFolder` (not `TranscriptDir` or `SaveLocation`).
- **Slice B:** month folder=`YYYY-MM` (e.g. `2026-10`), from `SessionId`, not from `Local::now()`; file=`<id>.md`; ext const=`TRANSCRIPT_EXTENSION`; keep the names `path_for`, `transcript_paths`, `is_transcript`.
- **Slice C:** const=`MENU_CHOOSE_TRANSCRIPTS_FOLDER`; id=`"choose-transcripts-folder"` (kebab-case); variant=`MenuAction::ChooseTranscriptsFolder`; suggested label `"Choose transcripts folder…"`; dep=`tauri-plugin-dialog` 2.x (`DialogExt`, `app.dialog().file().pick_folder(..)`). Notifications follow `notify_<thing>_<outcome>` (e.g. `notify_transcripts_folder_changed` / `_not_changed`).
- **Slice D:** keep `StartFailureKind::TranscriptsFolder` / `"transcripts_folder"` (the wire value the frontend reads). Change only the text.

## 4c. Slice traps
- **Slice A:** (1) Do NOT add `tauri` or `tauri-plugin-store` to `store-markdown` or `scribe-mcp`. Read the file as plain JSON (`serde_json::from_slice` into a map/`Value`). (2) The file may be missing, unparseable, missing the key, or hold a non-string value. Every one of these falls back to `Documents/Scribe` and must never fail MCP startup. Test each case. (3) Do NOT compute the path with `config_dir` or a hand-built `~/Library/...`. Use `data_dir()/com.incubyte.scribe/settings.json`, so macOS and Windows match tauri-plugin-store. (4) Tests must inject the settings path. Never read the developer's real `settings.json` (CI Linux has no Tauri app data). (5) Store the path as a JSON string. Do NOT use `to_string_lossy` when persisting (it silently changes non-UTF-8 paths). Refuse instead.
- **Slice B:** (1) Existing flat `folder/<id>.md` files must still list, read, get marked abandoned on launch, and open. Do NOT migrate or move them (brief). (2) Look inside **only** folders whose names match `YYYY-MM` exactly, one level deep. Never `walkdir`-recurse, because the user may pick a large folder such as `~/Documents`. (3) Create the month folder with `private_dir_builder` (0o700). Do NOT use `fs::create_dir_all`, and do NOT chmod a user-picked parent folder. (4) `try_create` probes `collision_candidates` with `create_new` inside the month folder. Do not break the `-2..-99` suffix behaviour (`a_second_call_started_in_the_same_second_gets_its_own_file`). (5) `path_for` is `pub` and used by `open_session` and tests. If it changes meaning, `read`/`append`/`finalise` must find a legacy flat file before giving `NotFound`. Update tests that assume a flat layout (`transcript_text`, `file_names_in`, `folder_contents`, the `0o600` permission test) to the new layout. Do not loosen their assertions.
- **Slice C:** (1) `Recorder` has no store setter, and `app.manage` **cannot replace** state that is already managed (it returns `false`). Re-calling `recording::install` does not switch folders. (2) An active `SessionWriter` holds its own `Arc` of the store. If the folder is swapped *inside* the store mid-recording, `append`/`finalise` resolve to the new folder and fail with `NotFound`. Either refuse or defer the change while `RecorderState::Recording`, or pin the folder per session. (3) Menu callbacks run on the main thread. Do NOT call `blocking_pick_folder` there, because it deadlocks. Use the callback `pick_folder`. (4) A cancelled picker (`None`) is a no-op: no save, no error notification. (5) After a swap, `recent_sessions`, `open_session` and `open_transcripts_folder` must all use the new folder. Today `tray.rs` recomputes `default_location()` on its own. Remove that copy. (6) Picking a folder is not destructive, so no confirmation or blast-radius convention applies (the project has neither). Old transcripts stay where they are. Decide whether the user is told this.
- **Slice D:** (1) `advice()` returns `&'static str`. Keep that signature. Use generic wording (e.g. "your transcripts folder") rather than threading a path through. (2) Update the exact-string assertions in `PrivacyStatement.spec.tsx` (both of them) together with the copy. (3) Do NOT rename `StartFailureKind::TranscriptsFolder` or `"transcripts_folder"`.

## 5. Known dragons
- No `TODO`/`FIXME`/`LEGACY` markers in the walked modules.
- **Three independent folder resolutions** (`scribe-mcp` main, `recording::install`, `tray::try_open_transcripts_folder`). They agree today only because all three are hardcoded.
- **MCP resolves the folder once at startup.** Claude Desktop keeps `scribe-mcp` running, so a change made in the tray is not seen until the MCP process restarts, unless the plan resolves on each request (see §8).
- **Permissions:** the folder is 0o700 and files are 0o600 (`transcripts_folder.rs` `OWNER_ONLY_MODE`, `private_file.rs` `OWNER_READ_WRITE`), enforced only on unix. Windows inherits the ACL. `ensure_exists` does not chmod an existing folder. Keep it that way for user-picked folders.
- **Path safety in MCP** depends entirely on `SessionId::parse` (digits and dashes only) in `TranscriptLibrary::get` and is tested by `an_id_that_tries_to_leave_the_transcripts_folder_is_rejected`. Build month paths from a parsed `SessionId` only, never from raw input.
- **Noisy listing in broad folders:** `summary_or_skip` warns for every non-transcript `.md` file. If the user picks `~/Documents`, every unrelated `.md` there logs a warning on each list.
- **Capability scope is stale but harmless:** `opener:allow-open-path` is scoped to `$DOCUMENT/Scribe`, but the opener is only called from Rust (`tray.rs`, `commands.rs` `open_session`), and Rust calls aren't capability-checked. The dialog is likewise Rust-only, so it needs no `dialog:*` permission. Do NOT widen the scope to `**`.
- **macOS accessory app:** `ActivationPolicy::Accessory` (`lib.rs` `run`) can open the native folder panel behind other windows. Verify by hand.
- CI's Linux runner synthesises a Documents dir (`.github/workflows/ci.yml`) for `default_location_is_a_scribe_folder_in_documents`. CI also enforces that `scribe-mcp` has no audio/STT deps. Keep tauri out too (ARCHITECTURE §3).

## 6. Test conventions
- Rust: integration tests only, in `<crate>/tests/<topic>.rs` (no co-located `#[cfg(test)]`). Names are behaviour sentences in snake_case (`a_folder_that_does_not_exist_yet_lists_nothing`). Arrange/act/assert are separated by blank lines. Each file has local helpers such as `store_in(parent)` → `MarkdownTranscriptStore::new(TranscriptsFolder::at(parent.join("Scribe")))`.
- Filesystem: `tempfile::tempdir()` with an injected path. Unix modes are checked under `#[cfg(unix)]` with `mode & 0o777`. Legacy-layout tests can `fs::write(folder.join("<id>.md"), ..)` directly, as the corrupt-file tests do.
- scribe-core fakes: `scribe_core::testing` (feature `testing`), e.g. `RecordingListenerSpy`, `FakeAudioSource`.
- MCP: `apps/scribe-mcp/tests/mcp_tools.rs` (`connect_to(folder)` over `tokio::io::duplex`, `call`, `listed_ids`, `text_of`, `is_tool_error`).
- Tray: `apps/scribe-tray/src-tauri/tests/*.rs` against `pub mod`s only. `tests/menu_action.rs` follows the `<id>_id_maps_to_<action>` pattern. Code that needs an `AppHandle` (picker, `app.store`) is untested by convention. Keep it thin.
- Frontend: vitest + testing-library + `vitest-axe`, co-located `*.spec.tsx`. Mock IPC with `@tauri-apps/api/mocks` `mockIPC`. Run `npm test` and `npm run lint` in `apps/scribe-tray`.
- Run order: `apps/scribe-tray/scripts/stage-sidecar.sh` → `cargo fmt --all` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo test --workspace`. CI runs `cargo mutants --in-diff` and Stryker, so every fallback branch (missing file, bad JSON, missing key, non-string, month vs flat lookup) needs an asserting test.

## 7. Hot paths and performance landmines
- `list_all` → `TranscriptLibrary::search` reads **every** transcript on each MCP search (`tests/transcript_search_performance.rs` guards this). Month folders add one `read_dir` per month. Do not add a per-file `read_dir` or recursive walk.
- `recent_sessions` runs on each popover open. Listing must stay cheap.
- `append` runs once per utterance while recording. Do NOT re-read `settings.json` or re-scan folders on the append path. Resolve the path once per session.

## 8. Open uncertainties resolved during orientation
- **"Can the Recorder's store be swapped at runtime?"** Not as built. It is built once in `recording::install`, fixed in `Recorder::new`, and duplicated as managed state `Arc<MarkdownTranscriptStore>`. The plan must add a swap point: interior mutability on the folder, a new store handed to `RecordingController` while idle, or a wrapper port. It must also keep the active session pinned (trap C2).
- **"Where is the Tauri app-data dir computed?"** By tauri's path resolver as `dirs::data_dir()/<identifier>`. `settings.json` sits directly in it. The MCP binary can produce the identical path with `dirs::data_dir()` plus the hardcoded identifier `com.incubyte.scribe`. Store format: a pretty-printed JSON object of keys, e.g. `{"privacy_acknowledged": true, "transcripts_folder": "<path>"}`.
- **"Where does the shared resolver live?"** In `store-markdown`. It is the only crate both apps may depend on that may touch the filesystem. It needs `serde_json.workspace = true`.
- **Open for the plan:** whether MCP resolves the folder per request or at startup; whether to notify on success; whether the "Open transcripts folder" action ensures the custom folder exists (today it calls `ensure_exists`).

## 8a. Design system (UI touch is copy-only)
- **Source of truth:** CSS custom properties in `apps/scribe-tray/src/theme/tokens.css`, plus `primitives.css` and `popover.css`. There is no `design.md`.
- **Component library:** none as such. The privacy window uses BEM classes (`window`, `checklist__item`, `button`). The folder picker is a native OS dialog, so no new web UI is needed.
- **Per-slice UI note:** **Slice D:** edit only the `STATEMENTS` entry in `PrivacyStatement.tsx`. Keep the four-item list, the single button and the `aria-describedby` wiring that the a11y spec asserts.

---
## Changelog
- 2026-10-19 — generated by code-orienteer from plan 003 (no spec) at HEAD 8855cf9.
