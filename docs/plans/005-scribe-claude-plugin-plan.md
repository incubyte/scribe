# Plan: 005-scribe-claude-plugin

> Spec: none — planned from prompt (standard tier) · Context: [`docs/specs/005-scribe-claude-plugin-context.md`](../specs/005-scribe-claude-plugin-context.md) · Authored: 2026-10-20
> Risk: MODERATE (user-facing output; plugin skills and artifact runtime in Claude Desktop chat are partly unconfirmed; no app code changes, easy to revert) · Hard floors: none · Fabric standards: reachable

---

# Part 1 — Review this

*Everything a reviewer needs to approve or redirect the approach. If you only read one part, read this one.*

## What we're doing

A Claude plugin named `scribe` with two commands, **recap** and **meeting-notes**. It lives in this repo at `plugins/scribe/` and is installable as a marketplace from `incubyte/scribe`. The plugin is only instructions (skills) and one HTML template. It reads calls through the `scribe` MCP server that the tray's **Connect to Claude Desktop** button already registers, so it ships no MCP server, no binary and no Rust change.

**recap** prints a table of recent calls: `#`, inferred title (with the date), a 2–3 line summary, and inferred participants. **meeting-notes** reads one call. It works out who is speaking from the dialogue (the transcript only says `Me` and `Them`), decides what kind of meeting it was, and writes notes in a format that fits that kind. Each note comes with todos, and each todo has one or two AI actions (for example "Draft follow-up email") that run Claude from inside the artifact. Everything is shown in one artifact styled like the Scribe tray.

