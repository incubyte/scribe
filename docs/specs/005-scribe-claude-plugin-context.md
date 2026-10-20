# Task context: 005-scribe-claude-plugin

> Generated for plan 005 "scribe-claude-plugin" (planned from prompt, no spec) at HEAD 7d3481e5711be402ee0906eef95a096df66fc5d5 · 2026-10-20T05:17:20Z
> Read first by every agent in /anthara:plan-implementer's slice loop.

**No spec file exists**, so this file cites no spec anchors. The slices below follow the natural seams of the brief. If the plan slices the work differently, map onto these.
- **A** plugin scaffold: `plugins/scribe/.claude-plugin/plugin.json`, `plugins/scribe/README.md`, repo-root `.claude-plugin/marketplace.json` (source `./plugins/scribe`).
- **B** `plugins/scribe/skills/recap/SKILL.md`: a table of recent calls (index, inferred title, summary, participants).
- **C** `plugins/scribe/skills/meeting-notes/SKILL.md` plus an HTML artifact template beside it: infer speakers, classify the meeting type, write notes and todos with per-todo AI actions, styled like the tray.

**De-facto style:** Hexagonal (ADR 1 in `docs/ARCHITECTURE.md`). The plugin is a new **consumer** outside the hexagon. It talks only to the existing `scribe-mcp` adapter over MCP, so no Rust, crate or tray change is in scope. `plugins/` and `.claude-plugin/` do not exist yet, so this is greenfield. There is no LICENSE file.

## 1. Affected modules
| Module / file | Role | Notes |
|---|---|---|
| `apps/scribe-mcp/src/server.rs` (`ScribeServer`) | The three tools and the server `name`/`instructions` | Read only. It is the contract the skills depend on |
| `apps/scribe-mcp/src/requests.rs` | Request and response DTOs (`ListTranscriptsRequest`, `TranscriptSummaryDto`, `SearchHitDto`, `SegmentMatchDto`) | Use the field names exactly as written there (see 4b) |
| `crates/store-markdown/src/format.rs` (`render_header`, `render_segment`) | On-disk markdown that `get_transcript` returns as is | Front matter plus one line per segment |
| `crates/scribe-core/src/recording/session.rs` (`TITLE_FORMAT`, `SessionStatus::as_str`, `STAMP_FORMAT`) | Default title, status strings, id shape | No code renames a call, so every title is the default |
| `apps/scribe-tray/src/theme/tokens.css`, `primitives.css`, `popover.css` | Visual source for the artifact | Copy the values into the template. Do not import the files (see 8a) |
| `apps/scribe-tray/src/popover/PopoverActions.tsx` (`CONNECT_LABEL`, `UPDATE_LABEL`) | Button labels that the README and skills tell users about | |
| `README.md` § "Connect an AI app" | The current user-facing connect instructions | The plugin README should point here, not repeat it |
| `plugins/scribe/**`, `.claude-plugin/marketplace.json` | New files | Greenfield |

## 2. Patterns to follow
- **The MCP contract is the only interface.** Skills call `list_transcripts`, `get_transcript` and `search_transcripts`. They never read `~/Documents/Scribe/**/*.md` directly, because the folder can be moved (`TranscriptsFolder::configured_in`, which `SettingsFollowingStore` re-reads on every call).
- **Voice from the server instructions.** "Scribe records the user's calls and meetings on this computer… the user's lines marked Me and the other side marked Them… Nothing here leaves the computer." Use the same plain, second-person, privacy-forward wording in the skill descriptions.
- **README tone.** Short imperative steps with **bold** UI labels, for example "click **Connect to Claude Desktop** in the popover, then restart Claude Desktop". The plugin README should match it.
- **Design.** Path A, existing system. Mirror the tray tokens and do not invent a palette (8a).
- AI-ergonomics: keep each SKILL.md under about 150 lines. Put the HTML template in its own file next to the SKILL.md and reference it by relative path. Do not inline it.

## 3. Reusable utilities
- `apps/scribe-mcp/src/server.rs` (`list_transcripts`): newest first. Takes `limit` (1-100, default 20) and `since` (`YYYY-MM-DD`). Calls with status `recording` are left out.
- `apps/scribe-mcp/src/server.rs` (`get_transcript`): takes `id` and returns the whole raw markdown as a **text** result, not JSON.
- `apps/scribe-mcp/src/server.rs` (`search_transcripts`): takes `query` (literal, case-insensitive, must not be empty) and `limit`. Returns only the matching lines, not whole calls.
- `crates/scribe-core/src/library/error.rs` (`LibraryError`): the error strings skills may see: "Invalid transcript id: …", "Transcript not found: …", "Search query is empty", and "Invalid since date: … (expected YYYY-MM-DD)" from `requests.rs` (`parse_since`).

