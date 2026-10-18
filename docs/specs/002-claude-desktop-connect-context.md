# Task context: 002-claude-desktop-connect

> Generated for plan 002 "claude-desktop-connect" (standard tier, no spec) at HEAD b5d1e32717e60d5c0a7ac642f6162f5f88d4e31f · 2026-10-18T09:49:58Z
> Read first by every agent in /anthara:plan-implementer's slice loop.

**No spec file exists.** No spec or plan file exists yet for 002, so this file cites no spec anchors. The slices below are the natural seams of the brief: **A** pure merge, **B** locate + back up + write atomically, **C** tray menu item + dispatch + notifications. If the plan slices the work differently, map onto these.

**De-facto style:** Hexagonal, per `docs/ARCHITECTURE.md`. `apps/scribe-tray` is a primary adapter and holds the per-OS wiring, so this whole feature lives in `apps/scribe-tray/src-tauri/src`. Do not add it to `scribe-core`. ARCHITECTURE §7 forbids Tauri types below `apps/` and "domain code touching the filesystem".

## 1. Affected modules
| Module / file | Role | Notes |
|---|---|---|
| `src/mcp_config.rs` | Pure MCP config: `bundled_mcp_path`, `config_snippet`, `McpConfigError` | `pub` and exported from `lib.rs` so `tests/` can reach it. Put the merge logic here or in a new sibling `pub` module |
| `src/mcp_clipboard.rs` | Orchestrator for "Copy MCP config": try-fn → notify ok/failed | Private module. Keep it unchanged. The new feature gets its own sibling (e.g. `claude_desktop.rs`) |
| `src/menu_action.rs` | Menu id constants, `ACTIONS_BY_ID` table, `MenuAction` enum | `pub`. Pure, unit-tested |
| `src/tray.rs` | `build_menu` (items + order) and `dispatch` (enum → handler) | 246 lines. Pedantic clippy with `too_many_lines = deny`, so keep new handlers out of here |
| `src/notifications.rs` | All user-facing title/body consts + `notify_*` fns | 179 lines. Private module |
| `src/lib.rs` | Module list + plugin wiring | Add the `mod` line. No new plugin needed |
| `apps/scribe-tray/src-tauri/Cargo.toml` | Deps | `serde_json` is present (workspace `"1"`, default features). `dirs` and `tempfile` are **not** deps of scribe-tray yet. Both are in `[workspace.dependencies]` (`dirs = "6"`, `tempfile = "3"`) |
| `tests/mcp_config.rs`, `tests/menu_action.rs` | Integration tests against the `scribe_tray` lib | New pure fns must be `pub` and in a `pub mod` to be tested here |

## 2. Patterns to follow
- **Split each tray action into a pure core and a try/notify shell.** The pure core is `mcp_config.rs` (`config_snippet`, returns `String`). The shell is `mcp_clipboard.rs` (`copy_config`), shaped as `match try_x(app) { Ok(()) => notify_ok(app), Err(error) => { tracing::error!(%error, "..."); notify_failed(app) } }`.
- **Typed errors with `thiserror`.** Module-local enums compose via `#[error(transparent)] #[from]`, e.g. `mcp_clipboard.rs` (`CopyError`) wrapping `McpConfigError`. Never return a sentinel or null (ARCHITECTURE §5 Errors).
- **Menu items: one const, one table row, one enum variant.** Add `pub const MENU_<X>: &str = "<kebab-id>"` and a row in `ACTIONS_BY_ID` (bump the array length `[(&str, MenuAction); 5]`) in `menu_action.rs`. Then add a `MenuAction::<X>` variant and an `enabled_item(app, MENU_<X>, "<Label>")` in `tray.rs` (`build_menu`), placed in the `Menu::with_items` slice. Then add a `dispatch` arm calling `<module>::<fn>(app)`.
- **Notification text lives as `const` title/body pairs** in `notifications.rs`, exposed via `pub fn notify_<event><R: Runtime>(app: &AppHandle<R>)` that calls `show_plain`. Copy uses sentence case and "Scribe" in the third person, and the body gives the next action.
- **Per-OS notifications.** Plain notifications on **every** OS go through `show_plain` → `tauri-plugin-notification` (`app.notification().builder()`). Only *actionable* (button) notifications differ by OS. macOS uses `mac-notification-sys` in `mod system_settings`, on a spawned thread with a dev sender `com.apple.Terminal`. Windows uses `tauri-winrt-notification::Toast` in `mod privacy_settings`, with the dev sender `Toast::POWERSHELL_APP_ID`. This feature only needs plain notifications. Do not add a button unless the plan asks for one.
- **Locate files with a default and an injectable path.** Mirror `store-markdown/src/transcripts_folder.rs` (`TranscriptsFolder::default_location` uses `dirs::*` with `.ok_or(TypedError)`, and `TranscriptsFolder::at(PathBuf)` lets tests inject a path). Also see `stt-parakeet/src/models_location.rs` (`default_models_dir`).
- **Atomic write.** Follow `crates/store-markdown/src/private_file.rs` (`replace_atomically`): write to a sibling staging path (`path.with_extension("md.tmp")`), then `write_all` + `sync_data`, then `fs::rename` over the target. New files are opened with `private_options` (unix `0o600`). The module is `pub(crate)` and Markdown-specific, so it **cannot be imported** from the tray. Copy the shape into the tray module and do not export it from store-markdown.
- **Platform consts with `cfg!`/`#[cfg]`.** Example: `mcp_config.rs` (`MCP_BINARY_NAME`). Keep `#[cfg]` out of pure logic where possible.
- **AI-ergonomics (advisory):** `tray.rs` and `notifications.rs` are the largest files. Add one line per item to them, not logic. Name every new path segment or JSON key as a `const` (`"mcpServers"`, `"scribe"` and `"command"` are currently inline literals inside `config_snippet`).

