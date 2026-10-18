# Task context: 001-scribe-tray-transcriber

> Generated for spec c154d03d-7c8a-4c10-8baa-c0d1cc880d19 at HEAD 78b47786a9b8fc0c8a8e8ec8bf7fa10ee1acd904 · 2026-10-17
> Read first by every agent in /anthara:plan-implementer's slice loop.

**Greenfield.** The repo has only `.anthara/` (plugin config), `.claude/settings.json`, `.gitkeep` and `docs/`. There is no source code, `Cargo.toml`, `package.json`, CI config, test or fixture yet, and no repo `CLAUDE.md`. Every file path below is a **target** taken from [spec §7.3](001-scribe-tray-transcriber.md#73-module-decomposition) and `docs/ARCHITECTURE.md`, not an existing file. Nothing here describes a pattern already in the code. The first slice to write each pattern sets the convention for the rest.

## 1. Affected modules
| Module / file | Role | Notes |
|---|---|---|
| workspace root (`Cargo.toml`, CI workflows) | Cargo workspace + GitHub Actions (macOS + Windows) | Does not exist. Created by 5.1 |
| `apps/scribe-tray` | Primary adapter: Tauri tray, menu, popover, privacy window, composition root | 5.1, 5.2, 5.3, 5.5, 5.7, 5.9, 5.10. React+TS (Vite) webview inside |
| `apps/scribe-mcp` | Primary adapter: `rmcp` stdio binary, read-only | 5.7, 5.8. Depends only on `scribe-core` + `store-markdown` |
| `crates/scribe-core` | Domain: `recording` (Recorder, Session, Segment, EchoFilter), `library` (list/get/search), ports | 5.3, 5.4, 5.6, 5.7, 5.8. No OS/engine/Tauri deps |
| `crates/audio-mic` | `AudioSource` via `cpal` | 5.3 |
| `crates/audio-macos` | System-output `AudioSource` via Core Audio process tap (`cidre`) | 5.4 |
| `crates/audio-windows` | System-output `AudioSource` via WASAPI loopback (`wasapi`) | 5.10 |
| `crates/stt-speechanalyzer` | `Transcriber` via Swift bridge to SpeechAnalyzer | 5.3 |
| `crates/stt-parakeet` | `Transcriber` via `parakeet-rs` + model provisioning | 5.9, 5.10 |
| `crates/store-markdown` | `TranscriptStore` on Markdown files in `Documents/Scribe` | 5.3 (writer), 5.7/5.8 (reader/search) |

## 2. Patterns to follow
No patterns exist in the code yet. The patterns below are **mandated** by the spec and `docs/ARCHITECTURE.md`. Treat them as binding.
- **Hexagonal style.** Adapters → ports ← domain. Ports `AudioSource`, `Transcriber`, `TranscriptStore` are defined in `crates/scribe-core`. Source: `docs/ARCHITECTURE.md` §1, §3 and [spec §7.2](001-scribe-tray-transcriber.md#72-architectural-style).
- **Composition root.** Concrete adapters are wired per OS only in `apps/scribe-tray`, and in `apps/scribe-mcp` for `store-markdown`. Per-OS selection happens there, never in the core (`docs/ARCHITECTURE.md` §8).
- **Typed errors.** Use `Result` with typed errors and `Option` for absence. No sentinels. Fail fast at boundaries such as MCP input, file parsing and device events ([spec §6.1](001-scribe-tray-transcriber.md#61-clean-code-org-standards) 6.1.3).
- **Small units.** Functions under 20 lines, cyclomatic complexity ≤ 5, nesting ≤ 3, at most 3 arguments. No explanatory comments. Use intent-revealing names (6.1.1, 6.1.4).
- **Front-matter contract.** The transcript file shape in [spec §3.2](001-scribe-tray-transcriber.md#32-kinds-of-data) is the storage contract. There are no migrations.
- **New MCP tool path.** Add the capability to `scribe-core::library` first, then expose it in `scribe-mcp` (`docs/ARCHITECTURE.md` §8).
- **AI-ergonomics (advisory).** There is no repo `CLAUDE.md`. Slice 5.1 should consider adding one that names the workspace layout, test commands and the forbidden list in §5 below, so later agents do not guess. Name the status values, speaker labels and tool names as constants or enums, not repeated string literals.

## 3. Reusable utilities
None exist. Do not search for helpers to reuse. Candidates the first slice will create and later slices should reuse once they exist:
- Session-id / file-stem formatting (`YYYY-MM-DD-HHmmss`). Created in 5.3 and reused by 5.7 for id validation.
- Front-matter parse/serialise in `store-markdown`. Written in 5.3 and read in 5.7/5.8.
- Fake adapters for the three ports in `scribe-core` tests. Created in 5.3 and reused by 5.4, 5.6, 5.10.
- Simulated recorder-event source for the webview. Created in 5.5 and reused by 5.9 for download states.

## 4. Adjacent code that must coordinate (per slice)
- **Slice 5.2:** The Recorder `Locked` gate in 5.3 reads the acknowledgement. The tray menu gains "Privacy".
- **Slice 5.3:** The tray menu built in 5.1 (Start/Stop, elapsed time, icon shape). Launch-time incomplete-marking runs at app start.
- **Slice 5.4:** The `scribe-core` Recorder from 5.3 gains a second `AudioSource` lane. Both lanes use one session clock.
- **Slice 5.5:** Event bridge from the Recorder (PartialSegmentUpdated / SegmentFinalised) to the webview. Tray left-click and right-click split. Recent sessions come from the `TranscriptStore` list.
- **Slice 5.6:** Sits between the Transcribers and both sinks (store + popover) per [spec §7.4](001-scribe-tray-transcriber.md#74-data-flow).
- **Slice 5.7:** The `store-markdown` reader shares the parser with the 5.3 writer. The tray menu gains "Copy MCP config". Bundling `scribe-mcp` into the app touches the Tauri bundle config.
- **Slice 5.8:** Uses the `library` module and reader from 5.7.
- **Slice 5.9:** The Recorder `Locked` gate (5.3.1) and the popover (5.5) add progress and failure states.
- **Slice 5.10:** Reuses the 5.3/5.5/5.6 pipeline unchanged. Only adapters and wiring are added.

## 4a. Slice imitation targets
- **Slice 5.1:** Greenfield. No imitation target. This slice sets workspace, CI and Tauri conventions. External prior art, not in repo: Anarlog, Meetily ([spec §2](001-scribe-tray-transcriber.md#2-sources) [9, 10]), and the in-org your-steno-desktop (Tauri).
- **Slice 5.2:** Greenfield. Establishes the first React component, first Tauri command and settings persistence. Shape these carefully.
- **Slice 5.3:** Greenfield. Establishes the port trait shapes and the typed-error style. Each later adapter mirrors the adapter written here.
- **Slice 5.4:** Mirror `crates/audio-mic` (its `AudioSource` impl, created in 5.3).
- **Slice 5.5:** Mirror the 5.2 privacy window for component structure, Tauri command/event wiring and `*.spec.ts` layout.
- **Slice 5.6:** No direct target. Keep the filter as a pure domain unit inside `scribe-core::recording`.
- **Slice 5.7:** Greenfield for `scribe-mcp`. The reader mirrors the 5.3 `store-markdown` writer's error type and module layout.
- **Slice 5.8:** Mirror the 5.7 `list_transcripts` tool for its `library` function, the tool registration and the error mapping.
- **Slice 5.9:** No in-repo target for downloading. The UI mirrors the 5.5 popover states.
- **Slice 5.10:** Mirror `crates/audio-macos` (5.4) for `audio-windows` and `crates/stt-speechanalyzer` (5.3) for `stt-parakeet`.

## 4b. Slice lexicon
- **All slices:** ports=`AudioSource`, `Transcriber`, `TranscriptStore`. Domain types=`Recorder`, `Session`, `Segment`, `EchoFilter`. Recorder states=`Locked`, `Idle`, `Recording`. Status=`recording` \| `complete` \| `incomplete`. Speaker=`Me` \| `Them`. Folder=`Documents/Scribe`.
- **5.1:** menu=`Open transcripts folder`, `Quit Scribe`. Crate dirs as in §1 (`apps/scribe-tray`, `crates/scribe-core`, ...).
- **5.2:** button=`I understand`. Menu=`Privacy`. Type=Privacy acknowledgement.
- **5.3:** menu=`Start recording` / `Stop recording`. File=`<YYYY-MM-DD-HHmmss>.md`. Front-matter keys=`id, title, started_at, ended_at, status, engine, language`. engine=`speechanalyzer`, language=`en-US`. Line=`[hh:mm:ss] **Me:** <text>`. Default title=`Call <date> <time>`. Events=`RecordingStarted`, `SegmentFinalised`, `RecordingStopped`.
- **5.4:** events=`CaptureSilenced`, `OutputDeviceChanged`. Notification="Scribe can't hear the other side of the call" + action `Open System Settings`. Plist key=`NSAudioCaptureUsageDescription`.
- **5.5:** event=`PartialSegmentUpdated`. Type=Partial segment.
- **5.6:** module=`EchoFilter` in `scribe-core::recording`.
- **5.7:** binary=`scribe-mcp`. Tools=`list_transcripts(limit?, since?)`, `get_transcript(id)`. Output fields=`id, title, started_at, duration_seconds, status`. Error=`Transcript not found: <id>`. Menu=`Copy MCP config`. Config key=`mcpServers.scribe.command`.
- **5.8:** tool=`search_transcripts(query, limit?)`. Match shape=`{ at, speaker, text }` under `matches[]`.
- **5.9:** events=`ModelDownloadProgressed`, `ModelDownloadCompleted`, `ModelDownloadFailed`. UI state="Download failed — Retry".
- **5.10:** engine=`parakeet`, language=`auto`.

## 4c. Slice traps
- **5.1:** (1) Do NOT create `Documents/Scribe` with default umask. Set user-only mode explicitly and test it (AC 5.1.3). (2) Do NOT leave a Dock or taskbar window. Set activation policy / skip-taskbar. (3) CI must block merge and run mutation testing on both OSes, not macOS only.
- **5.2:** (1) Closing the window is not an acknowledgement. Persist only on "I understand". (2) Lock down the CSP: bundled assets only, no remote content ([spec §6.2](001-scribe-tray-transcriber.md#62-secure-coding-org-standards-owasp-derived) 6.2.5). (3) Final wording is pending legal review ([spec §9](001-scribe-tray-transcriber.md#9-open-questions) 9.1). Keep it in one place so it is easy to swap.
- **5.3:** (1) Do NOT buffer audio to temp files, not even for chunked fallback (invariant 4.2). (2) Append finalised segments and flush per segment. Never rewrite earlier lines. Only front matter changes, and only on finalise or incomplete-marking (4.5). (3) Never log segment text, only session ids (5.3.11). (4) No `#[cfg(target_os)]` and no `std::fs` in `scribe-core`. File I/O lives in `store-markdown`. (5) Recorder state must not be a global singleton. On permission denial, create no file (5.3.2).
- **5.4:** (1) Exclude Scribe's own process from the tap (4.8). (2) Detect exact-zero buffers for 5 s and emit `CaptureSilenced`. Do not treat it as silence. (3) Timestamp both lanes against one session clock, not per-device clocks (≤ 1 s drift over 60 min).
- **5.5:** (1) Announce only finalised segments in a polite live region. Never announce partials. (2) Mark Me/Them by text label, not colour alone. (3) Closing the popover must never stop or pause recording. Honour `prefers-reduced-motion`.
- **5.6:** (1) Drop only Me segments, never Them. (2) Hold-back is capped at 1.5 s. Do not wait indefinitely for a matching Them segment. (3) Text match alone is not enough: the segments must also overlap within ±1 s.
- **5.7:** (1) Reject ids containing a path separator or `..` **before** any filesystem call. (2) Never return `status: recording` sessions. (3) `scribe-mcp` must not depend on `audio-*` or `stt-*` crates and must not write files. A byte-identical folder test proves this. (4) Skip malformed files and log the file name only. Cap stdio messages at 1 MB.
- **5.8:** (1) Use literal, case-insensitive substring matching. Never compile `query` as a regex. (2) Reject empty or whitespace-only queries. (3) The < 1 s budget for 500 hours of transcripts rules out re-parsing heavy structures per call. Stream lines.
- **5.9 (destructive: deletes a hash-mismatched file):** (1) Delete only the file that failed SHA-256, inside the app-data model dir, and never a user path. (2) The project has **no** confirmation or impact-count convention. None is needed here because the action is automatic, and none should be invented. (3) Never disable TLS certificate verification. Resume partial downloads and do not restart them. Download into per-user app data.
- **5.10:** (1) Keep domain code free of Windows branches. Add adapters only. (2) Reopen loopback on default-device change within 3 s. (3) Resolve the Parakeet model choice by spike first ([spec §9](001-scribe-tray-transcriber.md#9-open-questions) 9.2). VAD-chunk fallback must not write audio to disk.

## 5. Known dragons
No code yet, so there are no TODO, FIXME or LEGACY markers. Dragons come from the spec and external sources:
- **SpeechAnalyzer `start(inputSequence:)` nilError on macOS 26.3** ([spec §9](001-scribe-tray-transcriber.md#9-open-questions) 9.3). Check at the start of 5.3. The fallback is short chunks, in memory only.
- **Core Audio tap fails silently** with zero buffers when `NSAudioCaptureUsageDescription` is missing. Permissions are tied to the code signature, so unsigned dev builds may behave differently (6.5.4).
- **Signing identities are not yet available** (9.5). Verification for 5.4 and 5.10 needs signed builds.
- **Forbidden list** (`docs/ARCHITECTURE.md` §7): `#[cfg(target_os)]` in core, Tauri types below `apps/`, `scribe-mcp` depending on capture or STT, domain touching the filesystem, shared mutable recorder singletons, audio on disk.

## 6. Test conventions
None are established in the repo. These are mandated by [spec §6.4](001-scribe-tray-transcriber.md#64-test-integrity--delivery-org-standards):
- **Rust unit:** domain tested with fake port adapters. Location and harness are not yet chosen; slice 5.1 decides and records them. `cargo test` is the Rust default.
- **Adapter integration:** thin tests behind a platform gate for OS capture and engines.
- **Frontend:** files end in `*.spec.ts`. The runner is not yet chosen; slice 5.1/5.2 decides.
- **Behavioural checks required by ACs:** a no-audio-on-disk test and a no-egress test (5.3.10), a folder-mode test (5.1.3), and an MCP read-only byte-identical test (5.7.4).
- **Naming:** business-language test names, ZOMBIES case derivation, every test asserts.
- **CI:** macOS + Windows runners. Mutation testing comments on PRs (tool not chosen). Merge is blocked on red.
- **Process:** Conventional Commits, `<type>/<kebab-case>` branches, merge (not rebase).
- **Active packs** (per slice in [spec §5](001-scribe-tray-transcriber.md#5-slices)):

| Pack | Slices |
|---|---|
| Clean Code | all |
| Secure Coding | 5.1, 5.3, 5.4, 5.7, 5.8, 5.9, 5.10 |
| Accessibility (WCAG 2.2 AA) | 5.1, 5.2, 5.3, 5.5, 5.9 |
| Test Integrity | 5.1, 5.3–5.10 |

No regulated packs (HIPAA, PCI, SOC 2, GDPR) apply.

## 7. Hot paths and performance landmines
- **Segment pipeline** (capture → resample 16 kHz → Transcriber → EchoFilter → store/UI): ≤ 2 s finalised-text latency on M1 8 GB and on x64 4-core CPU-only (6.5.3). Do not block the audio callback with file I/O.
- **EchoFilter hold-back:** adds at most 1.5 s on top of finalisation.
- **`search_transcripts`:** 500 one-hour files in < 1 s.
- **Them-lane recovery** after a device change: ≤ 3 s. Silence detection: 5 s.

## 8. Open uncertainties resolved during orientation
- **Greenfield confirmed** at HEAD `78b4778`. The spec's "every module is new" ([spec §8](001-scribe-tray-transcriber.md#8-codebase-impact-map)) holds. No existing code constrains placement.
- **[spec §9](001-scribe-tray-transcriber.md#9-open-questions) 9.1–9.5 remain open.** The repo holds no information that resolves them.

## 8a. Design system
- **Source of truth:** none detected (greenfield / no-system). There is no `design.md`, Tailwind config, CSS custom properties or theme module. Direction comes from [spec §1](001-scribe-tray-transcriber.md#1-overview--business-context): balanced density, utilitarian, follows system light/dark, subtle motion, Raycast and native menu-bar references.
- **Tokens / theme:** none. Slice 5.2 sets the first tokens (neutral scale, one accent, a recording colour paired with a shape) as CSS custom properties. Slice 5.5 reuses them.
- **Component library:** none. The 5.2 button and window become the primitives that 5.5 and 5.9 reuse.
- **Conventions:** WCAG 2.2 AA. Contrast ≥ 4.5:1 in both modes. Visible focus. No colour-only meaning. Polite live regions. `prefers-reduced-motion`.
- **Per-slice UI note:** **5.5** and **5.9** imitate the 5.2 privacy window once it exists. Tray and menu surfaces (5.1, 5.3) are native, not webview.

---
## Changelog
- 2026-10-17 — generated by code-orienteer from spec c154d03d-7c8a-4c10-8baa-c0d1cc880d19 at HEAD 78b47786a9b8fc0c8a8e8ec8bf7fa10ee1acd904.