## 4. Adjacent code that must coordinate (per slice)
- **Slice A:** `scripts/release.sh` (`publish_assets`) runs `gh release upload "$tag" <files> --clobber`. A plugin zip can be added to that list later. `.github/workflows/release.yml` builds Windows only. `.github/workflows/ci.yml` has `paths-ignore: ["docs/**", "**/*.md"]` and does no JSON or markdown validation.
- **Slice B:** `requests.rs` (`TranscriptSummaryDto`) has no participants or summary. Recap has to call `get_transcript` for each call.
- **Slice C:** `format.rs` (`render_segment`) gives the line shape that speaker inference parses. `popover.css` (`.line`, `.line--me`, `.session`) is the visual precedent.

## 4a. Slice imitation targets
- **Slice A:** greenfield, with no plugin or marketplace in the repo. Take the version from `apps/scribe-tray/src-tauri/tauri.conf.json` (`version`, currently `0.1.2`), the license from root `Cargo.toml` (`[workspace.package] license = "MIT"`), the author from git (`Sapan Parikh`, plus `Incubyte`, which matches `identifier: com.incubyte.scribe`), and the repo URL `https://github.com/incubyte/scribe`.
- **Slice B:** mirror the tray's recent-calls list, `popover.css` (`.session`, `.session__title`, `.session__meta`): title, then muted tabular metadata.
- **Slice C:** mirror the transcript view, `popover.css` (`.line`, `.line--me`, `.line__speaker`), and the button pair `primitives.css` (`.button`) / `popover.css` (`.actions__primary`) for the per-todo actions.

## 4b. Slice lexicon
- **All slices:** server=`scribe` (`mcp_config.rs` `SCRIBE_SERVER_NAME`, under `mcpServers`). Tools: `list_transcripts`, `get_transcript`, `search_transcripts`. Claude Code sees them as `mcp__scribe__list_transcripts` and so on. Speaker labels are exactly `Me` and `Them`. Product name `Scribe`. Button labels: `Connect to Claude Desktop`, `Update Claude Desktop connection`, and the status text `Connected to Claude Desktop`. Other button: `Copy MCP config`. Claude Code install command: `claude mcp add scribe -s user -- <path to scribe-mcp>`.
- **Slice B:** list fields `transcripts[]` → `id`, `title`, `started_at` (RFC 3339 with offset), `duration_seconds` (number or null), `status` (`complete` | `incomplete`). Request fields: `limit`, `since`.
- **Slice C:** front matter keys, in order: `id`, `title` (quoted), `started_at`, `ended_at` (empty while recording), `status`, `engine`, `language`. Segment line: `[HH:MM:SS] **Me:** text`, where the clock is the offset from `started_at`, not wall time. Search hit fields: `transcripts[]` → `id`, `title`, `started_at`, `matches[]` → `at`, `speaker`, `text`.
- **IDs:** `YYYY-MM-DD-HHMMSS`, with an optional `-N` suffix when two calls collide (`STAMP_FORMAT`, `with_collision_suffix`). Default title is `Call YYYY-MM-DD HH:MM` (`TITLE_FORMAT`).

## 4c. Slice traps
- **Slice A:** (1) Do NOT bundle a `.mcp.json` that launches `scribe-mcp`. The binary sits inside the app bundle at a path that differs per install, and the tray already owns that registration. Tell users to press **Connect to Claude Desktop** instead. (2) Do NOT invent a LICENSE id other than `MIT`, and do not add a LICENSE file as part of this work. (3) Do NOT put the plugin version in a third place that has to be bumped by hand without saying so in the README. `tauri.conf.json` is the release version, and `package.json` already lags it (0.1.1). (4) CI ignores `*.md` and does not lint JSON. Check `plugin.json` and `marketplace.json` by hand, for example with `jq . <file>`.
- **Slice B:** (1) Do NOT show `title` as the inferred title. It is always the default `Call …` stamp, so infer a title from the content and show the date separately. (2) Do NOT claim participants' names as fact. Only `Me` and `Them` exist, so label inferred names as inferred. (3) Pass `limit` explicitly, because the default is 20 and the maximum is 100. Calling `get_transcript` once per call is expensive, so keep the recap small (about 5-10). (4) Mark `incomplete` calls rather than hiding them.
- **Slice C:** (1) `Them` can be several remote people mixed on one lane (README "Status": separating multiple remote speakers is unsolved). Infer people from the dialogue and admit when unsure. Never present a guess as certain. (2) `[HH:MM:SS]` is the offset into the call. Wall time is `started_at` plus the offset. (3) The artifact has to be **one reusable** file. Update the same artifact on later runs instead of creating a new one per meeting. (4) The template must be self-contained, with inline CSS, no external fonts, CDN or network calls, to respect "Nothing here leaves the computer". (5) Per-todo AI actions must be real buttons with visible focus (`--focus-ring`) and must not depend on colour alone.

