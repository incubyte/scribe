# Plan: 008-chrome-like-auto-update

> Spec: [`docs/specs/008-chrome-like-auto-update.md`](../specs/008-chrome-like-auto-update.md) · Context: [`docs/specs/008-chrome-like-auto-update-context.md`](../specs/008-chrome-like-auto-update-context.md) · Authored: 2026-10-23
> Risk: HIGH (a defective updater can strand installs; losing the update-signing key ends updates for every install) · Hard floors: slices 1, 2 · Fabric standards: reachable

---

# Part 1 — Review this

*Everything a reviewer needs to approve or redirect the approach. If you only read one part, read this one.*

## What we're doing

Scribe will update itself: every published GitHub Release reaches every installed copy without anyone downloading anything. Scribe checks once an hour, downloads and verifies a new version in the background, and swaps to it by relaunching itself when nothing is happening: not recording, no call, popover closed. Quitting Scribe or choosing a new "Restart to update" tray item also installs a waiting update, but nobody has to do either.

The approach is forced by three facts. First, an update is only safe if it is signed by a key that never leaves the release Mac, so the release script signs and checks every update file, and the app refuses anything else. Second, the update source is `releases/latest/download/latest.json` on the now-public repo, which never shows a draft ([spec §2](../specs/008-chrome-like-auto-update.md#2-sources), source 7), so publishing the draft stays the one human release decision. Third, Scribe must never lose a meeting, so the "is now a good moment?" decision lives in one pure, tested function in `scribe-core`.

One deliberate exception: the first version that contains the updater has to be installed by hand once. Versions before it cannot update themselves.

## How it breaks down

Slice 1 makes every release carry signed update files and a manifest. Slice 2 makes Scribe find and download them. Slices 3 to 6 add the three install moments (quit, idle, tray item) plus the Windows fix that lets an install complete while Claude is using `scribe-mcp`. The order is the spec's ([spec §5](../specs/008-chrome-like-auto-update.md#5-slices)); each slice can ship on its own.

| # | Slice | Delivers | Why this tier | Tier |
|---|-------|----------|---------------|------|
| 1 | Publish a signed update manifest | `release.sh` signs, verifies and uploads `latest.json` and the macOS update archive | Secrets and signing; a mistake here ships unverifiable or forged-looking updates | `excellent` |
| 2 | Check hourly and download in the background | Updater plugin, `update-check` thread, core schedule and lifecycle | Async plugin bridged into a thread, startup ordering, CI guard change | `excellent` |
| 3 | Install on quit | Quit installs a waiting update, without relaunching | Small, but platform-specific install semantics | `excellent` |
| 4 | Stop scribe-mcp before the Windows installer copies files | One NSIS pre-install hook | Three lines reusing Tauri's own macro | `fast` |
| 5 | Install silently when idle | Pure install-moment policy, shared in-call flag, idle install and relaunch | The rule that protects meetings; threads and shared state | `excellent` |
| 6 | "Restart to update" tray item | Menu item inserted and removed, disabled while recording | Menu kept in managed state, kept in sync with recorder state | `excellent` |

Slices 3, 5 and 6 share one install function in the shell; only what happens after the install (exit or relaunch) differs ([decision 7](#decisions-made)).

## Decisions made

**1. The updater lives only in the Rust shell, and the webview gets no updater permission.** Everything runs through `tauri_plugin_updater::UpdaterExt` from Rust; `capabilities/default.json` is not touched. Lands in slice 2. *Rejected: the JS API, which needs `updater:default` and would let any webview code trigger installs.*

**2. The public key and update URL go in the base config; producing update files is switched on only by a new release-only config.** A build test on 2026-10-23 showed a build with the public key and no private key succeeds, while turning on `createUpdaterArtifacts` without the private key fails with "A public key has been found, but no private key". So `tauri.conf.json` gets `plugins.updater`, and a new `tauri.release.conf.json` (used only by `release.sh`) sets `bundle.createUpdaterArtifacts`. CI, teammates and `release.yml` keep working without the key. This settles [spec §9.2](../specs/008-chrome-like-auto-update.md#92-does-tauri-build-fail-or-skip-signing-when-createupdaterartifacts-is-on-and-no-private-key-is-set). Lands in slice 1. *Rejected: putting `createUpdaterArtifacts` in `tauri.ci.conf.json`, which `release.yml` also uses.*

**3. Each signature is bound to its version, and the app enforces it.** `tauri build` already writes `version:<x.y.z>` into the signature's trusted comment (checked on 2026-10-23), and the plugin's `requireSignedVersion` rejects a manifest that pairs a new version number with an old signed file. Turning it on from the first updater release costs nothing. Lands in slices 1 and 2. *Rejected: leaving it off, which lets anyone who can tamper with the manifest roll installs back to an older signed release.*

**4. The release script checks every update file before uploading it.** It verifies each signature against the committed public key with `minisign` (checked on 2026-10-23 to accept Tauri signatures and reject a tampered file), checks the signed version, and runs Gatekeeper on the macOS app extracted from the archive after the `.dmg` is notarized. Lands in slice 1. *Rejected: trusting the build output, which would let a key mix-up or a non-notarized app reach users silently.*

**5. One background thread does all the timing.** The `update-check` thread wakes every 30 seconds; it checks the manifest when an hour has passed since the last check, and when an update is waiting it asks the install policy whether now is a good moment. The plugin's async calls are bridged with `tauri::async_runtime::block_on`, like the rest of the shell, which has no async code. Lands in slices 2 and 5. *Rejected: reacting to recorder, call and popover events, which would spread install triggers over four modules.*

**6. The rules live in `scribe-core` as plain data and functions.** A new `scribe_core::update` module holds the check schedule, the update lifecycle (up to date, downloading, ready), and `install_moment`, which turns "is an update due, recorder state, in a call, popover open" into install-now or wait-with-a-reason. The lifecycle is generic over the downloaded package, so the core never sees Tauri types. Lands in slices 2 and 5. *Rejected: deciding inside the shell thread, which is untestable by this repo's convention.*

**7. Whether Scribe relaunches is chosen at install time.** The plugin's `Update` can be cloned and has `restart_after_install(bool)`, so the same downloaded update installs without a relaunch on quit and with one when idle or from the tray item. On macOS the shell calls `app.exit(0)` or `app.restart()` after installing; on Windows the plugin runs the installer and exits Scribe itself. Lands in slices 3, 5 and 6. *Rejected: a second manifest check at quit time just to change the relaunch setting.*

**8. Debug builds never check for updates.** `tauri dev` and debug test builds skip the update thread entirely, so a developer's build never downloads a release and tries to install over itself. Lands in slice 2.

**9. The Windows installer stops `scribe-mcp.exe` with Tauri's own "app is running" macro.** The installer template already includes `CheckIfAppIsRunning`, which uses Windows Restart Manager on an exact file path and stops it silently during an update. The hook calls it for `$INSTDIR\scribe-mcp.exe`, which stops only processes running that exact file. Lands in slice 4. *Rejected: `taskkill /IM scribe-mcp.exe`, which matches by name and could stop a copy from somewhere else.*

**10. The private key is created and stored by Sapan, never by an agent.** The key file and its password are secrets; an agent that generated them would put them in a transcript. Slice 1 starts with two commands Sapan runs, and the build agent only reads the public key. Lands in slice 1.

**11. The CI rule "only Parakeet may use an HTTP client" gains one named exception.** The updater plugin brings in `reqwest` 0.13, the same version Parakeet uses, so the guard stays meaningful when it allows exactly `tauri-plugin-updater` as well. `docs/ARCHITECTURE.md`'s privacy line and the README say Scribe also checks GitHub for updates and sends nothing about the user. Lands in slice 2.

## Where this could go wrong

- **A bug in the first updater release strands it.** Versions only move forward, so a broken updater can only be fixed by reinstalling by hand. Guarded by slice 2's check that the update thread starts first and cannot be stopped by other startup failures, and by the manual end-to-end check in slices 3 and 5 before a release is published.
- **The update key is lost.** Every installed copy would stop updating for good. Guarded by slice 1's key gate and the ADR that names where the backup lives; the backup itself is Sapan's step.
- **Windows behaviour is unproven.** CI only runs clippy for Windows and no real Windows install has happened yet (ADR 6). Slices 3 and 4 have Windows manual checks; until they run, Windows updating is untested.
- **The idle rule installs during a meeting.** Guarded by slice 5's pure-policy tests, one per waiting reason, and by including the 30-second call grace in the shared in-call flag.
- **Signing silently stops happening.** Guarded by slice 1's pre-upload checks, which fail the release instead of uploading unsigned or wrongly signed files.

## What's out of scope

Update channels, an off switch, delta updates, downgrades, a notification after updating, signed Windows builds, updating the Claude plugin, and building releases with the GitHub Actions fallback ([spec §9.4](../specs/008-chrome-like-auto-update.md#94-the-windows-release-fallback)).

---

# Part 2 — Build detail

*Prescriptive detail for the build agent. Reviewers can skim or skip.*

**Confidence markers used below:** `[verified]` = the planner read the code or docs and confirmed it. `[inferred]` = pattern-matched from a sibling implementation, not confirmed — the build agent should check before assuming.

## Shared conventions (all slices)

- **Architecture:** hexagonal (ADR 1, [context](../specs/008-chrome-like-auto-update-context.md)). New rules go in `crates/scribe-core/src/update/`; Tauri and plugin types stay in `apps/scribe-tray/src-tauri/src/updates/`. No `#[cfg(target_os)]` in core.
- **Dependency direction:** `apps/scribe-tray` → `scribe-core`. `updates` calls `tray::show_*`, `popover::is_open` and `RecordingController::state`; `tray` calls `updates::install_on_quit` and `updates::restart_to_update`. This two-way shell call shape already exists between `tray` and `recording`.
- **Lexicon (from [context §4b](../specs/008-chrome-like-auto-update-context.md#4b-slice-lexicon) — do not rename):** shell module `updates` with `updates::install`, `THREAD_NAME = "update-check"`; core module `scribe_core::update`; `CHECK_INTERVAL`; `InstallMoment` with `Now` and `Wait(WaitReason)`; `WaitReason::{NothingReady, Recording, InCall, PopoverOpen}`; `install_moment`; `CallWatch::is_in_call`; `popover::is_open`; `MENU_RESTART_TO_UPDATE = "restart-to-update"`; `MenuAction::RestartToUpdate`; label `"Restart to update"`; `tauri.release.conf.json`; `require_update_signing_key`; `write_update_manifest`. New names introduced by this plan: `UpdateController` (managed state, `*Controller` convention), `UpdateLifecycle<P>`, `ReleaseVersion`, `CheckSchedule`, `StagedPackage`, `AfterInstall::{Exit, Relaunch}`, `CallPresence`, `TrayMenu`. Never name a shell type with the bare word "Updater" (`tray_listener.rs` has `TrayUpdater`).
- **Tests:** core rules get new files in `crates/scribe-core/tests/` (`update_schedule.rs`, `update_lifecycle.rs`, `install_moment.rs`) and an extension to `tests/call_watch.rs`; shell-pure code extends `apps/scribe-tray/src-tauri/tests/menu_action.rs`. AppHandle glue is not unit-tested and goes into `.cargo/mutants.toml` `exclude_globs`. Test names are business-language sentences; time is passed in as `Instant`. `release.sh` and `hooks.nsh` are checked by their slices' manual runs.
- **Limits:** `clippy.toml` caps functions at 20 lines, 3 arguments, cognitive complexity 5, with `-D warnings` [verified].
- **Dependency:** `tauri-plugin-updater = "2.14"` in `apps/scribe-tray/src-tauri/Cargo.toml`; it needs Tauri ^2.12 (lock has 2.12.4) and Rust 1.90 (CI pins 1.93.1) [verified on crates.io]. Register with `tauri_plugin_updater::Builder::new().build()` [verified].

---

## Slice 1 — Publish a signed update manifest with every release

**Key decision:** Sapan creates the key; `release.sh` gains a key gate, builds the update files through the new release-only config ([decision 2](#decisions-made)), verifies them ([decision 4](#decisions-made)), writes `latest.json`, and uploads it with a versioned macOS archive. The public key, update URL, Windows quiet mode and version-bound signatures go into the base config ([decision 3](#decisions-made)).

**Before building (Sapan, one time, [decision 10](#decisions-made)):**
1. `npx tauri signer generate -w ~/.tauri/scribe-update.key` in `apps/scribe-tray`, choosing a password when prompted.
2. `security add-generic-password -a "$USER" -s scribe-update-key -w`, entering the same password.
3. Save the key file and password in the password manager.

The build agent checks `~/.tauri/scribe-update.key.pub` exists; if it does not, it stops and asks Sapan to run these steps. It never generates, prints or reads the private key.

**Structure**

- `apps/scribe-tray/src-tauri/tauri.conf.json` (modify) — add `plugins.updater`: `pubkey` (contents of `~/.tauri/scribe-update.key.pub`, a single base64 line), `endpoints: ["https://github.com/incubyte/scribe/releases/latest/download/latest.json"]`, `requireSignedVersion: true`, `windows: { installMode: "quiet" }`. Keys are camelCase [verified in plugin `config.rs`].
- `apps/scribe-tray/src-tauri/tauri.release.conf.json` (new) — `{ "bundle": { "createUpdaterArtifacts": true } }` only.
- `scripts/release.sh` (modify):
  - variables: `update_key_path="$HOME/.tauri/scribe-update.key"`, `update_key_service="scribe-update-key"`, `release_config="src-tauri/tauri.release.conf.json"`, `update_archive="$root/target/release/bundle/macos/Scribe.app.tar.gz"`, `versioned_update_archive="$root/target/release/Scribe_${version}_aarch64.app.tar.gz"`, `manifest="$root/target/release/latest.json"`, `download_base="https://github.com/incubyte/scribe/releases/download/$tag"`.
  - `require_update_signing_key()` — key file exists, Keychain has a password for `$update_key_service`, `minisign` is on `PATH`; each failure prints the setup command to stderr and exits 1, like `require_notary_credentials` [verified].
  - `export_update_signing_key()` — exports `TAURI_SIGNING_PRIVATE_KEY="$update_key_path"` (the CLI accepts a path for `build`) [verified by test build] and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` from `security find-generic-password -s "$update_key_service" -w` [verified flag].
  - `build_macos()` — `--bundles app,dmg` (a `dmg`-only build makes no update archive: "no updater-enabled targets were built" [verified in CLI 2.12.3]) and adds `--config "$release_config"` after the CI overlay; multiple `--config` flags merge [verified by test build].
  - `build_windows()` — adds `--config "$release_config"`. Installer signature at `<installer>.sig` next to the `.exe` [inferred from the macOS naming; glob it].
  - `verify_update_signature <file>` — decodes the base64 public key from `tauri.conf.json` (`jq -r .plugins.updater.pubkey`) and the base64 `<file>.sig` into a `mktemp -d` directory, runs `minisign -V -p <pub> -m <file> -x <sig>`, and requires the output to contain `version:$version`; fails the release otherwise. Decoding to files and minisign verification [verified on 2026-10-23].
  - `verify_macos_update_archive()` — extracts `$update_archive` into a temp dir (the archive's root is `Scribe.app/` [verified]) and requires `spctl --assess --type execute --verbose` to report `source=Notarized Developer ID`.
  - `write_update_manifest()` — `jq -n` writes `$manifest`: `version`, `notes` (contents of `scripts/release-notes.md`), `pub_date` (`date -u +%Y-%m-%dT%H:%M:%SZ`), and `platforms.darwin-aarch64` / `platforms.windows-x86_64`, each with `url` (`$download_base/<asset name>`) and `signature` (the `.sig` file's contents, not decoded).
  - `publish_assets()` — also uploads `$versioned_update_archive` and `$manifest`.
- `docs/ARCHITECTURE.md` (modify) — ADR 9: Tauri updater with signed, version-bound updates from `releases/latest`; key at `~/.tauri/scribe-update.key`, password in Keychain service `scribe-update-key`, both backed up in Sapan's password manager; losing the key ends updates for every install; `tauri.release.conf.json` is the only place update files are switched on.
- `CLAUDE.md` (modify) — Releasing section: the key, the Keychain entry and `brew install minisign` as prerequisites.

**Collaborators:** `npx tauri build`, `security`, `minisign`, `jq`, `spctl`, `gh`. No Rust changes.

**Control flow (bottom of `release.sh`):** `require_clean_main` → `require_notary_credentials` → `require_update_signing_key` → `export_update_signing_key` → `build_macos` → `notarize_macos` → `verify_macos_update_archive` → `verify_update_signature "$update_archive"` → copy `$update_archive` to `$versioned_update_archive` → `build_windows` → `verify_update_signature "<installer>"` → `build_plugin_zip` → `write_update_manifest` → `tag_release` → `publish_assets`. Any failed check exits before tagging.

**Tests:** no automated tests (bash and config). Verified by the spec's release dry run: a throwaway version to a draft, inspect assets and `latest.json`, delete the draft and tag. Also run `npx tauri build --bundles app --config src-tauri/tauri.ci.conf.json` with no key in the environment (AC 5.1.6).

**Standards that bite here:** Security pack (secrets out of repo and logs); Clean Code: small single-purpose functions; Incubyte: Conventional Commits.

**Quality checks**

- **No line of `release.sh` prints the private key, its password or a `.sig` body; the password only ever lives in an exported variable.** (AC 5.1.7)
- `tauri.ci.conf.json` and `tauri.conf.json` contain no `createUpdaterArtifacts`. (AC 5.1.6)
- `latest.json` from a dry run has `version` equal to `tauri.conf.json`'s, both platform keys, URLs that resolve on the release, and signatures equal to the `.sig` files. (AC 5.1.1, 5.1.2)
- The release stops before tagging when a signature fails `minisign`, lacks `version:<version>`, or the extracted app is not "Notarized Developer ID". (AC 5.1.3, 5.1.4)
- Removing the key file or the Keychain entry makes the script stop before building with a message naming the fix. (AC 5.1.5)
- ADR 9 and `CLAUDE.md` name the key's location, backup and loss consequence. (AC 5.1.8)

---

## Slice 2 — Scribe checks for updates hourly and downloads a verified update in the background

**Key decision:** Add the plugin and a `updates` shell module whose `update-check` thread starts first in `start_scribe` ([decision 5](#decisions-made)), checks when the core `CheckSchedule` says so, and stages a verified download in an `UpdateController` that wraps the core `UpdateLifecycle` ([decision 6](#decisions-made)). Debug builds skip it ([decision 8](#decisions-made)). Update the CI guard and the privacy wording ([decision 11](#decisions-made)).

**Structure**

- `crates/scribe-core/src/update/mod.rs` (new) — re-exports, like `call/mod.rs`; `pub const CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);`.
- `crates/scribe-core/src/update/schedule.rs` (new) — `pub struct CheckSchedule { interval: Duration, last_check: Option<Instant> }`; `#[must_use] pub fn new(interval: Duration) -> CheckSchedule`; `#[must_use] pub fn is_check_due(&self, now: Instant) -> bool` (due when never checked); `pub fn mark_checked(&mut self, now: Instant)`.
- `crates/scribe-core/src/update/lifecycle.rs` (new) — `pub struct ReleaseVersion(String)` with `new(impl Into<String>)`, `as_str(&self) -> &str`, `Display`, `PartialEq`, `Clone`; `pub struct UpdateLifecycle<P> { phase: Phase<P> }` with private `enum Phase<P> { UpToDate, Downloading(ReleaseVersion), Ready { version: ReleaseVersion, package: P } }`; methods: `new() -> Self`; `begin_download(&mut self, version: &ReleaseVersion) -> bool` (false while downloading, or when ready with the same version); `finish_download(&mut self, package: P)` (Downloading(v) → Ready{v, package}; no-op otherwise); `abandon_download(&mut self)` (Downloading → UpToDate); `is_ready(&self) -> bool`; `ready_version(&self) -> Option<&ReleaseVersion>`; `package_to_install(&self) -> Option<P> where P: Clone`.
- `crates/scribe-core/src/lib.rs` (modify) — `pub mod update;`.
- `apps/scribe-tray/src-tauri/src/updates/mod.rs` (new) — `pub fn install<R: Runtime>(app: &AppHandle<R>)` (manages `UpdateController`, spawns `THREAD_NAME`; returns immediately in debug builds via `cfg!(debug_assertions)`); private `check_forever`, `tick`, `check_and_stage`.
- `apps/scribe-tray/src-tauri/src/updates/controller.rs` (new) — `pub struct StagedPackage { pub update: tauri_plugin_updater::Update, pub bytes: Arc<[u8]> }` (`Clone`; `Update: Clone + Send + Sync` [verified: `Clone` on docs.rs, `impl Resource for Update` requires `Send + Sync`]); `pub struct UpdateController { lifecycle: Mutex<UpdateLifecycle<StagedPackage>> }` with `begin_download`, `stage(package)`, `abandon`, `is_ready`, `package_to_install`, each a thin poison-tolerant lock (`unwrap_or_else(PoisonError::into_inner)`).
- `apps/scribe-tray/src-tauri/src/updates/release_feed.rs` (new) — the plugin adapter: `pub fn newer_release<R: Runtime>(app: &AppHandle<R>) -> Result<Option<Update>, FeedError>` (`app.updater_builder().timeout(REQUEST_TIMEOUT).build()?` then `block_on(updater.check())`); `pub fn download(update: &Update) -> Result<Vec<u8>, FeedError>` (`block_on(update.download(|_, _| {}, || {}))`, which verifies the signature and version [verified: "download already verified the bytes"]). `FeedError` is a `thiserror` enum with `#[from] tauri_plugin_updater::Error`. `REQUEST_TIMEOUT = 5 minutes`. No `.header()` calls (invariant 4.6). Signatures of `check`, `download`, `updater_builder` [verified on docs.rs 2.14.0].
- `apps/scribe-tray/src-tauri/src/lib.rs` (modify) — `.plugin(tauri_plugin_updater::Builder::new().build())` in `with_plugins`; `updates::install(app);` as the first line of `start_scribe`; `mod updates;`.
- `apps/scribe-tray/src-tauri/Cargo.toml` (modify) — `tauri-plugin-updater = "2.14"`.
- `.github/workflows/ci.yml` (modify) — the HTTP-client guard's filter becomes `grep -v -E '^(stt-parakeet|tauri-plugin-updater) '` and its step name says Parakeet and the updater; both depend on `reqwest` 0.13.x [verified], so `cargo tree -i reqwest` still resolves one package.
- `.cargo/mutants.toml` (modify) — add `apps/scribe-tray/src-tauri/src/updates/**`.
- `README.md`, `docs/ARCHITECTURE.md` §5 (modify) — the network sentence names the hourly update check to GitHub and says it sends nothing about the user or their calls.

**Collaborators:** `updates` → `scribe_core::update`, `tauri_plugin_updater::UpdaterExt`. Nothing in core imports Tauri.

**Control flow:**
- `install`: guard debug build → manage `UpdateController::new()` → spawn named thread running `check_forever`; spawn failure logged with `tracing::error!`.
- `check_forever`: create `CheckSchedule::new(CHECK_INTERVAL)`; loop `tick(app, &mut schedule, Instant::now())` then `thread::sleep(TICK_INTERVAL)` (`TICK_INTERVAL = 30 s`, shell const).
- `tick`: if `schedule.is_check_due(now)` → `mark_checked(now)` → `check_and_stage(app)`. (Slice 5 adds the idle-install step here.)
- `check_and_stage`: `newer_release` → `Err` → `tracing::warn!(%error, "could not check for an update")`, return; `Ok(None)` → `tracing::debug!`, return; `Ok(Some(update))` → `ReleaseVersion::new(update.version.clone())` → `begin_download` false → return; `download` → `Err` → `abandon`, `tracing::error!(%error, version = %version, "could not download the update")`; `Ok(bytes)` → `stage(StagedPackage { update, bytes: bytes.into() })` → `tracing::info!(version = %version, "an update is ready")`.

**Tests:**
- New `crates/scribe-core/tests/update_schedule.rs` — the first check is due at launch; no second check before an hour; a check is due again once an hour has passed (AC 5.2.1).
- New `crates/scribe-core/tests/update_lifecycle.rs` — a found release starts a download; a finished download makes the update ready with its version; a failed download returns to up to date; no second download starts while one runs; the same version is not downloaded again once ready; a newer version replaces a ready one (AC 5.2.2, 5.2.4, 5.2.5, 5.2.6). Uses a small `Clone` test package type, not Tauri types.
- Shell glue: manual check per spec verification (log lines, only GitHub contacted).

**Standards that bite here:** Security pack (HTTPS only, no user data, least privilege, AC 5.2.8); Clean Code: SRP, guard clauses, functions ≤ 20 lines; ZOMBIES for lifecycle tests (zero = never checked, one = one release, boundaries = exactly one hour).

**Quality checks**

- **`updates::install` is the first statement of `start_scribe`, cannot return an error, and nothing after it can stop the thread.** (AC 5.2.7)
- `capabilities/default.json` is unchanged and no `updater:` permission exists anywhere. (AC 5.2.8)
- No request builder call adds headers or query parameters; the only endpoint is the one in `tauri.conf.json`. (AC 5.2.8, invariant 4.6)
- `scribe-core` gains no dependency on Tauri, the plugin, or any HTTP crate. (spec §7.2)
- Each failure path logs exactly one line and leaves the lifecycle up to date. (AC 5.2.4, 5.2.5)
- CI's HTTP-client step still fails for any new `reqwest` dependent other than the two named. (decision 11)
- README and ARCHITECTURE §5 mention the update check; `PrivacyStatement.tsx` is unchanged. (AC 5.2.9)

---

## Slice 3 — Quitting Scribe installs a downloaded update

**Key decision:** `tray::quit` keeps "stop the recording, then exit" and inserts `updates::install_on_quit` between them. Installing goes through one shell function, `install_staged(app, AfterInstall::Exit)`, which clones the staged `Update` and calls `restart_after_install(false)` before `install` ([decision 7](#decisions-made)).

**Structure**

- `apps/scribe-tray/src-tauri/src/updates/install.rs` (new) — `pub enum AfterInstall { Exit, Relaunch }`; `pub fn install_staged<R: Runtime>(app: &AppHandle<R>, after: AfterInstall)`; private `apply(package: StagedPackage, after: &AfterInstall) -> Result<(), tauri_plugin_updater::Error>` (`package.update.restart_after_install(matches!(after, AfterInstall::Relaunch)).install(&package.bytes)` [verified: `Update::restart_after_install(self, bool) -> Self`, `install(&self, impl AsRef<[u8]>)`]); private `finish<R>(app, after)` (`Exit` → `app.exit(0)`, `Relaunch` → `app.restart()`).
- `apps/scribe-tray/src-tauri/src/updates/mod.rs` (modify) — `pub fn install_on_quit<R: Runtime>(app: &AppHandle<R>)` → `install_staged(app, AfterInstall::Exit)` when an update is ready.
- `apps/scribe-tray/src-tauri/src/tray.rs` (modify) — `quit`: `RecordingController::stop()` → `updates::install_on_quit(app)` → `app.exit(0)`.

**Collaborators:** `tray` → `updates`; `updates::install` → plugin `Update`.

**Control flow:**
- `install_on_quit`: no package (`package_to_install` is `None`) → return; otherwise `install_staged(app, AfterInstall::Exit)`.
- `install_staged`: no package → return; `tracing::info!(version, "installing the update")`; `apply` → `Ok` → `finish(app, after)`; `Err` → `tracing::error!(%error, version, "could not install the update")` and return (on quit, `tray::quit`'s `app.exit(0)` follows).
- On Windows `install` launches the installer and calls `std::process::exit(0)` itself, so nothing after it runs [verified in plugin source]; the stop-and-save already happened.

**Tests:** no unit test (AppHandle glue, mutation-excluded with `updates/**`). Manual: the spec's macOS check, plus the Windows check that quit runs a silent install with no UAC prompt and does not relaunch Scribe (AC 5.3.1–5.3.5).

**Standards that bite here:** Clean Code: no flag arguments (the enum is a category), single level of abstraction in `quit`.

**Quality checks**

- **`RecordingController::stop()` returns before any install starts.** (AC 5.3.3)
- With no update ready, `quit` does exactly what it did before. (AC 5.3.5)
- An install error is logged once and Scribe still exits. (AC 5.3.4)
- The quit path installs with `restart_after_install(false)`. (AC 5.3.2)

---

## Slice 4 — The Windows installer stops a running scribe-mcp so updates complete

**Key decision:** Add `NSIS_HOOK_PREINSTALL` to `hooks.nsh` and call Tauri's `CheckIfAppIsRunning` for `$INSTDIR\scribe-mcp.exe` ([decision 9](#decisions-made)). Record in the README what Claude Desktop does afterwards.

**Structure**

- `apps/scribe-tray/src-tauri/windows/hooks.nsh` (modify) — `!define SCRIBE_MCP_EXE "scribe-mcp.exe"` at the top with the other defines; new `!macro NSIS_HOOK_PREINSTALL` containing `!insertmacro CheckIfAppIsRunning "$INSTDIR\${SCRIBE_MCP_EXE}" "Scribe"`. The macro exists in the installed CLI's template, `utils.nsh` is included before the hooks file, and in silent mode it force-stops through Restart Manager without a prompt [verified in CLI 2.12.3 strings and the Tauri template]. `NSIS_HOOK_POSTINSTALL` stays as is.
- `README.md` (modify, after the hardware run) — one sentence in the Claude Desktop section on whether Claude reconnects on its own or needs a restart after an update.

**Collaborators:** none in Rust.

**Control flow:** pre-install → stop processes running that exact file (none running → nothing happens) → normal copy → post-install VC++ step.

**Tests:** manual on real Windows hardware with Claude Desktop connected (AC 5.4.1, 5.4.4); a reinstall with Claude closed shows unchanged behaviour (AC 5.4.3).

**Standards that bite here:** Clean Code: named constants instead of literal strings.

**Quality checks**

- **Processes are matched by full path under `$INSTDIR`, never by image name alone.** (AC 5.4.2)
- `NSIS_HOOK_POSTINSTALL` is byte-for-byte unchanged. (AC 5.4.3)

**Open at build time (AC 5.4.4):** whether Claude Desktop restarts the server by itself can only be seen on a Windows PC ([spec §9.1](../specs/008-chrome-like-auto-update.md#91-does-claude-desktop-restart-a-stopped-scribe-mcp-server-on-its-own)). Probe: update with Claude Desktop open, then ask Claude to list Scribe's tools before and after restarting Claude; write the observed behaviour into the README.

---

## Slice 5 — Scribe installs and relaunches silently when idle

**Key decision:** Add `install_moment` and failed-install tracking to the core ([decision 6](#decisions-made)), publish the call watcher's grace-aware in-call state through managed `CallPresence`, add `popover::is_open`, and have each `update-check` tick ask the policy and, on `Now`, call `install_staged(app, AfterInstall::Relaunch)`.

**Structure**

- `crates/scribe-core/src/update/install_moment.rs` (new) — `pub struct InstallConditions { pub install_due: bool, pub recorder: RecorderState, pub in_call: bool, pub popover_open: bool }`; `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum WaitReason { NothingReady, Recording, InCall, PopoverOpen }`; `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum InstallMoment { Now, Wait(WaitReason) }`; `#[must_use] pub fn install_moment(conditions: &InstallConditions) -> InstallMoment`.
- `crates/scribe-core/src/update/lifecycle.rs` (modify) — `Phase::Ready` gains `failed_install_at: Option<Instant>`; new `record_failed_install(&mut self, now: Instant)` and `#[must_use] is_install_due(&self, now: Instant) -> bool` (ready, and either never failed or `CHECK_INTERVAL` has passed since the failure).
- `crates/scribe-core/src/call/call_watch.rs` (modify) — `#[must_use] pub fn is_in_call(&self) -> bool` returning `in_call`, which stays true through the grace [verified in `release`].
- `apps/scribe-tray/src-tauri/src/call_watch.rs` (modify) — `pub struct CallPresence(AtomicBool)` with `is_in_call(&self) -> bool`; `install` manages it before spawning; `watch_forever` stores `watch.is_in_call()` after every `observe`.
- `apps/scribe-tray/src-tauri/src/popover.rs` (modify) — `pub fn is_open<R: Runtime>(app: &AppHandle<R>) -> bool` (`popover_window` → `is_visible().unwrap_or(false)`; missing window → `false`); `toggle` and `open_for_call` keep their inline checks.
- `apps/scribe-tray/src-tauri/src/updates/controller.rs` (modify) — `is_install_due(now)`, `record_failed_install(now)`.
- `apps/scribe-tray/src-tauri/src/updates/install.rs` (modify) — on `apply` error, `UpdateController::record_failed_install(Instant::now())`.
- `apps/scribe-tray/src-tauri/src/updates/mod.rs` (modify) — `tick` gains `install_if_idle(app, now)`; private `current_conditions(app, now) -> InstallConditions`.

**Collaborators:** `updates` reads `RecordingController` via `app.try_state` (missing → `RecorderState::Locked`, not recording), `CallPresence` via `app.try_state` (missing → in call, the safe answer), `popover::is_open`, and `UpdateController`.

**Control flow:**
- `install_moment`: not `install_due` → `Wait(NothingReady)`; recorder `Recording` → `Wait(Recording)`; `in_call` → `Wait(InCall)`; `popover_open` → `Wait(PopoverOpen)`; else `Now`. `Locked` and `Idle` are not recording.
- `install_if_idle`: `install_moment(&current_conditions(app, now))` → `Now` → `install_staged(app, AfterInstall::Relaunch)`; `Wait(_)` → return without logging.
- macOS: install then `app.restart()`; Windows: installer relaunches Scribe with `/R` because `restart_after_install(true)` [verified: `Quiet` → `/S`, restart → `/R` in plugin `config.rs`].

**Tests:**
- New `crates/scribe-core/tests/install_moment.rs` — installs when nothing is happening; waits when no update is due; waits while recording; waits during a call even when not recording; waits while the popover is open; a locked recorder does not hold back an update (AC 5.5.1–5.5.4, 5.5.6).
- Extend `crates/scribe-core/tests/update_lifecycle.rs` — a failed install is retried only after an hour; an update with no failed install is due at once (AC 5.5.5).
- Extend `crates/scribe-core/tests/call_watch.rs` — a call still counts during the grace after the call app lets go; a call no longer counts once the grace has passed (AC 5.5.3).

**Standards that bite here:** Clean Code: guard clauses, complexity ≤ 5 (the policy is a chain of guards); test-craft: one test per waiting reason, named in business language; ARCHITECTURE §7: no Tauri types in core.

**Quality checks**

- **`install_moment` returns `Wait` whenever the recorder is `Recording` or a call is in progress, including the grace, whatever the other inputs.** (invariant 4.1, AC 5.5.2, 5.5.3)
- `CallPresence` is an atomic store per poll; the 2-second call poll gains no lock. (context §7)
- Idle install shows no window or notification. (AC 5.5.1)
- `scribe-core` still has no `#[cfg(target_os)]` and no Tauri import. (spec §7.2)

---

## Slice 6 — Show "Restart to update" while an update waits

**Key decision:** Keep the tray `Menu` in managed state as `TrayMenu`, insert the "Restart to update" item just above "Quit Scribe" when an update becomes ready, enable or disable it alongside the recording toggle, and route its click to `updates::restart_to_update`, which uses the same `install_staged(app, AfterInstall::Relaunch)` as idle install.

**Structure**

- `apps/scribe-tray/src-tauri/src/menu_action.rs` (modify) — `pub const MENU_RESTART_TO_UPDATE: &str = "restart-to-update";`, `MenuAction::RestartToUpdate`, `ACTIONS_BY_ID` grows to 7 entries.
- `apps/scribe-tray/src-tauri/src/tray.rs` (modify) — `struct TrayMenu<R: Runtime>(Menu<R>)` managed in `build_menu` (alongside `RecordingMenuItem`); `pub fn show_restart_to_update<R: Runtime>(app: &AppHandle<R>)` (no-op if `menu.get(MENU_RESTART_TO_UPDATE)` already exists; otherwise inserts a new `MenuItem` at the position of `MENU_QUIT`, enabled unless recording); private `enable_restart_to_update(app, enabled: bool)` called from `show_recorder_state` (absent item → nothing). `dispatch` gains `Some(MenuAction::RestartToUpdate) => updates::restart_to_update(app)`. Menu API: `insert(&self, item: &dyn IsMenuItem<R>, position: usize)`, `get`, `items` [verified on docs.rs tauri 2.12.4]; `Menu: Send + Sync` for `app.manage` [inferred from `MenuItem` already being managed — the compiler confirms].
- `apps/scribe-tray/src-tauri/src/updates/mod.rs` (modify) — `pub fn restart_to_update<R: Runtime>(app: &AppHandle<R>)` (recording → return; else `install_staged(app, AfterInstall::Relaunch)`); `check_and_stage` calls `tray::show_restart_to_update(app)` after staging.

**Collaborators:** `tray` ↔ `updates` as in the shared conventions; the item disappears on relaunch because the new process builds the menu without it.

**Control flow:**
- `show_restart_to_update`: no `TrayMenu` → return; item exists → return; find `MENU_QUIT` position → insert → set enabled from `RecordingController::state() != Recording`; any menu error → `log_failure` warn.
- `show_recorder_state`: existing behaviour, then `enable_restart_to_update(app, state != RecorderState::Recording)`.

**Tests:** extend `apps/scribe-tray/src-tauri/tests/menu_action.rs` — the "Restart to update" menu id maps to the restart action (AC 5.6.2). Menu insertion, enablement and relaunch are checked manually per the spec (AC 5.6.1, 5.6.3, 5.6.4).

**Standards that bite here:** Clean Code: OCP via the `ACTIONS_BY_ID` table, no duplicated install path; context advisory: keep `tray.rs` growth to the menu concerns only.

**Quality checks**

- **The restart item and idle install call the same `install_staged` with `AfterInstall::Relaunch`; there is no second install implementation.** (context §4c)
- The item is inserted once, directly above "Quit Scribe", and never while no update is ready. (AC 5.6.1)
- The item is disabled whenever the recorder is `Recording` and re-enabled when it stops. (AC 5.6.3)
- Label is exactly "Restart to update", sentence case, no punctuation. (context §8a)

---

## Hard floors

- **Slice 1** — security (signing keys, secrets handling, signature verification in the release pipeline). Floor tier `≥ excellent`.
- **Slice 2** — security (update signature and version verification config, network surface, webview permission boundary). Floor tier `≥ excellent`.

## Escalation log

