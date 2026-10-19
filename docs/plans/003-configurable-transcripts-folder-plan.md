# Plan: 003-configurable-transcripts-folder

> Spec: none — planned from prompt (standard tier) · Context: [`docs/specs/003-configurable-transcripts-folder-context.md`](../specs/003-configurable-transcripts-folder-context.md) · Authored: 2026-10-19
> Risk: MODERATE (changes where user transcripts are written and can move existing files; no regulated material) · Hard floors: slice 3 · Fabric standards: reachable

---

# Part 1 — Review this

*Everything a reviewer needs to approve or redirect the approach. If you only read one part, read this one.*

## What we're doing

Two user-visible changes. First, new transcripts go into a month folder named `YYYY-MM` (for example `Scribe/2026-10/2026-10-19-114521.md`), so the transcripts folder stays browsable as it grows. Second, the user can pick a different transcripts folder from a new **Settings** view in the popover. That view has one button, **Change Transcripts Folder**. Picking a folder takes effect for the next recording. Scribe then offers to move existing transcripts into the new folder.

Scribe has no database. The chosen folder is stored under a new key, `transcripts_folder`, in the `settings.json` the tray already keeps through `tauri-plugin-store`, beside `privacy_acknowledged`. The constraint behind most of the design is that **`scribe-mcp` is a separate process**. Claude Desktop starts it and keeps it running, and it has no access to Tauri. So the rule that turns `settings.json` into a folder must live in `store-markdown`, the one crate both apps may use (ARCHITECTURE §3). The MCP server must also re-read that rule on every tool call. Otherwise it would keep reading the old folder after the user changes it.

Existing flat files (`Scribe/<id>.md`) are never migrated automatically. They stay readable in place. The only time files move is when the user explicitly accepts the "move existing transcripts" prompt after changing folders. That move is the one deliberate file-moving operation in this plan, and it is guarded as a data move (see Hard floors).

## How it breaks down

Storage first, then wiring, then UI. Slice 1 teaches the store the month layout. Slice 2 adds the shared "which folder?" resolver, makes the store's folder swappable, and points all three callers at it. Slice 3 adds the guarded move. Slice 4 is the tray command that ties picker, persistence, swap and move together. Slice 5 is the popover Settings view. Slice 6 updates the text that still says "Documents/Scribe".

| # | Slice | Delivers | Why this tier | Tier |
|---|-------|----------|---------------|------|
| 1 | Store transcripts in month folders | New files land in `YYYY-MM/`; legacy flat files still list, read, open and get marked abandoned | Touches every store path and several existing tests; collision-suffix and permission behaviour must survive | `excellent` |
| 2 | Resolve the configured folder | One resolver reads `settings.json` with fallback to `Documents/Scribe`; the store's folder can be swapped at runtime; tray and MCP use the resolver; MCP follows changes per call | Cross-process contract; every fallback branch needs a test for mutation testing | `excellent` |
| 3 | Move existing transcripts | `move_transcripts` moves transcript files safely, never deleting a source before its copy is complete | Moves user data; partial failures must never lose a file | `excellent` (floored) |
| 4 | Change the folder from the tray backend | `change_transcripts_folder` command: native picker, recording guard, persist, swap, move prompt, notification | Thread and dialog pitfalls (main-thread deadlock, blur-hide, lock ordering) | `excellent` |
| 5 | Add the Settings view to the popover | Gear button opens Settings with the current folder and a **Change Transcripts Folder** button, disabled while recording | Small UI with accessibility (focus moves, labels) | `excellent` |
| 6 | Replace "Documents/Scribe" text | Privacy statement, start-failure advice and ARCHITECTURE storage row stop naming a fixed folder | Text and assertion edits only | `fast` |