## 3. Reusable utilities
- `src/mcp_config.rs` — `bundled_mcp_path()`, which resolves `scribe-mcp[.exe]` beside `current_exe` and errors `BundledServerMissing(PathBuf)`. Reuse it as the `command` value. Do not recompute it.
- `src/mcp_config.rs` — `McpConfigError`, which the new error enum wraps via `#[from]`.
- `src/notifications.rs` — `show_plain(app, title, body)` (private, so add the `notify_*` fns in this file).
- `dirs::config_dir()` (dirs 6) returns `~/Library/Application Support` on macOS and `{FOLDERID_RoamingAppData}` (= `%APPDATA%`) on Windows. One call covers both required paths, joined with `Claude/claude_desktop_config.json`. Add `dirs.workspace = true` to the tray's `[dependencies]`.
- `serde_json::Value` / `Map` for the merge. `json!` is already used in `mcp_config.rs` and `privacy.rs`.
- `tempfile::tempdir()` is the test-folder idiom (see `crates/store-markdown/tests/transcripts_folder.rs`). Add `tempfile.workspace = true` under a new `[dev-dependencies]` in the tray Cargo.toml.

## 4. Adjacent code that must coordinate (per slice)
- **Slice A (merge):** `mcp_config.rs` (`config_snippet`). The clipboard snippet and the merged entry must give the same `{"command": <path>}` shape. Share the key consts or a `scribe_server_entry(path)` builder (DRY).
- **Slice B (locate/backup/write):** `mcp_config.rs` (`bundled_mcp_path`). Use `private_file.rs` (`replace_atomically`) as the reference shape.
- **Slice C (menu/notify):** `menu_action.rs` (`ACTIONS_BY_ID`, `MenuAction`), `tray.rs` (`build_menu`, `dispatch`), `notifications.rs`, `lib.rs` (mod list), `tests/menu_action.rs`.

## 4a. Slice imitation targets
- **Slice A:** mirror `mcp_config.rs` (`config_snippet`). It is a `#[must_use] pub fn` taking `&Path`, pure, with no I/O. For the merge, take the existing text (or `Option<&str>` when the file is absent) plus the server path, and return `Result<String, TypedError>`.
- **Slice B:** mirror `transcripts_folder.rs` (`TranscriptsFolder::default_location` / `::at`) for locating. Mirror `private_file.rs` (`replace_atomically`) for the write. Errors follow `McpConfigError`: one `thiserror` enum with a `#[source] io::Error` per I/O step.
- **Slice C:** mirror `mcp_clipboard.rs` (`copy_config` / `try_copy_config` / `CopyError`) exactly, and `notifications.rs` (`notify_mcp_config_copied` / `notify_mcp_config_not_copied`).

## 4b. Slice lexicon
- **Slice A:** keys=`mcpServers`, `scribe`, `command`. Reuse the server name `"scribe"` from `config_snippet`, not `"Scribe"` or `"scribe-mcp"`.
- **Slice B:** dir=`Claude`; file=`claude_desktop_config.json`; base=`dirs::config_dir()`. Proposed staging=`with_extension("json.tmp")`, following the `md.tmp` precedent. No backup-suffix convention exists, so pick one name (e.g. `claude_desktop_config.json.bak`) and make it a const.
- **Slice C:** const=`MENU_CONNECT_CLAUDE_DESKTOP`; id=`"connect-claude-desktop"` (kebab-case like `copy-mcp-config`); variant=`MenuAction::ConnectClaudeDesktop`; label=`"Connect to Claude Desktop"` (from the brief); keep `MENU_COPY_MCP_CONFIG` / `"Copy MCP config"`. Notify fns follow `notify_<thing>_<outcome>`, e.g. `notify_claude_desktop_connected`, `notify_claude_desktop_missing`, `notify_claude_desktop_config_invalid`, `notify_claude_desktop_not_connected`.