The constraint that shapes the artifact: Claude can update an artifact it already made **in the same conversation**, but there is no documented way to write into an artifact from a different conversation ([help center](https://support.claude.com/en/articles/9487310-what-are-artifacts-and-how-do-i-use-them)). So "reuse the same artifact" means **one Scribe notebook per conversation**: every meeting-notes run in a chat adds a meeting to that chat's notebook instead of making a new artifact. The README tells users to keep one "Scribe notes" chat. This is a deliberate limit, not an oversight.

## How it breaks down

Each slice can be installed and used on its own. Slice 1 ships recap and the install path, slice 2 adds the notebook, slice 3 makes its todos actionable, slice 4 adds the release zip.

| # | Slice | Delivers | Why this tier | Tier |
|---|-------|----------|---------------|------|
| 1 | Ship the plugin with recap | `plugin.json`, `marketplace.json`, README, the recap skill | Small, well-specified files; the recap wording is mostly reused | `fast` |
| 2 | Write meeting notes into one notebook | meeting-notes skill + notebook template: speakers, meeting type, notes, todos, reuse per chat | Prompt design plus an HTML/CSS template that must look good, in light and dark, accessibly | `excellent` |
| 3 | Run AI actions on todos | Per-todo action buttons calling Claude in the artifact; ticked todos remembered | Artifact runtime APIs that are only partly documented | `excellent` |
| 4 | Attach the plugin zip to releases | `release.sh` builds and uploads `scribe-plugin.zip` | One function in an existing script | `fast` |

## Decisions made

**1. Two commands now, nothing more.** recap and meeting-notes cover the asked-for jobs. *Rejected: adding "ask across calls" or "follow-up email" commands now (YAGNI). Both are listed as next candidates in the README.*

**2. Skills, not `commands/*.md`.** In Desktop chat, commands load as skills anyway, and meeting-notes needs a supporting file (the template) that only a skill folder carries. Users type `/recap` or `/meeting-notes`, or just ask ("recap my calls this week").

**3. No `.mcp.json` in the plugin.** A local stdio server in a plugin is ignored in Desktop chat, and the binary path differs per install. When the Scribe tools are missing, each skill says to click **Connect to Claude Desktop** in the Scribe tray and restart Claude Desktop ([context §4c](../specs/005-scribe-claude-plugin-context.md#4c-slice-traps)).

**4. One notebook artifact per conversation.** On each run, meeting-notes checks for a Scribe notebook artifact it already made in the conversation. If one exists it rewrites that artifact with the new meeting added and selected. Otherwise it creates the notebook. *Rejected: an artifact that collects meetings across conversations through its own storage, because a new chat cannot write into another artifact's storage. Also rejected: one artifact per meeting, which the user ruled out.*

**5. The notebook is a fixed template plus a data block.** `notebook.html` holds all the markup, CSS and script. Claude only fills one JSON data island (`<script type="application/json" id="scribe-meetings">`) with every meeting so far. That keeps the look consistent across runs and keeps Claude from redesigning the page each time. All text is rendered with `textContent`, never `innerHTML`, because transcript text is untrusted (SonarJS `disabled-auto-escaping`).

**6. Speakers are inferred and marked as inferred.** `Me` is the user. `Them` can be several people on one lane. The skill names people only from evidence in the dialogue ("Thanks, Priya"), shows how sure it is, and otherwise falls back to "Them (one or more people)". A guess is never shown as fact.

**7. Notes follow a guide, not fixed formats, and are written in two passes.** Pass 1 reads only the transcript and decides what kind of call it was, who the notes are for, what shape serves them, which kinds of action fit (create ticket, draft email, add CRM lead, …) and which screenshots to fetch. Pass 2 fetches those screenshots with `get_screenshot` (spec 004 §5.3, when available), reads each against the nearby transcript, and writes the notes. The notebook keeps a thin shell (look, meeting list, AI-action plumbing, saved ticks), and Claude composes the content from a small block vocabulary. *Rejected: six fixed type→section lists, which gave every executive call and product discussion the same shape. Also rejected: dropping the template entirely, which would regenerate the action and escaping code on every run and make notebook reuse fragile.* Revised 2026-10-20 at the user's request.

**8. AI actions call Claude from inside the artifact.** Each todo carries 1–2 actions, and each action is a short prompt the skill writes ahead of time with the meeting context. The button calls the artifact's built-in Claude call (`window.claude.complete`) and shows the result under the todo with a copy button. Usage counts against the viewer's own plan, and the first use asks for permission. Ticked todos are saved in the artifact's personal storage, keyed by meeting id and todo id. *Rejected: buttons that paste a prompt back into the chat, because no documented artifact-to-chat API exists.*

**9. The plugin has its own version.** `plugin.json` starts at `0.1.0` and is bumped only when the plugin changes, separate from the app version in `tauri.conf.json`. `claude plugin validate --strict` requires a version [verified]. The README states the rule.

## Where this could go wrong

- **The Desktop chat skill can't read the supporting `notebook.html`.** Skills on claude.ai read their bundled files with code execution. If that capability is off, the template is unreadable. Guarded by slice 2's check that the skill says what to do when the template can't be read, and by the README's requirements line.
- **`window.claude.complete` or personal storage is unavailable or named differently.** These names come from Anthropic's artifact environment as quoted publicly, and the help center confirms the capabilities but not the names [inferred]. Guarded by slice 3's check that the notebook still renders and stays usable with both missing (buttons disabled, with a one-line reason).
- **The notebook loses earlier meetings on update.** Claude rewrites the data island and could drop a meeting. Guarded by slice 2's check that the skill carries every existing meeting forward unchanged.
- **Recap gets slow or too large.** One `get_transcript` per call. Guarded by slice 1's default scope of today and yesterday and a maximum of 15 calls.
- **A guess shown as a name.** Guarded by slice 2's inferred-name check.

## What's out of scope

Collecting meetings across conversations. Real speaker diarization. Any Rust, MCP server or tray change. Plugin evals (`claude plugin eval`), since there is no mock of the user-installed `scribe` server yet. A LICENSE file (the repo has none even though `Cargo.toml` says MIT, which is flagged for the user to decide). Publishing to the public plugin directory.

---

# Part 2 — Build detail

*Prescriptive detail for the build agent. Reviewers can skim or skip.*

**Confidence markers used below:** `[verified]` = the planner read the code or docs and confirmed it. `[inferred]` = pattern-matched from a sibling implementation, not confirmed — the build agent should check before assuming.

## Shared conventions (all slices)

- **Architecture:** the plugin is a consumer outside the hexagon. It depends only on the `scribe` MCP tool contract ([context §3](../specs/005-scribe-claude-plugin-context.md#3-reusable-utilities)). No file under `apps/` or `crates/` changes, apart from `scripts/release.sh` in slice 4.
- **Dependency direction:** skill → `scribe` MCP tools. Template ← data island written by the skill. The template never calls Scribe.
- **Lexicon ([context §4b](../specs/005-scribe-claude-plugin-context.md#4b-slice-lexicon) — do not rename):** server `scribe`; tools `list_transcripts`, `get_transcript`, `search_transcripts`; fields `id`, `title`, `started_at`, `duration_seconds`, `status`; speakers `Me`, `Them`; button `Connect to Claude Desktop`; product `Scribe`. New names: plugin `scribe`, skills `recap` and `meeting-notes`, template `notebook.html`, data island id `scribe-meetings`, artifact title `Scribe notebook`.
- **Voice:** plain second person, privacy-forward, matching the server instructions and the README ([context §2](../specs/005-scribe-claude-plugin-context.md#2-patterns-to-follow)). Each SKILL.md stays under about 150 lines.
- **Tests:** there are no automated tests for markdown or JSON ([context §6](../specs/005-scribe-claude-plugin-context.md#6-test-conventions)). Every slice is verified with `claude plugin validate --strict plugins/scribe`, `claude plugin validate .` for the marketplace, and `jq .` on each JSON file. Skills are also run once in Claude Code with `claude --plugin-dir ./plugins/scribe` against real transcripts. Slices 2 and 3 also open `notebook.html` with sample data in a browser. `cargo test` and `npm test` must stay green with nothing changed.
- **Standards:** WCAG 2.2 AA contrast and visible focus (Design Standards: Global Component), no disabled escaping (SonarJS `disabled-auto-escaping`), no table used for layout (SonarJS `no-table-as-layout`; the recap table and the notebook's data are real tabular data or lists), and `rel="noopener noreferrer"` on any `target="_blank"` link. No code comments; names carry meaning.

---

## Slice 1 — Ship the plugin with recap

**Key decision:** Create the plugin shell, the repo-root marketplace and the recap skill, so the plugin installs and `/recap` works end to end (decisions 1–3, 9). Recap lists calls, reads each one, and prints one markdown table.

**Structure**

- `plugins/scribe/.claude-plugin/plugin.json` (new): `name` `scribe`, `displayName` `Scribe`, `version` `0.1.0`, `description` (one sentence about recaps and meeting notes from Scribe's local transcripts), `author` `{ name: "Incubyte" }`, `license` `MIT`, `homepage` and `repository` `https://github.com/incubyte/scribe`. `version` is required by `--strict` [verified].
- `.claude-plugin/marketplace.json` (new, repo root): `name` `scribe`, `owner` `{ name: "Incubyte" }`, `plugins: [{ name: "scribe", source: "./plugins/scribe", description }]` [inferred schema — confirm with `claude plugin validate .` and fix any field it rejects].
- `plugins/scribe/skills/recap/SKILL.md` (new): frontmatter `name: recap` and a `description` that triggers on "recap my calls", "what calls did I have", "summarise this week's meetings". Body sections: *When the Scribe tools are missing*, *Choose the calls*, *Read each call*, *Write the table*, *Rules*.
- `plugins/scribe/README.md` (new, at least 40 words): what it does, requirements (Scribe installed and connected through **Connect to Claude Desktop**, Claude Desktop restarted, a paid plan for AI actions, and code execution on for the notebook template), install (Customize → Plugins → Add marketplace `incubyte/scribe`; Claude Code `/plugin marketplace add incubyte/scribe` then `/plugin install scribe@scribe`; or upload the release zip), the two commands, the version-bump rule (decision 9), and next candidate commands. Link to the root README's "Connect an AI app" section instead of repeating it.

**Collaborators:** the `scribe` tools `list_transcripts` and `get_transcript` only.

**Control flow:** missing tools → tell the user to click **Connect to Claude Desktop** and restart, then stop. Pick the scope: no argument means today's and yesterday's calls (`since` = yesterday, `limit` 15); "last N" means `limit` N, capped at 15; a date phrase becomes `since` `YYYY-MM-DD`, computed from today. Call `list_transcripts` with an explicit `limit`. If the list is empty, say "No calls recorded yet" (or none since that date) and stop. Otherwise call `get_transcript` once per id, oldest-to-newest order not needed, and keep the list's newest-first order. For each call, infer a 3–6 word title and show it with the date and time from `started_at`. Write a 2–3 sentence summary. List participants as `Me` plus names heard in the dialogue, each marked "(inferred)", else "Them". Mark `incomplete` calls with "(incomplete)" instead of hiding them. Output one table `| # | Meeting | Summary | Participants |`, then one line offering `/meeting-notes` for any row number.

**Tests:** none automated (shared conventions). Manual: validate both manifests, then run `/recap`, `/recap last 3` and "recap my calls since Monday" against real transcripts.

**Standards that bite here:** `Clean Code: SRP` per skill section; [context §4c](../specs/005-scribe-claude-plugin-context.md#4c-slice-traps) slice B traps.

**Quality checks**

- `claude plugin validate --strict plugins/scribe` and `claude plugin validate .` both pass.
- No `.mcp.json`, `bin/`, hooks or binaries anywhere under `plugins/scribe/`.
- **The recap skill never shows the stored `title` as the meeting title; titles are inferred from content.**
- `limit` is always passed explicitly, the default scope is today and yesterday, and the cap is 15 calls.
- Participant names other than `Me` are always labelled inferred.
- Every tool, field and button name matches the context lexicon verbatim.

---

## Slice 2 — Write meeting notes into one notebook

**Key decision:** Add the meeting-notes skill and the `notebook.html` template (decisions 4–7). The skill picks a call, infers speakers and meeting type, writes notes and todos as one meeting object, and puts it into the conversation's single Scribe notebook artifact. Slice 2 renders todos as a checklist with their action labels visible but disabled. Slice 3 makes them run.

**Structure**

- `plugins/scribe/skills/meeting-notes/SKILL.md` (new): frontmatter `name: meeting-notes`, with a `description` that triggers on "meeting notes for…", "notes from my last call", "write up the call with…". Body sections: *When the Scribe tools are missing*, *Pick the call*, *Work out who spoke*, *Decide the meeting type*, *Note formats* (one short section list per type), *Todos and AI actions*, *Build or update the notebook*, *Rules*.
- `plugins/scribe/skills/meeting-notes/notebook.html` (new): a self-contained page with inline CSS and script, and no external fonts, CDN or network ([context §4c](../specs/005-scribe-claude-plugin-context.md#4c-slice-traps)). It contains the data island `<script type="application/json" id="scribe-meetings">` holding a `Meeting[]`.
- **Meeting object** (the island's schema, documented in the skill as a field list, not code):
  - `id`: the transcript `id`.
  - `title`: inferred.
  - `started_at` and `duration_seconds`: from the list.
  - `type`: a short free label such as "Product discussion" ("Meeting" when empty).
  - `speakers[]`: `{ label, name, confidence: "stated" | "likely" | "unknown", evidence }`.
  - `summary`: a string.
  - `blocks[]`: each `{ kind, … }` with kind one of `heading`, `paragraph`, `list`, `decision`, `metric`, `quote`, `table`, `screen`, `callout`; unknown kinds are skipped.
  - `todos[]`: `{ id, text, owner, due, actions[] }`.
  - Each action is `{ label, prompt }`.
- **Template rendering functions** (names fixed, bodies the builder's):
  - `readMeetings(): Meeting[]`.
  - `renderMeetingList(meetings, selectedId)`: a sidebar, or a select element at narrow widths.
  - `renderMeeting(meeting)`.
  - `renderSpeakers(speakers)`.
  - `renderSections(sections)`.
  - `renderTodos(meeting)`.
  - `selectMeeting(id)`.

**Collaborators:** the `scribe` tools `list_transcripts`, `get_transcript` and `search_transcripts`. The artifact is created or updated with Claude's own artifact mechanism in chat.

**Control flow:**
1. If the tools are missing, give the same message as recap and stop.
2. Pick the call. With no argument, take the latest call. A recap row number refers to the recap earlier in the chat. With a name or topic, run `search_transcripts` with 1–3 short keywords. If several calls match, list up to 5 and ask which one.
3. Read the call with `get_transcript`.
4. Infer the speakers per decision 6.
5. Classify the type, falling back to `general`.
6. Write the summary and the type's sections.
7. Write 2–6 todos. Each has an owner (`Me`, an inferred name, or "Unassigned") and 1–2 actions whose `prompt` includes the todo, the owner and the context the action needs from the meeting.
8. Build the notebook:
   - If the template cannot be read, explain that the notebook needs code execution turned on, then give the notes as plain markdown.
   - If a Scribe notebook artifact exists in this conversation, take its current meetings, replace any meeting with the same `id` (otherwise append), and put the newest first. Then rewrite that same artifact with the template and the full array, with the new meeting selected.
   - Otherwise create the artifact titled `Scribe notebook` from the template.
9. Reply with one line naming the meeting added and how many meetings the notebook holds.

**Template behaviour:**
- **Layout.** Sticky header with "Scribe notebook". Meeting list ordered newest first, showing the inferred title and the muted date and duration. Main pane shows a type badge, speakers as chips with "(inferred)" and the evidence on hover or focus, summary, sections, and todos as a checklist using the `.checklist__item` look. The user's own todos get the `.line--me` accent rule.
- **Theme.** Tray tokens copied as CSS custom properties, with light and dark through `prefers-color-scheme` ([context §8a](../specs/005-scribe-claude-plugin-context.md#8a-design-system)). Honour `prefers-reduced-motion`.
- **Accessibility.** Proper heading levels. List markup for meetings and todos. Real `<button>`s and checkboxes with labels. Visible `:focus-visible` ring. Meaning never shown by colour alone.

**Tests:** none automated. Manual:
- Open `notebook.html` in a browser with a 3-meeting sample island in light and dark. Check keyboard-only navigation and AA contrast with the browser's accessibility checker.
- In Claude Code with `--plugin-dir`, run meeting-notes twice on different calls and confirm the second run keeps the first meeting.

**Standards that bite here:**
- SonarJS `disabled-auto-escaping`: only `textContent` and created elements, never `innerHTML` with data.
- WCAG 2.2 AA contrast and focus.
- `no-table-as-layout`.
- `Clean Code: SRP` per render function.

**Quality checks**

- **No `innerHTML`, `outerHTML`, `insertAdjacentHTML` or `document.write` with meeting data anywhere in `notebook.html`.**
- The template loads nothing over the network: no `<link>`, external `src`, `@import` or `fetch`.
- The skill tells Claude to carry every existing meeting forward unchanged when it updates the notebook.
- The skill works in two passes, and fetches only the screenshots it chose in pass 1 (at most 8). When no screenshots or no `get_screenshot` exist, it skips that step silently.
- Every speaker name other than `Me` carries a confidence value, and "unknown" is rendered as "Them".
- Every todo has 1–2 actions, each with a non-empty `label` and `prompt`.
- Light and dark both pass AA contrast for text and controls, and every control has a visible focus state.
- SKILL.md is under about 150 lines, and the template is not inlined in it.

---

## Slice 3 — Run AI actions on todos

**Key decision:** Turn the disabled action labels into working buttons that call Claude from inside the artifact, and remember which todos are ticked (decision 8). The notebook must stay fully readable when either runtime API is missing.

**Structure** (`plugins/scribe/skills/meeting-notes/notebook.html`, modify):

- `canAskClaude(): boolean`: true when `window.claude?.complete` is a function [inferred API name].
- `runAction(meeting, todo, action, outputElement): Promise<void>`.
- `renderActionResult(outputElement, text)`: the result as plain text with a **Copy** button that uses `navigator.clipboard.writeText`.
- `loadTicked(meetingId): Promise<Set<string>>` and `saveTicked(meetingId, ticked): Promise<void>`: these use `window.storage.get`/`set` with personal (non-shared) scope, under the key `ticked:<meetingId>` [inferred API name].
- `SKILL.md` (modify): the *Todos and AI actions* section says what makes a good action prompt (self-contained, names the output form, for example "a 5-line email to …", and contains only meeting facts needed for it).

**Collaborators:** the artifact runtime only. The template still never calls Scribe.

**Control flow:**
- **Clicking an action:**
  - If `canAskClaude()` is false, the button stays disabled with the visible reason "AI actions need Claude web or Desktop on a paid plan."
  - Otherwise, set the button busy (`aria-busy`, label "Working…") and call `window.claude.complete` with `action.prompt`.
  - On success, render the result.
  - On failure, show "Couldn't run this action. Try again." in the output area. The output area is `aria-live="polite"`.
  - Always restore the button afterwards.
- **Ticking a todo:** update the view at once, then save. If storage is missing or throws, keep the in-memory state and show nothing alarming.
- **On load:** read the ticked set for the selected meeting, with an empty set on any failure.

**Tests:** none automated. Manual:
- Open the template in a plain browser, where both APIs are missing. Buttons are disabled with the reason shown, and the page has no console errors.
- In Claude Desktop, upload the plugin zip and run meeting-notes. Run one action, tick a todo, then reopen the artifact and confirm the tick is kept.

**Standards that bite here:**
- `Clean Code`: fail at the boundary and catch only where you can recover (each API call).
- SonarJS `disabled-auto-escaping` for results, which are plain text via `textContent`.
- WCAG: status messages are announced, and busy state is not shown by colour alone.

**Quality checks**

- **With `window.claude` and `window.storage` both undefined, the notebook renders every meeting and logs no errors.**
- An action result is inserted only as text, never parsed as HTML.
- Storage uses personal scope, never shared.
- Each button disables itself while its action runs, so one click can't start two requests.

**Open at build time:** the exact runtime names `window.claude.complete` and `window.storage` (with get, set and personal scope) can only be confirmed inside a Claude Desktop artifact. Probe by running the slice 3 manual Desktop check. If either name differs, ask the Desktop Claude what its artifact runtime calls them, and update `canAskClaude`, `loadTicked` and `saveTicked` only.

---

## Slice 4 — Attach the plugin zip to releases

**Key decision:** Each GitHub Release also carries `scribe-plugin.zip`, the fallback install by upload, built from `plugins/scribe` so the zip's root holds `.claude-plugin/plugin.json`.

**Structure**

- `scripts/release.sh` (modify):
  - New function `build_plugin_zip(): path`, which zips the contents of `plugins/scribe` into the release output directory, excluding `.DS_Store`.
  - `publish_assets` gets the zip appended to its `gh release upload` list ([context §4](../specs/005-scribe-claude-plugin-context.md#4-adjacent-code-that-must-coordinate-per-slice)).
- `plugins/scribe/README.md` (modify): the upload-install step names `scribe-plugin.zip` from the latest release.

**Collaborators:** the existing `publish_assets` flow. Release ordering and tagging are unchanged.

**Control flow:** run `claude plugin validate --strict plugins/scribe` before zipping when the `claude` CLI is present, and skip the check with a printed note when it is not. A failed validation aborts the release before any tag or upload.

**Tests:** none automated. Manual: run `build_plugin_zip` alone and list the zip contents. `.claude-plugin/plugin.json` must be at the root, and there must be no `plugins/scribe/` prefix.

**Standards that bite here:** keep `release.sh`'s existing style (`set -euo pipefail`, function per step).

**Quality checks**

- **The zip's root contains `.claude-plugin/plugin.json` and `skills/` directly.**
- The release still uploads the `.dmg` and the Windows installer unchanged.
- An invalid plugin stops the release before tagging.

---

## Hard floors

None. No slice touches auth, authorization, payment, security/crypto or migrations. The plugin reads local transcripts only through the existing MCP tools.

## Escalation log