Slices 3 and 4 are split on purpose. Slice 3 is pure filesystem logic with integration tests. Slice 4 is the thin Tauri shell around it, untested by convention ([context §6](../specs/003-configurable-transcripts-folder-context.md#6-test-conventions)), so it must stay thin.

## Decisions made

**1. Month folders use `YYYY-MM`, taken from the `SessionId`, not from the clock.** The id's first seven characters are already `YYYY-MM` by construction (`STAMP_FORMAT`). Deriving the month from the id means every lookup computes the same folder with no stored state, and a call that runs past midnight at month end stays with its start month. *Rejected: `mmm-yy` (`Oct-26`), because it does not sort by date in Finder or Explorer; the user confirmed `2026-10`.*

**2. Legacy flat files are read in place and never migrated automatically.** `path_for(id)` resolves to the month file if it exists, else the legacy flat file if it exists, else the month path (so `NotFound` still maps correctly). Listing reads the root plus one level of folders named exactly `YYYY-MM`. *Rejected: a one-time migration at launch, because it moves user files without being asked.*

**3. The folder-resolution rule lives in `store-markdown`, with one injectable form.** `TranscriptsFolder::configured_in(settings_path)` reads the `transcripts_folder` key. If the file is missing, unparseable, lacks the key, holds a non-string, holds an empty string or holds a relative path, it falls back to `Documents/Scribe`. It never fails because of a bad setting. The tray passes Tauri's own `app_data_dir()/settings.json`, so it reads exactly where `tauri-plugin-store` writes. The MCP server passes `dirs::data_dir()/com.incubyte.scribe/settings.json`. *Rejected: passing the folder as an argument in the Claude Desktop MCP config, because it goes stale after every change and fails for hand-written configs.*

**4. The MCP server re-reads the folder on every tool call.** A small decorator in `scribe-mcp` (`SettingsFollowingStore`) relocates the inner store from `settings.json` before each read. Reading a tiny JSON file per user-initiated tool call costs nothing. *Rejected: resolving once at startup, because Claude Desktop keeps the server alive and would show the old folder, which is empty after a move.*

**5. The store's folder is swapped in place, and only while not recording.** `MarkdownTranscriptStore` holds its folder behind a `RwLock` and gains `relocate(folder)` and `folder()`. Its `Arc` is shared by `Recorder`, `commands.rs` and `tray.rs`, and Tauri cannot replace managed state, so swapping inside the store is the only seam that updates everyone at once. `RecordingController::relocate_store` holds the recorder lock while it checks for `Recording` and swaps. A recording therefore can never start halfway through a swap, and an active session never has its folder changed under it. *Rejected: a per-session folder pin, because the user chose "block the change while recording", which makes it unnecessary.*

**6. The whole change flow runs in Rust as one async command, and the popover only invokes it.** In an async command, `blocking_pick_folder` and `MessageDialogBuilder::blocking_show` run off the main thread, which the docs require [verified]. The popover hides when it loses focus to the native picker, so the result also arrives as a system notification. *Rejected: a picker in the webview through the JS dialog plugin, because the move prompt, persistence and swap would then round-trip through the webview and need `dialog:*` capabilities.*

**7. Moving existing transcripts is offered, never assumed, and never destructive on failure.** After a successful change, Scribe counts the transcripts in the old folder. Only files whose name parses as a `SessionId` count, so the user's other `.md` files are never touched. If there are any, a native prompt asks "Move N transcripts to the new folder?" with **Move** and **Keep in old folder** buttons. Each file is moved into its month folder with `fs::rename`. If the destination is on another volume, the file is copied to a temporary sibling, renamed into place, and only then is the source removed. A file whose destination already exists is kept where it is. *Rejected: moving silently, and copying without removing; the user asked for a move.*

**8. The settings key and file name are shared constants owned by `store-markdown`.** `SETTINGS_FILE_NAME` and `TRANSCRIPTS_FOLDER_KEY` are exported from there. `privacy.rs` stops keeping its own `"settings.json"` literal. *Rejected: a second literal in the tray, which is how the two processes would drift apart.*

## Where this could go wrong

- **A move that loses or truncates a transcript.** It could come from a cross-volume copy, a crash mid-move or a name collision. Guarded by slice 3's checks: the source is removed only after the destination has been renamed into place, and any collision or failure leaves the source in place.
- **The tray and MCP reading different `settings.json` paths on Windows.** The `dirs::data_dir()` = Tauri `app_data_dir()` parent equivalence is [inferred] on Windows (it was confirmed on macOS by the file on disk). Guarded by slice 2's manual Windows check.
- **The native picker deadlocking or opening behind other windows.** It deadlocks if called on the main thread. It can also open behind other windows because the app is an accessory app ([context §5](../specs/003-configurable-transcripts-folder-context.md#5-known-dragons)). Guarded by slice 4: the command must be `async`, and the result is checked by hand.
- **Existing tests weakened to fit the new layout.** `mcp_tools.rs` helpers and the `0o600` permission test assume flat files. Guarded by slice 1's check that each is updated to the month layout with the same assertions.
- **Noisy logs when the user picks a broad folder such as `~/Documents`.** Every unrelated `.md` warns on each listing. This is accepted and out of scope; slice 1 only guarantees the scan never recurses beyond one `YYYY-MM` level.

## What's out of scope

- No cloud-sync warning (the user's decision).
- No automatic migration of flat files into month folders.
- No cleanup of empty month folders left behind after a move.
- No further settings beyond the one button.
- No change to the `opener:allow-open-path` capability scope. It is stale but harmless, because every opener call happens in Rust. Do not widen it ([context §5](../specs/003-configurable-transcripts-folder-context.md#5-known-dragons)).
- No quieting of `summary_or_skip` warnings for unrelated `.md` files.

---

# Part 2 — Build detail

*Prescriptive detail for the build agent. Reviewers can skim or skip.*

**Confidence markers used below:** `[verified]` = the planner read the code or docs and confirmed it. `[inferred]` = pattern-matched from a sibling implementation, not confirmed — the build agent should check before assuming.

**Supersedes the context file where they differ:** the user chose a **popover Settings view**, not a tray menu item. Ignore the context file's slice-C lexicon for `MENU_CHOOSE_TRANSCRIPTS_FOLDER`; no menu item is added. The user also chose **offer to move** old transcripts, which replaces the context's "old transcripts stay where they are". Slice letters A–D in the context map to plan slices as A→2, B→1, C→3+4+5, D→6.

## Shared conventions (all slices)

- **Architecture:** Hexagonal ([`docs/ARCHITECTURE.md`](../ARCHITECTURE.md)). `scribe-core` stays filesystem-free. All filesystem layout, settings reading and moving lives in the `store-markdown` adapter. Apps are composition roots and thin shells.
- **Dependency direction:** `apps/scribe-tray` → `store-markdown` → `scribe-core`; `apps/scribe-mcp` → `store-markdown` + `scribe-core` only. No `tauri*` crate in `store-markdown` or `scribe-mcp` [verified].
- **Module map:**
  - `crates/scribe-core/src/recording/session.rs` · `SessionId` · nothing new [verified]
  - `crates/store-markdown/src/transcripts_folder.rs` · `TranscriptsFolder` + configured resolution · `dirs`, new `settings_file` [verified]
  - `crates/store-markdown/src/settings_file.rs` (new) · read the folder setting from `settings.json` · `serde_json`, `dirs`
  - `crates/store-markdown/src/month_layout.rs` (new) · where a session's file lives; which files are transcripts · `scribe-core` `SessionId`
  - `crates/store-markdown/src/writer.rs` · `MarkdownTranscriptStore` · `month_layout` [verified]
  - `crates/store-markdown/src/relocation.rs` (new) · `move_transcripts` · `month_layout`, `private_file`
  - `apps/scribe-mcp/src/main.rs`, `apps/scribe-mcp/src/following_store.rs` (new) · MCP composition and per-call folder following [verified main.rs]
  - `apps/scribe-tray/src-tauri/src/{recording,commands,tray,privacy,notifications,lib}.rs`, `transcripts_location.rs` (new) · tray wiring [verified]
  - `apps/scribe-tray/src/popover/{Popover,PopoverActions,SettingsPanel(new),commands,guards,types}.tsx?` · Settings UI [verified existing files]
- **Lexicon (from [context §4b](../specs/003-configurable-transcripts-folder-context.md#4b-slice-lexicon), do not rename):** `TranscriptsFolder`, `MarkdownTranscriptStore`, `SessionId`, `path_for`, `transcript_paths`, `is_transcript`, `TRANSCRIPT_EXTENSION`, `private_dir_builder`, `StartFailureKind::TranscriptsFolder`. New shared names: `SETTINGS_FILE_NAME = "settings.json"`, `TRANSCRIPTS_FOLDER_KEY = "transcripts_folder"`, `APP_IDENTIFIER = "com.incubyte.scribe"`, `SessionId::month`, `relocate`, `move_transcripts`, `MoveReport`, `FolderChange`, `SettingsPanel`.
- **Tests:** Rust integration tests in `<crate>/tests/<topic>.rs`, named as snake_case behaviour sentences, using `tempfile::tempdir()` with injected paths ([context §6](../specs/003-configurable-transcripts-folder-context.md#6-test-conventions)). Never read the developer's real `settings.json`. Frontend: co-located `*.spec.tsx` with testing-library, `mockIPC` and `vitest-axe`. Mutation testing runs on the diff in CI, so every branch below has an asserting test. Run order: `apps/scribe-tray/scripts/stage-sidecar.sh` → `cargo fmt --all` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo test --workspace` → `npm run lint && npm test` in `apps/scribe-tray`.
- **Logging:** never log a folder path (it can carry the user's name) or transcript text. Log counts, outcomes and file names only (ARCHITECTURE §5).
- **Commits:** Conventional Commits, lowercase imperative, under 72 characters. One commit per slice.

---

## Slice 1 — Store transcripts in month folders

**Key decision:** New transcripts are created in `<folder>/<YYYY-MM>/<id>.md`, with the month taken from the id (decision 1). Every lookup goes through one layout module, so `create`, `append`, `finalise`, `read`, listing, abandoned-marking and `open_session` all agree. Legacy flat files keep working in place (decision 2).

**Structure**

- `crates/scribe-core/src/recording/session.rs` (modify) — `impl SessionId { pub fn month(&self) -> &str }`: the first seven characters. They are always `YYYY-MM`, because `parse` and `from_start` guarantee a 4-2-2-6 stamp [verified].
- `crates/store-markdown/src/month_layout.rs` (new, crate-private):
  - `pub(crate) fn month_path(folder: &Path, id: &SessionId) -> PathBuf`
  - `pub(crate) fn legacy_path(folder: &Path, id: &SessionId) -> PathBuf`
  - `pub(crate) fn existing_path(folder: &Path, id: &SessionId) -> PathBuf` returns the month path if it is a file, else the legacy path if it is a file, else the month path.
  - `pub(crate) fn transcript_paths_in(folder: &Path) -> Result<Vec<PathBuf>, StoreError>` covers root transcript files plus transcript files inside direct children whose name `is_month_folder`. A missing root is an empty `Vec`. An unreadable month folder is skipped with a warn naming only the folder's file name.
  - `pub(crate) fn session_files_in(folder: &Path) -> Result<Vec<(SessionId, PathBuf)>, StoreError>` covers the subset of `transcript_paths_in` whose file stem parses as a `SessionId` (used by slice 3).
  - `fn is_month_folder(name: &str) -> bool` means exactly 7 chars: 4 ASCII digits, `-`, 2 ASCII digits.
  - Move `TRANSCRIPT_EXTENSION` and `is_transcript` here from `writer.rs`.
- `crates/store-markdown/src/transcripts_folder.rs` (modify) — `pub(crate) fn ensure_month_exists(&self, id: &SessionId) -> Result<PathBuf, StoreError>` creates `<folder>/<month>` via `private_dir_builder` (0o700) and maps errors to `StoreError::CreateFolder`.
- `crates/store-markdown/src/writer.rs` (modify) — `pub fn path_for(&self, id: &SessionId) -> PathBuf` now returns `month_layout::existing_path`. `try_create` writes to `month_layout::month_path`. `transcript_paths` delegates to `month_layout::transcript_paths_in`. `create` calls `ensure_month_exists` for the header's id month before the collision loop. Collision candidates share the base id's month.
- `apps/scribe-tray/src-tauri/src/commands.rs` (no code change expected) — `open_session` already uses `path_for` + `is_file`, which now resolves month or legacy [verified].

**Collaborators:** The writer calls `month_layout` and `TranscriptsFolder`. `month_layout` depends only on `scribe-core` `SessionId` and std. `reader.rs` is unchanged, because it takes whatever paths it is given [verified].

**Control flow:**
- `create`: ensure the root exists → ensure the month folder exists → for each collision candidate, `create_new` at `month_path` → on AlreadyExists, try the next → exhausted gives `NoFreeSessionId`.
- `append` / `finalise` / `read`: `path_for(id)` → existing behaviour (NotFound mapping unchanged).
- `transcript_paths_in`: read root → NotFound gives empty → for each entry, a transcript file is kept, and a directory named as a month folder has its transcript files kept; anything else is ignored → never descend further.

**Tests:**
- Extend `crates/scribe-core/tests/session_id.rs`: "a session id knows the month it started in" (`2026-10-19-114521` gives `2026-10`), and "a collision-suffixed id keeps its month".
- New `crates/store-markdown/tests/month_folders.rs`:
  - a new transcript is saved inside its month folder
  - a second call started in the same second gets its own file in the same month folder
  - a month folder is private to the user (`#[cfg(unix)]`, `0o700`)
  - a transcript saved before month folders is still listed and read
  - a transcript saved before month folders is still marked incomplete on launch
  - transcripts across two months are listed newest first
  - folders that are not months are not searched for transcripts (`notes/`, `2026-1`, `2026-10-01`)
  - `path_for` finds a legacy transcript
- Update existing tests that assume the flat layout (`transcript_file.rs` permission test `0o600`, `transcript_text` / `file_names_in` helpers, `apps/scribe-mcp/tests/mcp_tools.rs` `folder_contents`) to the month layout, keeping every assertion's strength.

**Standards that bite here:** Clean Code: small functions (<20 lines), complexity ≤5, nesting ≤3, intent-revealing names, DRY (one layout module); Incubyte: test names in business language, ZOMBIES (zero / one / many months, boundary folder names); `code-craft`: no comments, no `null`-style sentinels.

**Quality checks**

- **Every path a store method touches comes from `month_layout`; no `folder.join(format!("{id}.md"))` remains in `writer.rs`.**
- Listing never recurses beyond one level and only enters folders matching `YYYY-MM` exactly.
- The collision-suffix behaviour (`-2..-99`) is unchanged and covered in a month folder.
- Month folders are created through `private_dir_builder`; `fs::create_dir_all` is not used.
- Legacy flat transcripts are never moved or rewritten by this slice, except by the existing abandoned-marking header rewrite.
- `SessionId::month` is the only place the month is derived; no string slicing of ids elsewhere.
- No existing assertion is loosened. The updated tests assert the same properties at the new path.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.

---

## Slice 2 — Resolve the configured folder

**Key decision:** Add the single "which folder?" rule (decision 3) and the shared constants (decision 8). Point all three callers at it: tray `install`, tray "Open transcripts folder", and MCP `main`. Make the MCP server re-read the setting on every call (decision 4).

**Structure**

- `crates/store-markdown/Cargo.toml` (modify) — add `serde_json.workspace = true` [verified missing].
- `crates/store-markdown/src/settings_file.rs` (new):
  - `pub const SETTINGS_FILE_NAME: &str = "settings.json";`
  - `pub const TRANSCRIPTS_FOLDER_KEY: &str = "transcripts_folder";`
  - `const APP_IDENTIFIER: &str = "com.incubyte.scribe";` (matches `tauri.conf.json` `identifier` [verified])
  - `pub fn default_settings_path() -> Option<PathBuf>` returns `dirs::data_dir()?/APP_IDENTIFIER/SETTINGS_FILE_NAME`. This uses `data_dir`, not `config_dir` ([context §4c](../specs/003-configurable-transcripts-folder-context.md#4c-slice-traps)).
  - `pub(crate) fn configured_folder_path(settings_path: &Path) -> Option<PathBuf>` reads the file, parses it to `serde_json::Value`, and takes the key as a str. It returns `Some` only for a non-empty absolute path; every failure gives `None`.
- `crates/store-markdown/src/transcripts_folder.rs` (modify) — derive `Debug, Clone, PartialEq, Eq` on `TranscriptsFolder`. Add:
  - `pub fn configured_in(settings_path: &Path) -> Result<TranscriptsFolder, StoreError>` returns the configured path, or `default_location()`.
  - `pub fn configured() -> Result<TranscriptsFolder, StoreError>` uses `default_settings_path()` if present, else `default_location()`.
- `crates/store-markdown/src/lib.rs` (modify) — re-export `SETTINGS_FILE_NAME`, `TRANSCRIPTS_FOLDER_KEY`, `default_settings_path`.
- `crates/store-markdown/src/writer.rs` (modify) — make the store relocatable (decision 5). The field becomes `folder: RwLock<TranscriptsFolder>`.
  - `pub fn relocate(&self, folder: TranscriptsFolder)`
  - `pub fn folder(&self) -> TranscriptsFolder` returns a clone of the current folder.
  - Every method takes one snapshot of the folder at entry and uses only that snapshot for the whole operation. Recover a poisoned lock with `PoisonError::into_inner`, following `recording.rs` [verified pattern].
- `apps/scribe-mcp/src/following_store.rs` (new) — `pub struct SettingsFollowingStore { settings_path: PathBuf, inner: MarkdownTranscriptStore }`:
  - `pub fn new(settings_path: PathBuf) -> Result<SettingsFollowingStore, StoreError>`
  - `impl TranscriptStore`: `list` / `list_all` / `read` first call `self.follow_settings()?`, then delegate. `create` / `append` / `finalise` / `mark_abandoned_incomplete` delegate directly.
  - `fn follow_settings(&self) -> Result<(), StoreError>` does `inner.relocate(TranscriptsFolder::configured_in(&self.settings_path)?)`.
- `apps/scribe-mcp/src/main.rs` (modify) — when `default_settings_path()` is `Some`, build `SettingsFollowingStore::new(path)`. When it is `None`, build `MarkdownTranscriptStore::new(TranscriptsFolder::default_location()?)`.
- `apps/scribe-tray/src-tauri/src/recording.rs` (modify) — `install` builds the store from `TranscriptsFolder::configured_in(&settings_path(app))`, where `fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf>` is `app.path().app_data_dir().ok().map(|dir| dir.join(SETTINGS_FILE_NAME))`. If it is `None`, use `default_location()`. Put `settings_path` in `privacy.rs`'s neighbour or a small `settings.rs`, as long as it is defined once.
- `apps/scribe-tray/src-tauri/src/privacy.rs` (modify) — replace the private `SETTINGS_FILE` literal with `store_markdown::SETTINGS_FILE_NAME`.
- `apps/scribe-tray/src-tauri/src/tray.rs` (modify) — `try_open_transcripts_folder` uses `app.state::<Arc<MarkdownTranscriptStore>>().folder()` instead of `default_location()`, then `ensure_exists` as today.

**Collaborators:** The MCP decorator wraps the store-markdown adapter behind the `scribe-core` `TranscriptStore` port. The tray reads Tauri's path resolver only to pick the settings path, and the rule itself stays in `store-markdown`.

**Control flow:**
- `configured_folder_path`: read the file (error gives none) → parse (error gives none) → key present and string (else none) → non-empty and `is_absolute` (else none) → `Some(PathBuf)`.
- MCP read op: resolve the folder → relocate → delegate. A resolver error (no Documents folder) surfaces as the store error, exactly as at startup today.

**Tests:**
- New `crates/store-markdown/tests/configured_folder.rs`, each case writing a `settings.json` in a tempdir:
  - no settings file uses the default folder
  - a chosen folder is used
  - an unreadable settings file uses the default
  - a settings file without a chosen folder uses the default
  - a chosen folder that is not text uses the default
  - an empty chosen folder uses the default
  - a relative chosen folder uses the default
  - other settings are ignored (file containing `privacy_acknowledged`)
- Extend `crates/store-markdown/tests/month_folders.rs`: "after relocating the store, new transcripts are saved in the new folder and the old folder's transcripts are not listed".
- Extend `apps/scribe-mcp/tests/mcp_tools.rs`: "the MCP server follows a folder change without restarting". Write settings pointing at folder A with one transcript and list it. Rewrite the settings to folder B with a different transcript and list again: only B's transcript appears. Add a `connect_following(settings_path)` helper beside `connect_to`.

**Standards that bite here:** Clean Code: fail fast at boundaries (validate the setting where it is read, trust it after), DRY (one rule, one constant set), searchable names (no literal `"settings.json"` outside `settings_file.rs`); ARCHITECTURE §3 dependency rule; Incubyte: ZOMBIES (zero file, one key, wrong type, boundary empty/relative).

**Quality checks**

- **No `tauri` or `tauri-plugin-*` dependency is added to `store-markdown` or `scribe-mcp`.**
- `"settings.json"`, `"transcripts_folder"` and `"com.incubyte.scribe"` appear as literals only in `settings_file.rs`.
- No code path in the tray or MCP calls `TranscriptsFolder::default_location()` directly except as the fallback inside `configured*` and the MCP `None` branch.
- Every fallback branch of `configured_folder_path` has its own asserting test.
- Tests inject the settings path; nothing reads the real app-data directory.
- MCP startup never fails because of a bad settings file.
- Store operations use one folder snapshot each; `append` does not read `settings.json` or scan folders ([context §7](../specs/003-configurable-transcripts-folder-context.md#7-hot-paths-and-performance-landmines)).

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.

**Open at build time:** whether `dirs::data_dir()/com.incubyte.scribe/settings.json` equals Tauri's `app_data_dir()/settings.json` on Windows. Docs indicate both are `%APPDATA%` (Roaming) [inferred]. This can only be confirmed on a Windows machine: after slice 4, change the folder in the tray on Windows and confirm `scribe-mcp` lists from the new folder.

---

## Slice 3 — Move existing transcripts

**Key decision:** Add `move_transcripts`, which moves only session files into the new folder's month layout and never deletes a source before its destination is complete (decision 7). The store became relocatable in slice 2. This is the floored slice.

**Structure**

- `crates/store-markdown/src/relocation.rs` (new):
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub struct MoveReport { pub moved: usize, pub kept: usize }`
  - `pub fn count_transcripts(folder: &TranscriptsFolder) -> Result<usize, StoreError>` is `session_files_in(...).len()`.
  - `pub fn move_transcripts(from: &TranscriptsFolder, to: &TranscriptsFolder) -> Result<MoveReport, StoreError>`
  - `fn move_one(source: &Path, destination: &Path) -> Result<MoveOutcome, io::Error>` with `enum MoveOutcome { Moved, Kept }`.
  - `fn copy_across_volumes(source: &Path, destination: &Path) -> io::Result<()>` copies to a `.part` sibling of the destination, renames it onto the destination, then removes the source.
- `crates/store-markdown/src/lib.rs` (modify) — export `move_transcripts`, `count_transcripts`, `MoveReport`.

**Collaborators:** `relocation` uses `month_layout::{session_files_in, month_path}` and `TranscriptsFolder::ensure_month_exists`. It knows nothing of Tauri or the recorder.

**Control flow:**
- `move_transcripts`: list `session_files_in(from)` → none gives the default report. For each (id, source): destination = `month_path(to, id)` → ensure the month folder exists → `move_one` → Moved increments `moved`; Kept or any per-file error increments `kept`, with a warn naming only the file name → continue → return the report.
- `move_one`: destination exists → Kept (the source is untouched). `fs::rename` Ok → Moved. A rename error whose kind is `io::ErrorKind::CrossesDevices` → `copy_across_volumes` → Moved. Any other error → Err.
- `copy_across_volumes`: `fs::copy(source, part)` (copies permissions on unix [verified std docs]) → `fs::rename(part, destination)` → `fs::remove_file(source)`. On any failure before the final step, remove the `.part` file and return the error, so the source stays put.

**Tests:** new `crates/store-markdown/tests/moving_transcripts.rs`:
- moving to a new folder places each transcript in its month folder
- legacy flat transcripts are moved into their month folder
- a transcript already present in the new folder is kept in the old folder (the destination's content is unchanged and the source still exists)
- files that are not transcripts are left where they are (`notes.md`, `todo.txt`)
- an empty old folder moves nothing
- the count of transcripts to move ignores files that are not transcripts
- moved transcripts stay private to the user (`#[cfg(unix)]`, `0o600`)

Cross-volume copying cannot be produced in a tempdir test. `copy_across_volumes` must stay small and fully readable, and its order of operations is a quality check.

**Standards that bite here:** Clean Code: SRP (`move_one` decides, `copy_across_volumes` executes), small functions, exceptions over return codes; `code-craft`: catch close to recovery (per-file errors are counted and logged at the loop, not swallowed deeper); Incubyte: ZOMBIES (zero files, one, many across months, collision boundary, error path).

**Quality checks**

- **A source file is removed only after its destination exists under its final name; no code path removes a source before that rename succeeds.**
- An existing destination is never overwritten.
- Only files whose stem parses as a `SessionId` are moved or counted.
- `move_transcripts` never returns early on a per-file failure; it reports the file as `kept`.
- No folder path appears in any log line.
- `io::ErrorKind::CrossesDevices` is stable since Rust 1.85, and the workspace MSRV is 1.88 [inferred from the Rust 1.85 release notes]. If it fails to compile, stop and escalate rather than matching on raw OS codes.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.

---

## Slice 4 — Change the folder from the tray backend

**Key decision:** One async Tauri command, `change_transcripts_folder`, owns the flow (decision 6): guard → pick → swap under the recorder lock → persist → offer to move → notify. A second command, `transcripts_folder`, returns the current folder for display. The shell lives in a new `transcripts_location.rs` module, so `tray.rs` (near its line limit) is untouched apart from slice 2's one-line change.

**Structure**

- `apps/scribe-tray/src-tauri/Cargo.toml` (modify) — add `tauri-plugin-dialog = "2"`. Resolver 3 is MSRV-aware [verified `resolver = "3"`], so it selects a release compatible with Rust 1.88. The latest 2.8.3 states Rust 1.90 in its docs, so expect an older 2.x.
- `apps/scribe-tray/src-tauri/src/lib.rs` (modify) — `.plugin(tauri_plugin_dialog::init())` in `with_plugins`; register both commands in `with_commands`. No `dialog:*` capability is needed, because the dialog is only called from Rust ([context §5](../specs/003-configurable-transcripts-folder-context.md#5-known-dragons)).
- `apps/scribe-tray/src-tauri/src/recording.rs` (modify) — `impl RecordingController { pub fn relocate_store(&self, store: &MarkdownTranscriptStore, folder: TranscriptsFolder) -> Result<(), RecorderError> }`. It holds `self.recorder()` for the whole call; `RecorderState::Recording` gives `Err(RecorderError::AlreadyRecording)` [verified variant]; otherwise `store.relocate(folder)`. Do not use `ensure_idle`, because the `Locked` state must still allow a change.
- `apps/scribe-tray/src-tauri/src/transcripts_location.rs` (new):
  - `#[derive(Serialize)] #[serde(tag = "outcome", rename_all = "snake_case")] pub enum FolderChange { Cancelled, Unchanged, Recording, Changed { folder: String, moved: usize, kept: usize } }`
  - `#[derive(Debug, thiserror::Error)] pub enum FolderChangeError { UnsupportedPath, Picker(..), Settings(#[from] tauri_plugin_store::Error), Store(#[from] StoreError) }`
  - `pub fn change<R: Runtime>(app: &AppHandle<R>) -> Result<FolderChange, FolderChangeError>`
  - `pub fn current_folder<R: Runtime>(app: &AppHandle<R>) -> String` is the display string of `store.folder().path()`.
  - Helpers: `fn pick_folder<R>(app, starting_at: &Path) -> Result<Option<PathBuf>, FolderChangeError>` uses `app.dialog().file().set_title(..).set_directory(..).blocking_pick_folder()` → `Option<FilePath>` → `FilePath::into_path()` [verified 2.8.3 API]. `fn persist_folder<R>(app, folder: &Path) -> Result<(), FolderChangeError>` follows `privacy::acknowledge`'s set + `save()` [verified]. `fn offer_to_move<R>(app, from, to) -> Result<MoveReport, StoreError>`. `fn ask_to_move<R>(app, count: usize) -> bool` uses `app.dialog().message(..).title(..).buttons(MessageDialogButtons::OkCancelCustom("Move".into(), "Keep in old folder".into())).blocking_show()`, which returns `bool` [verified].
- `apps/scribe-tray/src-tauri/src/commands.rs` (modify) — `#[tauri::command] pub async fn change_transcripts_folder<R: Runtime>(app: AppHandle<R>) -> Result<FolderChange, String>` delegates to `transcripts_location::change`. `#[tauri::command] pub fn transcripts_folder<R: Runtime>(app: AppHandle<R>) -> String`.
- `apps/scribe-tray/src-tauri/src/notifications.rs` (modify) — `pub fn notify_transcripts_folder_changed<R>(app, moved: usize)` and `pub fn notify_transcripts_folder_not_changed<R>(app)`. The text names the folder by its last component only, e.g. "New transcripts will be saved in Meetings. Moved 12 transcripts."

**Collaborators:** The shell reads `RecordingController` and `Arc<MarkdownTranscriptStore>` from managed state [verified both managed in `install`]. It calls `store_markdown::{move_transcripts, count_transcripts, TranscriptsFolder, TRANSCRIPTS_FOLDER_KEY, SETTINGS_FILE_NAME}` and the dialog and store plugins. Nothing in `store-markdown` learns about Tauri.

**Control flow (`change`):**
1. The controller state is `Recording` → `Recording`.
2. `pick_folder` starting at the current folder → none → `Cancelled`.
3. A new `TranscriptsFolder::at(picked)` equal to the previous one → `Unchanged`.
4. The picked path is not valid UTF-8 → `Err(UnsupportedPath)`. It is never persisted lossily ([context §4c](../specs/003-configurable-transcripts-folder-context.md#4c-slice-traps)).
5. `relocate_store(previous → new)` returns `AlreadyRecording` → `Recording`.
6. `persist_folder` fails → `relocate_store` back to the previous folder → `Err(Settings)`.
7. `count_transcripts(previous)` > 0 and `ask_to_move` → `move_transcripts`; otherwise an empty report.
8. Notify changed → `Changed { folder, moved, kept }`.

On any `Err`, the command maps the error to `String` and calls `notify_transcripts_folder_not_changed`. Steps 2 and 7 block on dialogs, so this must run only inside the `async` command, never from a menu or main-thread callback ([context §4c](../specs/003-configurable-transcripts-folder-context.md#4c-slice-traps)). The recorder lock is held only inside `relocate_store`, never across a dialog.

**Tests:** extend `apps/scribe-tray/src-tauri/tests/` with a new `folder_change.rs` that covers what is reachable without an `AppHandle`. `FolderChange` serialises to the wire shape the popover reads, e.g. `{"outcome":"changed","folder":"/x","moved":2,"kept":1}` and `{"outcome":"cancelled"}`. Write one test per variant, named as behaviour. `relocate_store` on an idle controller moves where new transcripts are saved, with the recorder built from `scribe_core::testing` fakes [inferred feasible; `RecordingController::new` is `pub` (verified)]. The dialog and persistence shell is untested by convention ([context §6](../specs/003-configurable-transcripts-folder-context.md#6-test-conventions)), so keep each helper a few lines.

**Standards that bite here:** Clean Code: SRP (one helper per step, `change` reads as the eight-step outline), complexity ≤5 per function, typed errors with `thiserror`; `code-craft`: throw close to detection, catch at the command boundary only; Incubyte: Conventional Commit `feat(tray): ...`.

**Quality checks**

- **Dialogs are only ever invoked from the `async` command; no `blocking_*` dialog call is reachable from the main thread.**
- The recorder lock is never held across a dialog or a file move.
- A failed `save()` leaves the store pointing at the previous folder.
- A cancelled picker changes nothing and shows no notification.
- No `dialog:*` permission is added, and the `opener` scope is unchanged.
- `transcripts_location.rs` holds the flow; `tray.rs` gains no new lines in this slice.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
3. **User-facing flow.** In the running app, invoke the change from the popover (after slice 5) and pick a new folder. The picker opens in front. A recording afterwards lands in `<new>/<YYYY-MM>/`. Accepting the move prompt moves old transcripts. Choosing "Keep in old folder" leaves them in place. The success notification appears.

---

## Slice 5 — Add the Settings view to the popover

**Key decision:** A gear icon button in the popover's actions row switches the popover to a `SettingsPanel`. The panel shows the current transcripts folder and one button, **Change Transcripts Folder**, plus **Back**. The button is disabled while recording, with a visible reason. After a change, the panel refreshes the folder it shows (decision 6).

**Structure**

- `apps/scribe-tray/src/popover/types.ts` (modify) — `export type FolderChange = { readonly outcome: "cancelled" } | { readonly outcome: "unchanged" } | { readonly outcome: "recording" } | { readonly outcome: "changed"; readonly folder: string; readonly moved: number; readonly kept: number };`
- `apps/scribe-tray/src/popover/guards.ts` (modify) — `isFolderChange(value: unknown): value is FolderChange`, following the existing guards [verified pattern].
- `apps/scribe-tray/src/popover/commands.ts` (modify) — `fetchTranscriptsFolder(): Promise<string>` (`invoke("transcripts_folder")`, validated as a string) and `changeTranscriptsFolder(): Promise<FolderChange>` (`invoke("change_transcripts_folder")` + `expecting(isFolderChange, ..)`).
- `apps/scribe-tray/src/popover/useTranscriptsFolder.ts` (new) — `useTranscriptsFolder(): { folder: string | null; change: () => Promise<void> }` fetches on mount and refetches after `change` resolves. Follow `useClaudeDesktopStatus` [verified sibling hook].
- `apps/scribe-tray/src/popover/SettingsPanel.tsx` (new) — `SettingsPanel({ folder, isRecording, onChangeFolder, onBack }: SettingsPanelProps)`. It renders a `section` labelled by an `h2` "Settings" (`tabIndex={-1}`, focused on mount), the label "Transcripts folder" with the folder path, the button "Change Transcripts Folder" (`disabled={isRecording}`, `aria-describedby` pointing to the reason "Stop recording to change the folder." when recording), and a "Back" button.
- `apps/scribe-tray/src/popover/PopoverActions.tsx` (modify) — new prop `onOpenSettings: () => void`. An icon-only `button` with `aria-label="Settings"` and an inline gear SVG (`aria-hidden="true"`) is appended to `actions__secondary`.
- `apps/scribe-tray/src/popover/Popover.tsx` (modify) — `const [view, setView] = useState<"main" | "settings">("main")`. The settings view renders `SettingsPanel` in place of the main content. On Back, return to main and restore focus to the gear button (a ref passed down, or `PopoverActions` forwards it).
- `apps/scribe-tray/src/theme/popover.css` (modify) — BEM classes `settings`, `settings__heading`, `settings__folder`, `settings__reason`, `actions__icon`. They use only tokens from `tokens.css` [verified file exists]. The folder path wraps (`overflow-wrap: anywhere`) so a long path never scrolls horizontally.

**Collaborators:** UI → `commands.ts` → Tauri commands from slice 4. No component calls `invoke` directly.

**Control flow:** Gear click → settings view → focus the heading. Change click → `change()` → on resolve, refetch the folder. On reject, the folder is unchanged; the Rust side already notified, so the UI shows nothing extra. Back → main view → focus the gear. `isRecording` comes from `feed.phase === "recording"`.

**Tests:**
- New `SettingsPanel.spec.tsx`:
  - shows the folder transcripts are saved in
  - changing the folder asks Scribe to change it
  - the folder cannot be changed while recording, and says why
  - going back leaves settings
  - has no accessibility violations (`vitest-axe`), in both idle and recording
- Extend `PopoverActions.spec.tsx`: "opens settings from the popover" (the gear by its accessible name "Settings").
- Extend `Popover.spec.tsx` (`mockIPC` backend): opening settings shows the current folder from `transcripts_folder`; a successful change shows the new folder; going back returns focus to the settings button.

**Standards that bite here:** WCAG 2.2 AA (accessible name on the icon button, visible focus, focus management on view switch, 4.5:1 contrast using existing tokens); SonarJS + ESLint (complexity ≤5, max-depth 3, `prefer-const`, no inline disables); Incubyte: test names in business language; `code-craft`: named constants for labels, following `CONNECT_LABEL`.

**Quality checks**

- **The gear button has the accessible name "Settings", and focus moves to the Settings heading on open and back to the gear on Back.**
- The button text is exactly "Change Transcripts Folder" (the user's wording).
- The change button is `disabled` while recording, with a visible reason linked via `aria-describedby`.
- New styles use only existing CSS custom properties; no new colour literals.
- Existing PopoverActions buttons, handlers and test hooks are unchanged.
- A failed or cancelled change leaves the displayed folder unchanged.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
3. **User-facing flow.** Open the popover, open Settings, see the current folder, press Change Transcripts Folder, pick a folder, reopen the popover and see the new folder. While recording, the button is disabled with its reason shown.
4. **Accessibility.** Same session: tab order goes gear → Settings heading region → Change → Back. Focus is visible on every control. The icon button announces "Settings".

---

## Slice 6 — Replace "Documents/Scribe" text

**Key decision:** Text that names a fixed folder becomes folder-neutral. `StartFailureKind::advice()` keeps returning `&'static str` ([context §4c](../specs/003-configurable-transcripts-folder-context.md#4c-slice-traps)).

**Structure**

- `apps/scribe-tray/src/privacy/PrivacyStatement.tsx` (modify) — the 4th `STATEMENTS` entry becomes "Transcripts are saved as text files in your transcripts folder, Documents/Scribe unless you change it in Settings."
- `apps/scribe-tray/src/privacy/PrivacyStatement.spec.tsx` (modify) — update both exact-string assertions (`getByText`, `toHaveAccessibleDescription`).
- `apps/scribe-tray/src-tauri/src/start_failure.rs` (modify) — the `TranscriptsFolder` advice becomes "The transcript could not be created in your transcripts folder. Check the folder in Settings and try again." Update the matching assertion in `tests/start_failure.rs` if it pins the text [inferred].
- `docs/ARCHITECTURE.md` (modify) — §2 Storage row becomes "One Markdown file per session in `<transcripts folder>/YYYY-MM/` (default `Documents/Scribe`; configurable, stored in the app's `settings.json`)". The §5 File safety line is unchanged.

**Collaborators:** none.

**Control flow:** none.

**Tests:** update the existing assertions listed above, and add no new tests.

**Standards that bite here:** Design Standards: preserve labels and `aria-*` wiring; Incubyte: `docs(...)` / `fix(...)` commit types as appropriate.

**Quality checks**

- No user-visible string in the repo still says transcripts are saved in "Documents/Scribe" without mentioning it can be changed.
- `StartFailureKind::TranscriptsFolder` and the `"transcripts_folder"` wire value are not renamed.
- The privacy statement keeps four items, one button and its `aria-describedby` wiring.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
3. **User-facing flow.** Open the privacy statement window and read the updated storage line.
4. **Accessibility.** Same session: the acknowledge button's accessible description includes the updated line.

---

## Hard floors

- **Slice 3** — category: **migrations** (it is a data move of user transcripts between folders). Floor tier `≥ excellent`. The reviewer runs the security and architecture dimensions unconditionally over `relocation.rs`.

## Escalation log


## Deferred follow-ups

- **Cross-volume move paths have no automated test** (review F8/F21, medium). `copy_across_volumes`, the `.part` cleanup and the no-hard-link rename fallback in `crates/store-markdown/src/relocation.rs` only run when the destination is on another volume or a file system without hard links, which a tempdir cannot produce. Verify by hand: change the folder to a USB drive (exFAT) and to a second APFS volume, accept the move, and confirm every transcript arrives and none is left half-copied.