## 4c. Slice traps
- **Slice A:** (1) do NOT rebuild the document from scratch. Parse it, then replace only `mcpServers.scribe` and keep every other top-level key and server. (2) Invalid JSON **and** valid-but-wrong-shape JSON (root not an object, or `mcpServers` not an object) must both give a typed refusal. Never overwrite them with a fresh object. (3) Key order: `serde_json` is built **without** `preserve_order`, so `Map` is a BTreeMap and re-serialising sorts the user's keys alphabetically. Values survive but the order does not. Decide explicitly whether that is acceptable (see §8). (4) Windows paths must stay intact through escaping. Copy the backslash/space/quote cases from `tests/mcp_config.rs`.
- **Slice B (destructive, overwrites a user file):** (1) the project has **no** confirmation-dialog convention (it's a tray app without modal dialogs), and the brief asks for none. The safety net is the backup plus the atomic rename. Do not invent a confirm step. (2) The project also has **no** impact-count/blast-radius convention, so do not invent one. (3) Order: read → validate/merge in memory → back up the original → stage `.tmp` → `sync_data` → `rename`. Nothing touches disk if the parse fails. (4) "Claude folder missing" (install prompt) is a different case from "file missing" (create it). Do NOT `create_dir_all` the `Claude` folder. (5) `private_options` sets `0o600` only on newly created files. For an existing file, decide whether the staged copy keeps the original's permissions, because a rename swaps the inode. Do NOT stage in `/tmp` or `std::env::temp_dir()`, because the rename must stay on the same volume. Keep the staging file beside the target.
- **Slice C:** (1) "Copy MCP config" stays. Add the item, don't replace it, and bump the `ACTIONS_BY_ID` array length. (2) Do not put the handler body in `tray.rs`. `dispatch` calls `<module>::connect(app)` only. (3) Log errors with `tracing::error!(%error, ...)` and never log file contents (ARCHITECTURE §5 Logging). (4) Every failure branch ends in a user notification, as `copy_config` does. Do not fail silently.

## 5. Known dragons
- No `TODO`/`FIXME`/`LEGACY` markers in `src-tauri/src` or `store-markdown/src`.
- `private_file.rs` is `pub(crate)` and hard-codes `md.tmp`, so it cannot be reused across crates as-is. Promoting it to a shared crate is an architecture change for the reviewer, not this plan.
- `bundled_mcp_path` only succeeds after `apps/scribe-tray/scripts/stage-sidecar.sh` has run (CLAUDE.md). In tests, pass an explicit path and don't call it.
- Windows: rumoured MSIX/Store installs of Claude Desktop may virtualise `%APPDATA%` (`%LOCALAPPDATA%\Packages\...\LocalCache\Roaming\Claude`). This is unverified. The brief fixes `%APPDATA%\Claude`, so stick to it and flag it if the plan wants coverage.

## 6. Test conventions
- Rust integration tests live in `apps/scribe-tray/src-tauri/tests/<module>.rs`, import `scribe_tray::<pub mod>::...`, and are named as behaviour sentences in snake_case (`a_path_with_spaces_is_kept_intact`). There are no co-located `#[cfg(test)]` tests in the tray. Arrange/act/assert are separated by blank lines.
- File-system tests use `tempfile::tempdir()` with an injected path. Unix-only permission checks are `#[cfg(unix)]` and assert `mode & 0o777` (see `crates/store-markdown/tests/transcripts_folder.rs`).
- Extend `tests/menu_action.rs` with `connect_claude_desktop_id_maps_to_...`. Add merge/write cases to `tests/mcp_config.rs` or a new `tests/claude_desktop.rs`.
- Run: `apps/scribe-tray/scripts/stage-sidecar.sh` once, then `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all`. CI runs `cargo mutants --in-diff` on PR diffs, so every branch (missing folder, missing file, invalid JSON, update existing `scribe`) needs an asserting test.
- Do not run cargo inside `/private/tmp`.

## 7. Hot paths and performance landmines
- None. The action is a one-shot user click. Do the file I/O in the menu callback just as `copy_config` does, and don't block on anything long-running.

## 8. Open uncertainties resolved during orientation
- **"Is serde_json available?"** Yes. It's a direct tray dep (workspace `"1"`, default features, so no `preserve_order`). Adding `features = ["preserve_order"]` to the tray dep would keep the user's key order. That change is open to the plan, and it also affects `config_snippet` output order (single key, so no visible change).
- **"Which dirs call covers both OS paths?"** `dirs::config_dir()`. The tray needs the `dirs` dep added.
- **"Notification mechanism?"** Plain `show_plain` (tauri-plugin-notification) on both OSes. `mac-notification-sys` and `tauri-winrt-notification` are only for button notifications.
- **"Is there an existing backup convention?"** No, so the plan must name one.

---
## Changelog
- 2026-10-18 — generated by code-orienteer from plan 002 (no spec) at HEAD b5d1e32.
