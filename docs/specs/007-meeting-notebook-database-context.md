# Task context: 007-meeting-notebook-database

> Generated for spec d0922bbc-4934-4cef-b35c-07e9cc016c5f at HEAD 8339713abf37794c66fc84255f47d35649c843e2 · 2026-10-21T11:26:45Z
> Read first by every agent in /anthara:plan-implementer's slice loop.

**De-facto style:** The plugin is a consumer outside the Rust hexagon (ADR 1, `docs/ARCHITECTURE.md`, which does not mention the plugin). It has no code apart from `notebook.html`. Today the template is one flat procedural inline script: a global `state` object, about 70 top-level render and helper functions, and two runtime touchpoints, `resolveAskClaude` (ask-Claude) and `loadTicked`/`saveTicked` (storage). [spec §7.2](007-meeting-notebook-database.md#72-architectural-style) adds hexagonal seams inside that one script: a store adapter, an ask-Claude adapter and a pure core. Nothing in the repo tests the notebook or the plugin today.

## 1. Affected modules
| Module / file | Role | Notes |
|---|---|---|
| `plugins/scribe/skills/meeting-notes/notebook.html` | The whole page: CSS tokens, markup shell, inline script | Slices 5.1, 5.3-5.6. 5.7 republishes it. 1205 lines, kept as one file on purpose ([spec §9](007-meeting-notebook-database.md#9-deferred-follow-ups-and-accepted-limits) 9.5) |
| `plugins/scribe/skills/meeting-notes/SKILL.md` | Skill prose: pick the call, speakers, notes, todos, then "Build or update the notebook" | Slices 5.2 and 5.7 rewrite the notebook section and *Rules*. 120 lines now; plan 005 capped it at about 150 |
| `plugins/scribe/skills/meeting-notes/guide.md` | Note-shaping examples | Unchanged by this spec |
| `plugins/scribe/README.md` | Requirements, install, commands, **Versions** rule | 5.2/5.7 change what `/meeting-notes` does, and a published-artifact Claude becomes a requirement |
| `plugins/scribe/.claude-plugin/plugin.json` | Plugin version (`0.1.0`) | Bump with every plugin change (see §5) |
| `.claude-plugin/marketplace.json` | Marketplace entry | `description` repeats `plugin.json`'s `description`. Change both together |
| `README.md` (root) § "Install the Claude plugin" | User-facing summary of the two commands | Update it if the command wording changes |
| `scripts/release.sh` (`validate_plugin`, `build_plugin_zip`) | `claude plugin validate --strict` on the plugin and the root, then a zip of everything under `plugins/scribe` | Any file put under `plugins/scribe` ships in `scribe-plugin.zip` |
| `.github/workflows/ci.yml` (`portable` job) | The only CI that runs JS: `npm ci`, `npm run lint` and `npm test` in `apps/scribe-tray` | Nothing in CI touches `plugins/` |
| `apps/scribe-tray/vite.config.ts`, `src/test-setup.ts`, `stryker.config.json`, `eslint.config.js`, `tsconfig.json` | The only JS test tooling in the repo | Likely harness for NFR 6.3.1 tests (see §6) |
| `docs/plans/005-scribe-claude-plugin-plan.md`, `docs/specs/005-scribe-claude-plugin-context.md` | Earlier decisions about the notebook | Decision 4 is superseded, decisions 5 and 8 are partly superseded (see §8) |

## 2. Patterns to follow
- **Build DOM as text only.** Use `element(tagName, className, text)`, which sets `textContent`. Never use `innerHTML`, `insertAdjacentHTML` or `outerHTML`. Defined in `notebook.html` (`element`). Used by every `render*`. This covers [spec §4](007-meeting-notebook-database.md#4-invariants) 4.7 and AC 5.1.6.
- **Read untrusted fields defensively.** `records(list)` keeps only object entries. `textOf(value)`, `meetingTitle` and `meetingTypeLabel` fall back on bad input. `isRunnableAction` filters actions. Records written by viewers are just as untrusted as the old island.
- **Dispatch table for block kinds.** `BLOCK_RENDERERS` is guarded by `Object.hasOwn`, and unknown kinds are skipped silently (`renderBlocks`).
- **User-facing strings are named constants at the top of the script.** Examples: `AI_UNAVAILABLE_REASON`, `ACTION_FAILED_MESSAGE`, `WORKING_LABEL`, `UNTITLED_MEETING`, `FALLBACK_MEETING_TYPE`. New copy should follow suit ("No open todos", "Show older meetings", the unavailable line, the not-saved line).
- **Detect a runtime capability once and treat `null` as unavailable.** `resolveAskClaude` tries `window.claude.complete`, then `window.claude.use("sample")`, and returns `null` when neither exists. `start` passes the result to `enableClaudeActions`, and `canAskClaude` gates the UI. The store adapter should follow the same shape.
- **Async boundary handling.** Event handlers end with `.catch(logFailure)`. Best-effort writes use `ignoreFailure`. `selectMeeting` checks again that `state.selectedId === id` after its `await`, before applying anything. Live snapshot callbacks need the same stale-result check.
- **In-place updates versus a full re-render.** `markSelected` and `applyTicked` patch the existing DOM. `renderMeeting` and `renderMeetingList` replace everything with `replaceChildren`.
- **CSS.** Class names follow BEM (`block__element`, `block--modifier`) and all values come from `:root` custom properties. Light and dark modes swap tokens under `prefers-color-scheme`. The stylesheet also handles `prefers-reduced-motion`, `prefers-reduced-transparency`, `forced-colors` and a `max-width: 640px` layout.
- **Accessibility in place today.**
  - A `.visually-hidden` label sits on the mobile `<select>`.
  - Meeting buttons carry `aria-current`.
  - Disabled actions use `aria-describedby` to point at the reason `<p>`.
  - The action output is `aria-live="polite"`, and failures use `role="alert"`.
  - Busy buttons get `aria-busy` and `aria-disabled`.
  - The header mark is `aria-hidden` and `focusable="false"`.
- **Skill voice.** Plain, second person, imperative steps. The reply is one line. `SKILL.md` *Rules* is a flat list of "Never …" lines.
- AI-ergonomics (advisory): the script has no internal grouping and puts about 70 functions in one global scope. Mark the [spec §7.3](007-meeting-notebook-database.md#73-module-decomposition) adapters and core as clearly named contiguous sections so later agents can find them. If import and upgrade make `SKILL.md` grow past about 150 lines, move that procedure into a sibling file, as `guide.md` does.

## 3. Reusable utilities
- `notebook.html` (`formatStart`, `formatDuration`, `formatWhen`) formats dates and durations for the list, the meeting header and Todos-tab links.
- `notebook.html` (`describeTodo`) builds the "Owner: … · Due: …" meta line and falls back to `Unassigned`.
- `notebook.html` (`renderTodo`, `renderActions`, `buildActionButton`, `runAction`, `renderActionResult`, `buildCopyButton`) is the todo row with its AI actions. Reuse it in the Todos tab.
- `notebook.html` (`speakerDisplayName`, `speakerConfidenceNote`) holds the speaker-label rules.
- `notebook.html` (`renderEmptyState`) is the empty-state card (`.empty`), with the copy "No meetings yet".
- `notebook.html` (`renderMeetingCount`) renders the header count with singular and plural.
- `notebook.html` (`completionText`) normalises the `sample` and `complete` result shapes.
- `apps/scribe-tray/src/test-setup.ts` provides jest-dom matchers, a `matchMedia` stub, `cleanup` and `vi.unstubAllGlobals`.

## 4. Adjacent code that must coordinate (per slice)
- **Slice 5.1:** `notebook.html` (`readMeetings`, `start`, `selectMeeting`, `enableClaudeActions`). The data island `#scribe-meetings` stops being the data source. `SKILL.md` "Build or update the notebook" still describes the island.
- **Slice 5.2:**
  - `SKILL.md`: the "Build or update the notebook" steps, the `capabilities: {sample: {}}` paragraph, and the *Rules* lines "Never create a second notebook in the same conversation" and the `<` escape rule.
  - `plugins/scribe/README.md`: *Requirements* and *Commands*.
  - Root `README.md` § "Install the Claude plugin".
- **Slice 5.3:** `notebook.html` `:root` and its dark, `forced-colors` and 640px media blocks; `.notebook__header` (`--glass` blur); `.notebook__mark`; `.button`.
- **Slice 5.4:** `notebook.html` (`toggleTodo`, `tickedIds`, `applyTicked`, `loadTicked`, `saveTicked`, `tickedStorageKey`, `storedText`, `state.ticked`). The whole `window.storage` path is replaced.
- **Slice 5.5:**
  - The `notebook.html` header markup (`.notebook__header`, `#meeting-count`).
  - `--header-height`, which drives `scroll-padding-top`.
  - `renderTodo`/`renderActions` (their element ids).
  - `selectMeeting`.
- **Slice 5.6:** `notebook.html` (`renderMeetingList`, `buildMeetingSelect`, `buildMeetingButtons`, `renderMeetingCount`).
- **Slice 5.7:** `SKILL.md` (the new import and upgrade sections), `plugins/scribe/README.md` *Commands*, and legacy pages' `<script type="application/json" id="scribe-meetings">` island, read through the artifact `read` tool.
- **Every slice that changes the plugin:** bump `plugins/scribe/.claude-plugin/plugin.json` `version` in the same commit (README *Versions*).

## 4a. Slice imitation targets
- **Slice 5.1:** mirror `resolveAskClaude` + `enableClaudeActions` for the store adapter: probe once in `start`, take `null` as unavailable, then render. Mirror `renderEmptyState` for both the empty state and the unavailable state.
- **Slice 5.2:** mirror the current `SKILL.md` "Build or update the notebook" section: numbered find-or-create steps, then the capabilities paragraph, then the one-line reply.
- **Slice 5.3:** mirror the existing `:root` and `@media (prefers-color-scheme: dark)` token swap in `notebook.html`. Change values, not structure.
- **Slice 5.4:** mirror `selectMeeting`'s await-then-recheck guard and `applyTicked`'s in-place checkbox patch. Mirror `buildActionButton`'s `aria-disabled`/busy lock for the one-write-at-a-time rule (AC 5.4.5).
- **Slice 5.5:** rows mirror `renderTodo`. Group headings mirror `.sidebar__heading`. The empty line mirrors `.empty`. Nothing in the template or the tray has a tab list or a segmented filter yet (searched `role="tab"` and `tablist`), so build those from scratch.
- **Slice 5.6:** mirror `buildMeetingButtons` and `buildMeetingSelect`. The button reuses `.button`.
- **Slice 5.7:** mirror the `SKILL.md` *Pick the call* "list up to 5 and ask which one" interaction for the delete-confirmation offer.

## 4b. Slice lexicon
- **All slices:**
  - Artifact title `Scribe notebook`.
  - Collections `meetings/<transcript id>` and `todos/<meeting_id>--<todo_id>`.
  - Meeting fields `title`, `started_at`, `duration_seconds`, `type`, `summary`, `speakers`, `blocks`, `todo_ids`.
  - Todo fields `meeting_id`, `meeting_title`, `meeting_started_at`, `todo_id`, `text`, `owner`, `due`, `actions`, `done`, `done_at` ([spec §3.2](007-meeting-notebook-database.md#32-kinds-of-data)).
  - Transcript ids look like `YYYY-MM-DD-HHMMSS`, with an optional `-N` suffix. Todo ids are `t1`, `t2` and so on.
  - Owner values are `Me`, a name, or `Unassigned`.
- **Slice 5.1:**
  - Empty state "No meetings yet".
  - Reason constant `AI_UNAVAILABLE_REASON`.
  - Block kinds as in `BLOCK_RENDERERS`: `heading`, `paragraph`, `list`, `decision`, `metric`, `quote`, `table`, `screen`, `callout`.
  - View states: ready, empty, unavailable.
- **Slice 5.2:**
  - Capabilities `db` + `sample`. The current skill declares `capabilities: {sample: {}}` on create and leaves `capabilities` out on update.
  - Reply "Added <title>. The notebook holds N meetings." [spec §5.2](007-meeting-notebook-database.md#52-add-a-meeting-to-the-one-notebook-from-any-chat).
  - A legacy notebook is one that "declares no database".
- **Slice 5.3:**
  - Keep the existing custom-property names (`--surface`, `--text`, `--text-muted`, `--accent`, `--accent-soft`, `--recording`, `--recording-soft`, `--focus-ring`, `--font-body`, `--glass`, `--hairline`, `--control-border`) and re-value them from the [spec §5.3](007-meeting-notebook-database.md#53-wear-the-incubyte-theme) table.
  - Fonts `Fraunces` (headings) and `Inclusive Sans` (body).
- **Slice 5.4:** `done` (boolean) and `done_at` (tick time or `null`).
- **Slice 5.5:**
  - Tabs `Meetings`, `Todos <n open>`.
  - Filter `Open` (default), `Done`, `All`.
  - Groups `Mine`, `Waiting on others`, `Unassigned`.
  - Empty line "No open todos".
- **Slice 5.6:** page size 20 (make it a named constant). Button "Show older meetings".
- **Slice 5.7:** verbs import and update/upgrade. The report gives added and skipped counts. The legacy island id is `scribe-meetings`.

## 4c. Slice traps
- **Slice 5.1:**
  1. Do NOT keep `readMeetings`/the data island as a fallback data path. One code path ([spec §9](007-meeting-notebook-database.md#9-deferred-follow-ups-and-accepted-limits) 9.2).
  2. Do NOT re-run `renderMeeting` (`replaceChildren`) on every snapshot. It wipes keyboard focus, open `<details>` and any AI action result in progress. Patch in place, and keep the selection (AC 5.1.3).
  3. Do NOT issue a write from a render or snapshot callback ([spec §7.2](007-meeting-notebook-database.md#72-architectural-style)).
  4. Treat unavailable as distinct from empty. Catch the probe failure and render one line, with no uncaught rejection (AC 5.1.5).
- **Slice 5.2:**
  1. Do NOT pick the notebook by "exists in this conversation". Find it by title among published artifacts that declare the database. Skip legacy ones even when newer (AC 5.2.3).
  2. Write the meeting, upserts and deletes in one batch. Never write the meeting first and the todos later (AC 5.2.4, 5.2.8: nothing partial).
  3. Carry `done`/`done_at` over for surviving todo ids before the batch. Delete only that meeting's dropped todos, and touch no other meeting ([spec §4](007-meeting-notebook-database.md#4-invariants) 4.4, 4.5).
  4. Retry once on a conflict, then report in one line (AC 5.2.7).
  5. Do NOT read the notebook page on every run to check its version (9.3).
  6. "Never transcribe a sensitive screen" and the other *Rules* still apply. The `<`-escaping rule is specific to the island, so do not apply it to database writes and do not drop it while legacy pages are still read.
  7. Destructive surface (deleting dropped todos): no confirmation pattern or impact-count query exists in this codebase, because the delete is part of the rewrite, so do not invent a prompt.
- **Slice 5.3:**
  1. Do NOT map `--accent` wholesale to `#40A578`. `--accent` also feeds `--focus-ring`, the checkbox `accent-color`, the selected-meeting inset bar and the `.badge` border. `#40A578` is decorative only (AC 5.3.3).
  2. On the deep-green header band, `.notebook__count`, `.notebook__title` and `.notebook__mark` need light-on-dark tokens. Check them at 4.5:1.
  3. Keep the `prefers-reduced-transparency` fallback working on the new header background.
  4. The Google Fonts `<link>` is now allowed. Context 005's "no external fonts" trap is superseded by AC 5.3.2. Keep the system serif and sans fallbacks in `--font-body` and the heading stack.
  5. The arrow icon is an inline `svg` with `aria-hidden="true" focusable="false"`, like `.notebook__mark`.
- **Slice 5.4:**
  1. Remove the `window.storage` path entirely. Published artifacts never provided it (9.1).
  2. Read-only viewers get a disabled checkbox with `aria-describedby` pointing at a reason, as `buildActionButton` does, and make no write attempt (AC 5.4.3).
  3. On failure, revert to the saved state. Do not trust the optimistic DOM (AC 5.4.4).
  4. Do NOT "fix" one Contributor's tick closing the todo for the owner. It is intended ([spec §10](007-meeting-notebook-database.md#10-established-behaviour--documented-not-changed) 10.1).
- **Slice 5.5:**
  1. Grouping, ordering and filtering are pure core functions, not DOM code (§7.2, NFR 6.3.2).
  2. `Unassigned` must catch both the literal `"Unassigned"` owner and an empty owner. Otherwise "Unassigned" shows up as a person under Waiting on others.
  3. Ids collide when the same todo is in the DOM in both tabs: `reasonId` (`"reason-" + meeting.id + "-" + todo.id`) and `meeting-select`. Scope the ids per tab.
  4. Do NOT sort or group by `due`. It is a phrase (10.2).
  5. Adding tabs changes the header height. Keep `--header-height`/`scroll-padding-top` in step so the jump target is not hidden (AC 5.5.6).
  6. Remember the filter per device, not in the shared database. `window.storage` is absent and `localStorage` may throw in the artifact iframe, so wrap it and fall back to Open.
- **Slice 5.6:**
  1. The header count is the total, not `loaded.length` (AC 5.6.4).
  2. At exactly 20 meetings there is no button (AC 5.6.3). Fetch one extra record or count, and never show a button that loads nothing.
  3. Under 640px `.meeting-list` is `display: none`, so the button must sit outside it next to `.sidebar__select` (AC 5.6.5).
  4. Keep the selection and the scroll position. Append rather than re-render the list.
- **Slice 5.7 (destructive):**
  1. Never delete a legacy notebook without explicit per-notebook confirmation in chat (AC 5.7.3). The codebase has no typed-confirmation or impact-count convention, so the confirmation is a chat question naming each notebook and its meeting count.
  2. Imported todos start `done: false` (AC 5.7.2).
  3. De-duplicate by meeting id across both legacy notebooks and the target.
  4. When a cap is hit, stop, keep the earlier writes and report the counts (AC 5.7.5).
  5. Upgrade republishes to the same URL with the same capabilities. Do NOT create a new artifact.

## 5. Known dragons
- **The template must stay one self-contained file that the skill publishes as is.** "Start from the unmodified template file each time … Never edit the template's markup, styles or script" (`SKILL.md`). No external JS, no build step, no split CSS ([spec §7.1](007-meeting-notebook-database.md#71-tech-stack), 9.5). The only allowed external host is Google Fonts.
- **Runtime APIs are unconfirmed outside Claude Code.** The artifact `list`/`read`/publish-to-URL tools and the `db` capability are confirmed only in Claude Code ([spec §8](007-meeting-notebook-database.md#8-open-questions) 8.1). Commit 8339713 already found one wrong guess: published artifacts expose only `window.claude.use`, not `window.claude.complete`. Feature-detect the `db` API the same way, and take its exact names from the artifact runtime contract 0.2.88 type definitions (spec source 1, in the Claude Code bundle, not in this repo). Do not guess them.
- **The plugin version was not bumped by 8339713.** `plugin.json` is still `0.1.0` (changed only in ca25aed), even though the README rule requires a bump with every plugin change. Bump it in this spec's first plugin commit.
- **CI skips `**/*.md`.** A change only to `SKILL.md` or a README runs no CI. `notebook.html` changes do trigger CI, but nothing there checks the file.
- **`release.sh` (`build_plugin_zip`) zips all of `plugins/scribe`.** Tests, fixtures or `package.json` placed there would ship to users.
- **The `--on-accent` token is defined but unused** in `notebook.html`. It is harmless, but do not read it as being in use.

## 6. Test conventions
- **No tests exist for the plugin or the notebook.** Verification has been manual: `claude plugin validate --strict plugins/scribe` and `claude plugin validate --strict .`, a run with `claude --plugin-dir ./plugins/scribe`, and opening the template in a browser.
- **The only JS harness is `apps/scribe-tray`.** It uses Vitest 4 with `environment: "jsdom"` and `passWithNoTests: false`. Setup is `src/test-setup.ts`. Specs sit next to their code as `*.spec.ts(x)`, and test names are business sentences (`describe("the live transcript feed")`, `it("replaces a partial line with the final wording")`). Pure-logic reference: `src/popover/feedReducer.spec.ts` (its `replay` helper). Accessibility checks use `vitest-axe` (see `PopoverActions.spec.tsx`). Run `npm test` in `apps/scribe-tray` and **check the exit code**, because an unhandled rejection fails the run even when every test passes.
- **If notebook tests live in the tray harness:**
  - Load the template with Vite's `?raw` import (`vite/client` types are in `tsconfig.json`). Do not use `node:fs`, because there is no `@types/node` and `tsc` (part of `npm run lint`) would fail.
  - Run the script with `new JSDOM(html, { runScripts: "dangerously" })` (`jsdom` is a devDependency).
  - Only top-level `function` declarations become `window` properties. `const state` and `BLOCK_RENDERERS` do not.
  - Errors inside that JSDOM window go to its `VirtualConsole`, not to Vitest. Wire it up so AC 5.1.5 ("no uncaught errors") can be asserted.
  - `mockIPC` (`@tauri-apps/api/mocks`, used in `Popover.spec.tsx`) is for Tauri commands only. Stub `window.claude` on the notebook's window instead.
- **CI pickup.** Only specs under `apps/scribe-tray` run, in the `portable` job (ubuntu). A harness elsewhere, for example `plugins/<something>/`, needs its own `package.json` and a new CI step, and must stay outside `plugins/scribe` (the zip).
- **Lint.** `eslint .` and `tsc` cover only `apps/scribe-tray`. The template's inline JS gets no `sonarjs`, no `complexity: 5` and no `max-depth: 3` checks.
- **Mutation.** Stryker mutates `src/**/*.{ts,tsx}` only, so the template's inline script is never mutated. The CI mutation job runs only on pull requests, and ADR 7 puts work directly on `main`. Run `npm run mutate` locally.

## 7. Hot paths and performance landmines
- **Snapshot callbacks fire for every write from any viewer.** Full re-renders on each callback cost focus and in-flight action results (see 4c 5.1).
- **Three live subscriptions** ([spec §7.4](007-meeting-notebook-database.md#74-data-flow)): the paged meeting list, the selected meeting, and all todos. Queries run without indexes. Do not add per-todo meeting lookups, because the todo records already carry `meeting_title` and `meeting_started_at` for this.
- **Caps.** 256 KiB per document and 25,000 documents per artifact. A very long meeting's `blocks` is the record most likely to hit the size cap (AC 5.2.8).
- **Skill side.** Do not `read` the whole notebook page on each run (9.3). `get_transcript` stays at one call per run.

## 8. Open uncertainties resolved during orientation
- [spec §8](007-meeting-notebook-database.md#8-open-questions) 8.1 (Desktop and Cowork artifact tools): **not resolved by the repo.** It still needs the manual run after 5.2. Until then, the 5.2.9 markdown fallback is the safety net.
- **Plan 005 decisions.** The spec's changelog supersedes decision 4 (one notebook per conversation). Decision 5's "template + data island" keeps the fixed template, but the island survives only in legacy pages (read by 5.7). Decision 8's `window.storage` ticks are replaced by `done` on todo records.
- **NFR 6.3.1 "rewrite-keeps-done covered by automated tests".** In this design the merge runs in the skill (Claude following `SKILL.md` prose), not in page code, so the repo has nothing to unit-test for it. The planner must decide whether a pure merge function in the page core, tested but not run by the skill, satisfies the NFR, or whether it is verified manually.
- **Per-device filter memory (AC 5.5.4).** No storage that works in published artifacts is confirmed in the repo (see 4c 5.5 item 6).

## 8a. Design system
- **Source of truth.** The `:root` custom properties in `notebook.html`, copied from `apps/scribe-tray/src/theme/tokens.css` at 005. Slice 5.3 re-values them from the [spec §5.3](007-meeting-notebook-database.md#53-wear-the-incubyte-theme) token table (Incubyte brand: `#014D43`, `#EBFFF6`, `#D3FE73`, `#40A578`). There is no `design.md`, and the template does not import the tray's CSS.
- **Tokens.**
  - Spacing `--space-1..5` = 4/8/12/16/24px. Radius `--radius-control` 8px. Type sizes `--text-sm` 12, `--text-md` 13, `--text-lg` 15. Titles are 18 and 20px at weight 650. Body `line-height` is 1.45.
  - Surfaces: `--surface`, `--surface-sunken`, `--surface-hover`, `--glass`, `--hairline`, `--control-border`.
  - Risk: `--recording` / `--recording-soft`.
- **Components (in-template, reuse rather than re-create):**
  - `.button`, `.badge`, `.panel` with `__heading`/`__body`.
  - `.session` with `__title`/`__meta`.
  - `.item` with its accent dot, and `.line--me` with its 2px accent rule for the user's own todos.
  - The `.block--*` set, `.offset-chip`, `.empty`, `.action-output`, `.todo__*`, `.visually-hidden`.
- **Conventions.**
  - The layout is dense: a 200-280px sidebar beside a 760px max meeting column.
  - Dark mode is a real token swap under `prefers-color-scheme`, with `color-scheme: light dark`. Reduced motion turns transitions off, and reduced transparency drops the glass.
  - `forced-colors` sets `Highlight`/`ButtonText`/`GrayText` overrides.
  - The 640px breakpoint swaps the list for a `<select>`.
  - Every focus state uses the `:focus-visible` ring. The target is WCAG 2.2 AA.
- **Per-slice UI notes:**
  - **Slice 5.3:** the spec asks for lime pill buttons, which means changing `.button` radius and fill. Keep the 32px `min-height` and the disabled and `forced-colors` rules.
  - **Slice 5.5:** imitate `.session[aria-current]` for the selected tab and `.sidebar__heading` for group headings. Tabs and the filter have no precedent.

---
## Changelog
- 2026-10-21 — generated by code-orienteer from spec d0922bbc-4934-4cef-b35c-07e9cc016c5f at HEAD 8339713.
