# Plan: 002-claude-desktop-connect

> Spec: `none — planned from prompt` (standard tier; builds on [spec 001 §5.7](../specs/001-scribe-tray-transcriber.md#57-read-saved-transcripts-from-any-ai-app-over-mcp)) · Context: `docs/specs/002-claude-desktop-connect-context.md` · Authored: 2026-10-18
> Risk: MODERATE (writes into another app's user config file) · Hard floors: none · Fabric standards: reachable (Clean Code, Incubyte Engineering Standards)

---

# Part 1 — Review this

*Everything a reviewer needs to approve or redirect the approach. If you only read one part, read this one.*

## What we're doing

The tray menu gets a **"Connect to Claude Desktop"** item. It adds Scribe's MCP server to Claude Desktop's config file, so the user doesn't have to paste JSON by hand. It works on macOS (`~/Library/Application Support/Claude/claude_desktop_config.json`) and Windows (`%APPDATA%\Claude\claude_desktop_config.json`).

The constraint that shapes the design: **this file belongs to another app and may hold the user's other MCP servers and settings.** We may only ever add or update our own `mcpServers.scribe` entry. Every other key and server is kept in its original order. A file we can't parse is never overwritten. The original is backed up and the new version replaces it in one atomic step.

## How it breaks down

This is one slice. A pure merge function decides what the new file content is. A small writer applies it to disk safely. The tray item wires the two together and reports the outcome in a notification.

| # | Slice | Delivers | Why this tier | Tier |
|---|-------|----------|---------------|------|
| 1 | Connect Scribe to Claude Desktop from the tray | Menu item, config merge, safe write, four outcome notifications | Small, but a bad write breaks another app's setup | `excellent` |

"Copy MCP config" stays as it is, for other MCP clients such as Cursor.

## Decisions made

**1. Missing Claude folder means "not installed"; missing file means "create it".** If the `Claude` folder is absent, Claude Desktop isn't installed, so we tell the user to install it. If the folder exists but has no `claude_desktop_config.json` (a fresh install whose MCP settings were never opened), we create the file with only our entry. *Rejected: treating a missing file as "not installed". It would turn away people who have Claude Desktop installed.*

**2. We own exactly one key: `mcpServers.scribe`.** If it exists (for example, it points at an older build path), we update it. Other servers and top-level keys stay byte-for-byte in value and keep their order. We read and write with `serde_json`'s `preserve_order` feature. *Rejected: rebuilding the file from a typed struct, which would drop keys we don't know about.*

**3. A file we can't parse is refused, never repaired.** That covers invalid JSON, a root that isn't an object, and an `mcpServers` that isn't an object. The user sees "Claude Desktop's config has errors — fix it, or use Copy MCP config". *Rejected: backing up and replacing, which would silently wipe the user's other servers.*

**4. Back up the original, then replace it atomically; skip both when nothing changes.**
- When the merged content differs from the current file: copy the current file to `claude_desktop_config.json.scribe-backup` (one backup, replaced each time), write `claude_desktop_config.json.scribe-tmp` in the same folder, flush it, then rename it over the original.
- When the entry is already correct: write nothing and say "already connected".

**5. The outcome is a closed set, and each value maps to one notification.** The outcomes are `Connected`, `AlreadyConnected`, `ClaudeNotInstalled`, `ConfigUnreadable` and `Failed`. Success says "Restart Claude Desktop to use Scribe".

## Where this could go wrong

- **Losing the user's other MCP servers.** Guarded by the quality checks that other keys are preserved with their order, and that unparseable files are refused.
- **A half-written file if Scribe crashes mid-write.** Guarded by the temp-file-and-rename check, and the check that no temp file is left behind.
- **Microsoft Store builds of Claude Desktop on Windows** may keep the config in a virtualised AppData folder (`%LOCALAPPDATA%\Packages\…\LocalCache\Roaming\Claude`) `[inferred]`. If so, we would report "not installed" for that install. It is out of scope; the copy-and-paste fallback still works.
- **Claude Desktop running while we write.** It reads the config at startup, so we tell the user to restart. We don't try to restart it ourselves.

## What's out of scope

Other MCP clients (Cursor, VS Code, Claude Code CLI), an automatic restart of Claude Desktop, a "Disconnect" item, Microsoft Store virtualised paths, and Linux.

---

# Part 2 — Build detail

*Prescriptive detail for the build agent. Reviewers can skim or skip.*

**Confidence markers used below:** `[verified]` = the planner read the code or docs and confirmed it. `[inferred]` = pattern-matched from a sibling implementation, not confirmed — the build agent should check before assuming.

## Shared conventions (all slices)

- **Architecture:** Hexagonal, per [`docs/ARCHITECTURE.md`](../ARCHITECTURE.md). This is a primary-adapter concern: integrating with an external app from the tray. It lives entirely in `apps/scribe-tray/src-tauri`, and `scribe-core` is untouched.
- **Dependency direction:** `tray.rs` / menu dispatch → `claude_desktop` module → `mcp_config::bundled_mcp_path` and `notifications`. Inside the module, the pure merge has no I/O, and only the writer touches the filesystem.
- **Lexicon** (from [context](../specs/002-claude-desktop-connect-context.md); existing names reused verbatim): `bundled_mcp_path`, `config_snippet`, `McpConfigError`, `MenuAction`, `ACTIONS_BY_ID`, `notifications::show_plain`. The new names are `MENU_CONNECT_CLAUDE_DESKTOP`, `MenuAction::ConnectClaudeDesktop`, `ClaudeDesktopConfig`, `ConnectOutcome`, `merge_scribe_server`, `MergeResult`, `UnreadableConfig`.
- **Tests:** Rust behaviour tests in `apps/scribe-tray/src-tauri/tests/` against the lib's public API, using temp directories and no real home folder. Names are in business language. Workspace gates are fmt, clippy pedantic with `-D warnings` and the limits (20 lines, 3 arguments, complexity 5), and `cargo test --workspace`. Run `bash apps/scribe-tray/scripts/stage-sidecar.sh` before building.

---

## Slice 1 — Connect Scribe to Claude Desktop from the tray

**Key decision:** A pure `merge_scribe_server` turns the current file content (or none) into one of: new content, unchanged, or unreadable (Decisions 2, 3). `ClaudeDesktopConfig` finds the file and applies the result with backup and an atomic replace (Decisions 1, 4). A new menu action runs both and shows one notification per `ConnectOutcome` (Decision 5).

**Structure**
- `Cargo.toml` (workspace, modify): `serde_json = { version = "1", features = ["preserve_order"] }` `[verified: feature exists in serde_json 1.0.154; without it keys are re-sorted alphabetically (context)]`.
- `apps/scribe-tray/src-tauri/Cargo.toml` (modify): add `dirs.workspace = true` to `[dependencies]`, and a `[dev-dependencies]` block with `tempfile` (pin the version already in `Cargo.lock`) `[verified: context — neither present today]`.
- Context mapping: this one slice covers context slices A (merge), B (write) and C (menu). Use the context's imitation targets and traps for each part.
- `apps/scribe-tray/src-tauri/src/claude_desktop/mod.rs` (new):
  - `pub use merge::{merge_scribe_server, MergeResult, UnreadableConfig};`
  - `pub use location::ClaudeDesktopConfig;`
  - `pub enum ConnectOutcome { Connected, AlreadyConnected, ClaudeNotInstalled, ConfigUnreadable, Failed }`
  - `pub fn connect(config: &ClaudeDesktopConfig, server_command: &Path) -> ConnectOutcome`
- `apps/scribe-tray/src-tauri/src/claude_desktop/merge.rs` (new, pure):
  - `pub enum MergeResult { Unchanged, Write(String) }`
  - `#[derive(Debug, thiserror::Error)] pub enum UnreadableConfig { #[error("not valid JSON")] InvalidJson, #[error("top level is not an object")] RootNotObject, #[error("mcpServers is not an object")] ServersNotObject }`
  - `pub fn merge_scribe_server(existing: Option<&str>, server_command: &Path) -> Result<MergeResult, UnreadableConfig>`
  - Private helpers: `fn parse_root(existing: &str) -> Result<Map<String, Value>, UnreadableConfig>`, `fn scribe_entry(server_command: &Path) -> Value` (reuses the same `{"command": …}` shape as `config_snippet`; extract a shared `scribe_server_entry(path) -> Value` in `mcp_config.rs` and have `config_snippet` use it, for DRY), `fn render(root: &Map<String, Value>) -> String` (`serde_json::to_string_pretty` plus a trailing newline).
  - Constants: `SERVERS_KEY = "mcpServers"`, `SCRIBE_SERVER_NAME = "scribe"`.
- `apps/scribe-tray/src-tauri/src/claude_desktop/location.rs` (new):
  - `pub struct ClaudeDesktopConfig { folder: PathBuf }`
  - `impl ClaudeDesktopConfig { pub fn default_location() -> Option<ClaudeDesktopConfig>; pub fn in_folder(folder: PathBuf) -> ClaudeDesktopConfig; pub fn is_installed(&self) -> bool; pub fn read(&self) -> io::Result<Option<String>>; pub fn replace(&self, contents: &str) -> io::Result<()>; }`
  - `default_location` is `dirs::config_dir()?.join("Claude")` `[verified: dirs 6 config_dir = ~/Library/Application Support on macOS, {FOLDERID_RoamingAppData} = %APPDATA% on Windows]`.
  - `is_installed` is true when the folder is a directory.
  - `read` maps `NotFound` to `Ok(None)`.
  - Constants: `CONFIG_FILE = "claude_desktop_config.json"`, `BACKUP_FILE = "claude_desktop_config.json.scribe-backup"`, `STAGING_FILE = "claude_desktop_config.json.scribe-tmp"`.
- `apps/scribe-tray/src-tauri/src/claude_desktop_connect.rs` (new; tray glue, mirrors `mcp_clipboard.rs`): `pub fn connect_claude_desktop<R: Runtime>(app: &AppHandle<R>)`.
- `menu_action.rs` (modify): add `MENU_CONNECT_CLAUDE_DESKTOP = "connect-claude-desktop"`, `MenuAction::ConnectClaudeDesktop`, and the `ACTIONS_BY_ID` entry (the array grows to 6).
- `tray.rs` (modify): a menu item labelled **"Connect to Claude Desktop"**, placed directly after "Copy MCP config", and dispatch to `claude_desktop_connect::connect_claude_desktop`.
- `notifications.rs` (modify): `pub fn notify_claude_desktop(app, outcome: ConnectOutcome)` is one dispatch over the outcome to `show_plain` with title and body constants:

  | Outcome | Title | Body |
  |---|---|---|
  | `Connected` | "Scribe connected to Claude Desktop" | "Restart Claude Desktop to use your Scribe transcripts." |
  | `AlreadyConnected` | "Scribe is already connected" | "Claude Desktop is already set up to use Scribe." |
  | `ClaudeNotInstalled` | "Claude Desktop not found" | "Install Claude Desktop, open it once, then try again." |
  | `ConfigUnreadable` | "Couldn't update Claude Desktop" | "Its config file has errors. Fix it, or use Copy MCP config." |
  | `Failed` | "Couldn't connect to Claude Desktop" | "Something went wrong. Use Copy MCP config instead." |

**Collaborators:** `connect_claude_desktop` calls `bundled_mcp_path()`; if that fails, the outcome is `Failed`. It then calls `ClaudeDesktopConfig::default_location()` (`None` means `ClaudeNotInstalled`), then `claude_desktop::connect`, then `notifications::notify_claude_desktop`. `connect` uses `merge_scribe_server` and the `ClaudeDesktopConfig` I/O. `merge.rs` imports nothing from Tauri or the filesystem.

**Control flow:**
- `connect(config, command)`:
  1. If `!config.is_installed()` → `ClaudeNotInstalled`.
  2. `config.read()`; on an I/O error → `Failed`, logging the error kind and file name only.
  3. `merge_scribe_server(existing, command)`. `Err(UnreadableConfig)` → `ConfigUnreadable`, logged with the variant. `Unchanged` → `AlreadyConnected`. `Write(content)` → `config.replace(content)`, which gives `Connected` on `Ok` and `Failed` on `Err`.
- `merge_scribe_server(existing, command)`:
  1. If `None`, the root is an empty map.
  2. Otherwise `parse_root`. Guards: invalid JSON → `InvalidJson`; a root that isn't an object → `RootNotObject`.
  3. Get or insert `mcpServers` as an empty object. If it isn't an object → `ServersNotObject`.
  4. If `servers["scribe"]` already equals the new entry and `existing` was `Some` → `Unchanged`.
  5. Otherwise insert or replace `servers["scribe"]` → `Write(render(root))`.
- `replace(contents)`:
  1. If the config file exists, copy it to `BACKUP_FILE`, overwriting any previous backup.
  2. Write `STAGING_FILE` (create and truncate), `write_all`, then `sync_data`. When the original exists, copy its permissions onto the staging file (`fs::set_permissions` with the original's `metadata().permissions()`). A new file gets `0o600` on unix, mirroring `private_file.rs`. Copy that shape; don't import it, because it is crate-private and hard-codes a `md.tmp` staging name.
  3. `fs::rename(staging, config)`.
  4. On any error after staging was created, remove the staging file and return the error.

**Tests:**
- New `apps/scribe-tray/src-tauri/tests/claude_desktop_merge.rs`. Pure unit tests; cases chosen with ZOMBIES:
  - a fresh config contains only Scribe's server;
  - Scribe is added next to the user's other servers without changing them;
  - other top-level settings keep their values and order;
  - an old Scribe path is replaced with the current one;
  - a config already pointing at Scribe needs no change;
  - an empty `mcpServers` object gets Scribe added;
  - invalid JSON is refused;
  - a config whose top level is a list is refused;
  - an `mcpServers` that isn't an object is refused;
  - a path with spaces and backslashes is stored exactly.
- New `apps/scribe-tray/src-tauri/tests/claude_desktop_connect.rs`. Integration tests with a temp directory via `ClaudeDesktopConfig::in_folder`:
  - a missing Claude folder reports Claude Desktop is not installed and creates nothing;
  - a Claude folder without a config gets a new config containing Scribe;
  - connecting backs up the previous config first;
  - no staging file is left behind after connecting;
  - connecting twice leaves the file untouched and makes no backup the second time;
  - an unreadable config is left exactly as it was.
- Extend `apps/scribe-tray/src-tauri/tests/menu_action.rs`: the "connect-claude-desktop" id maps to `ConnectClaudeDesktop`.
- Extend `apps/scribe-tray/src-tauri/tests/mcp_config.rs`: the copied snippet still matches the shared server entry (guards the DRY extraction).

**Standards that bite here:**
- Clean Code: SRP (merge vs location/I/O vs tray glue), no flag arguments, guard clauses, functions ≤20 lines, ≤3 arguments, named constants, no comments.
- Fail fast at the boundaries: parse and shape guards.
- `confidential-information-logging`: log kinds and file names, never file contents.
- Incubyte: business-language test names, ZOMBIES, parameterised tests used sparingly (≤6 rows), domain grouping (the `claude_desktop/` folder).

**Quality checks**
- **Every key and server already in the config survives a connect with the same value and position; only `mcpServers.scribe` changes.** (Decision 2)
- An unparseable or wrongly shaped config file is byte-identical after a connect attempt. (Decision 3)
- The config file is only ever changed by renaming a fully written, synced staging file in the same folder; no staging file remains after success or failure. (Decision 4)
- When `mcpServers.scribe` is already correct, no write and no backup happen. (Decision 4)
- `merge.rs` has no filesystem, Tauri or logging dependency.
- The `{"command": …}` server entry is built in exactly one place, shared by Copy MCP config and Connect. (DRY)
- Each `ConnectOutcome` maps to exactly one notification, and no notification or log contains file contents. (Decision 5)

---

## Hard floors

None. The slice writes a non-secret config file belonging to another app. It touches no auth, authorization, payment, crypto/secrets or migrations. Its safety is guarded by the quality checks above.

## Escalation log

*(empty)*