## 5. Known dragons
- `get_transcript` returns the raw file, and the transport's message cap is `transport.rs` (`MAX_MESSAGE_BYTES` = 1 MiB). Very long calls are large, so skills should read one call at a time.
- Search is a literal substring match (`SearchQuery::matches`), not semantic search. Skills should try several short keywords, not sentences.
- `SettingsFollowingStore` re-reads the settings on every list or read. That is harmless here, but it means the transcripts folder can change between calls.
- There is no LICENSE file in the repo, even though the README and `Cargo.toml` say MIT. Flag this to the user, and do not create one unasked.

## 6. Test conventions
- Markdown and JSON have no automated tests, and CI skips `**/*.md`. Check the work by hand: `jq . plugins/scribe/.claude-plugin/plugin.json .claude-plugin/marketplace.json`, then install the plugin locally with `/plugin marketplace add ./` in Claude Code and run both skills against real transcripts.
- The tool contract is covered by `apps/scribe-mcp/tests/mcp_tools.rs`. If a skill relies on a field, that test is where the field is guaranteed. Do not modify it.

## 7. Hot paths and performance landmines
- Recap calls `get_transcript` N times, which grows the context with call length. Read only what is needed for the summary and participants.

## 8. Open uncertainties resolved during orientation
- Server key: `scribe` (`mcp_config.rs` `SCRIBE_SERVER_NAME`). Button label: `Connect to Claude Desktop`, which becomes `Update Claude Desktop connection` when the connection is outdated. After connecting, the user must restart Claude Desktop.
- Release assets: `gh release upload` in `release.sh` (`publish_assets`). A zip can be added there later. Nothing is needed now.
- License: `MIT` (`Cargo.toml` `[workspace.package]`). Author: Sapan Parikh / Incubyte. No `authors` field exists anywhere.

## 8a. Design system
- **Source of truth:** CSS custom properties in `apps/scribe-tray/src/theme/tokens.css`. There is no `design.md`.
- **Tokens, light / dark:** `--surface` `#fafafc` / `#1e1e21`; `--text` `#1d1d1f` / `#f2f2f5`; `--text-muted` `#4a4a50` / `#b9b9c0`; `--accent` `#0f6f6a` / `#5fd3c6`; `--on-accent` `#fff` / `#062b28`; `--accent-soft` (accent at 0.12 / 0.14 alpha); `--hairline` `rgb(0 0 0/.1)` / `rgb(255 255 255/.12)`; `--surface-sunken`, `--surface-hover`; `--recording` `#c9302c` / `#ff6b62`. Spacing `--space-1..5` = 4/8/12/16/24px. Radii: `--radius-window` 12px, `--radius-control` 8px. Fonts: `--font-body` (system: -apple-system, Segoe UI Variable) and `--font-figures` (ui-rounded). Sizes: `--text-sm` 12, `--text-md` 13, `--text-lg` 15. Body `line-height: 1.45`. Title 18px / 650 weight.
- **Primitives:** `primitives.css` (`.button`: accent fill, 32px min-height, 8px radius, `:focus-visible` ring; `.checklist__item`: accent dot bullet, a good fit for todos). `popover.css` (`.actions__primary`: soft accent with hairline border; `.session`; `.line--me`: 2px accent left border).
- **Conventions:** dense, macOS-native feel. Dark mode via `prefers-color-scheme` (do it with real token swaps, as the tray does). Honour `prefers-reduced-motion` and `prefers-reduced-transparency`. Use `color-scheme: light dark`. Contrast should reach WCAG AA.
- **Per-slice UI note:** **Slice C:** reuse the `.line--me` accent rule to mark the user's own todos, and `.session__meta` (muted, tabular-nums) for timestamps.

---
## Changelog
- 2026-10-20: generated by code-orienteer from the prompt (no spec) at HEAD 7d3481e.
