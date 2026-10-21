# Plan: 007-meeting-notebook-database

> Spec: `docs/specs/007-meeting-notebook-database.md` · Context: `docs/specs/007-meeting-notebook-database-context.md` · Authored: 2026-10-21
> Risk: MODERATE (user-facing, non-regulated; the skill writes and deletes shared records across chats) · Hard floors: slices 5.2, 5.7 · Fabric standards: reachable

---

# Part 1 — Review this

*Everything a reviewer needs to approve or redirect the approach. If you only read one part, read this one.*

## What we're doing

Today every chat that runs `/meeting-notes` makes its own notebook, because the meetings live inside the page. We move them into the published artifact's database. The meeting-notes skill finds your one notebook by title from any chat and writes a meeting and its todos as records. The page reads those records live, adds a Todos tab across all meetings, pages the meeting list 20 at a time, and takes on Incubyte's look.

The constraint that shapes everything is that the notebook must stay **one HTML file the skill publishes as is**, with no build step ([spec §7.1](../specs/007-meeting-notebook-database.md#71-tech-stack)). So the hexagonal seams the spec asks for (a pure core, a store adapter, an ask-Claude adapter) are named sections inside one script, not separate files. Tests load that file into a simulated browser with a fake Claude runtime, because nothing else can run it.

One deliberate exception: the "rewriting a meeting keeps done state" rule runs in the skill, which is prose Claude follows, so it cannot be unit-tested. It is verified by a scripted manual run instead (decision 4).

## How it breaks down

Slice 5.1 rebuilds the page around the database and lays down the test harness every later slice reuses. Slice 5.2 makes the skill write to it. Slices 5.3 to 5.6 are page features on top of 5.1. Slice 5.7 is skill-only. The order is the spec's surface order.

| # | Slice | Delivers | Why this tier | Tier |
|---|-------|----------|---------------|------|
| 5.1 | Show meetings from the notebook's database | Page reads meetings and todos live from the database; test harness with a fake runtime | Restructures the whole script and invents the harness every later slice depends on | `best` |
| 5.2 | Add a meeting to the one notebook from any chat | Skill finds or creates the notebook and writes a meeting in one batch | Correctness lives in prose Claude follows; declares access rules (floored) | `excellent` |
| 5.3 | Wear the Incubyte theme | Brand tokens, fonts, lime pill buttons, deep-green dark mode | Token and CSS work with a contrast guard; well understood | `excellent` |
| 5.4 | Tick a todo done for everyone | Shared `done` on the todo record, read-only and failure handling | Small, but async write ordering and rollback need care | `excellent` |
| 5.5 | See every todo in a Todos tab | Tabs, owner grouping, filter, jump to meeting | Most new UI: tab semantics, focus moves, id scoping | `excellent` |
| 5.6 | Show older meetings 20 at a time | "Show older meetings" in the list and the mobile picker | Client-side paging over an index already loaded | `fast` |
| 5.7 | Import old notebooks and upgrade the page | Skill imports legacy meetings, upgrades the page, offers deletes | Destructive and migration-shaped prose (floored) | `excellent` |

Slices 5.4 and 5.5 share one write path (`setTodoDone`) and one todo row renderer. 5.5 reuses both rather than building a second row.

## Decisions made

**1. The page never holds meeting data; the database does.** The skill writes records through the artifact's database, the page subscribes to them, and the old data island is removed with no fallback ([spec §9.2](../specs/007-meeting-notebook-database.md#9-deferred-follow-ups-and-accepted-limits)). Every slice depends on this. *Rejected: keeping the island as a second path for chat artifacts. That means two data models to keep in step forever.*

**2. A small `meeting_index` collection backs the list and the count.** Each meeting also gets a summary record (title, start, duration, type). The page loads the whole index, so the header count is exact and paging is just slicing what is loaded. It opens the full meeting only when you select it. The database has no count query and scans without indexes, so this is the cheapest way to meet AC 5.6.3 and 5.6.4. It adds one record kind to the spec's two, recorded in the escalation log. *Rejected: querying full meeting records 20 at a time. The total count would be unknowable, and each page would pull about 12 KB per meeting.*

**3. A `notebook/meta` marker record identifies the notebook.** The skill lists your artifacts titled "Scribe notebook", newest first, and reads `notebook/meta` on each. The database tool refuses outright on an artifact that declares no database. This was confirmed against your legacy notebook, so legacy notebooks are skipped cheaply without reading their pages. *Rejected: reading each candidate page to inspect its declared capabilities, which costs about 12k tokens per candidate per run ([spec §9.3](../specs/007-meeting-notebook-database.md#9-deferred-follow-ups-and-accepted-limits)).*

**4. Rewriting a meeting merges into surviving todos instead of replacing them.** For a todo id that already exists, the skill sends a merge update that never mentions `done` or `done_at`, so the done state survives by construction. New todos are created open, and dropped ones are deleted. No merge logic is needed, so nothing is left to unit-test; the rule is checked by a scripted manual run in 5.2's verification. *Rejected: a merge function in the page core that is tested but that the skill never runs. It would test code nobody executes.*

**5. Access rules are declared once at publish.** The rules are `notebook`, `meetings` and `meeting_index` writable at Editor level and above, and `todos` at Contributor level and above. The page asks the runtime whether this viewer can write shared records before enabling checkboxes ([spec §6.2.2](../specs/007-meeting-notebook-database.md#62-security-incubyte-engineering-standards-sonarjs)). *Rejected: default rules, which would let any Contributor rewrite meeting notes.*

**6. The template's script becomes one immediately invoked function with named sections.** The sections are Copy, Record contracts, Notebook core, Store adapter, Ask-Claude adapter, Views and App. Nothing leaks to `window`, the [spec §7.3](../specs/007-meeting-notebook-database.md#73-module-decomposition) seams are visible, and a fresh test window can run it cleanly every time. *Rejected: splitting into files, which the publish-as-one-file constraint rules out.*

**7. Notebook tests live in the tray's test runner under `apps/scribe-tray/src/meeting-notebook/`.** That runner is the only one CI runs. Anything under `plugins/scribe` would ship in the plugin zip. Each test builds a fresh simulated browser from the template, installs a fake Claude runtime (an in-memory database, ask-Claude and viewer access), and drives the page through its DOM. *Rejected: a new package and CI job for the plugin, which is a second toolchain for one HTML file.*

**8. Published artifacts only, so the chat-artifact `window.claude.complete` path is removed.** Ask-Claude goes through `claude.use("sample")` alone ([spec §9.2](../specs/007-meeting-notebook-database.md#9-deferred-follow-ups-and-accepted-limits)). *Rejected: keeping it "just in case". It is dead in every supported host.*

**9. Live updates patch in place.** A snapshot of the meeting index re-renders only the list. The selected meeting's detail re-renders only when that meeting's own record changes. A todo snapshot patches checkboxes and counts in place. This protects keyboard focus and in-progress AI results (context trap 5.1-2).

**10. The skill's notebook procedure moves to a sibling file, `notebook.md`.** `SKILL.md` would otherwise pass about 150 lines once import and upgrade arrive. `SKILL.md` keeps a short pointer, as it already does for `guide.md`.

**11. The plugin version goes to `0.2.0` in slice 5.1's commit and stays there for the rest of this run.** No version ships between slices, and the README rule is about released changes. The missed bump from commit 8339713 is folded into this one.

## Where this could go wrong

- **The skill picks the wrong notebook or writes half a meeting** — prose correctness is not machine-checked. It is guarded by 5.2's bolded quality check, which requires one batch and the marker-record lookup, and by the two-chat manual run in its verification.
- **The test harness diverges from the real runtime** — the fake database could accept calls the real one rejects. It is guarded by 5.1's quality check that the fake implements only the methods named in the 0.2.88 contract, delivers snapshots asynchronously, and rejects writes the way the contract describes.
- **Re-rendering on every snapshot** wipes focus and AI results. Guarded by 5.1's and 5.4's in-place patch checks.
- **Contrast regressions on the deep-green header band** — guarded by 5.3's token-contrast test, which reads the template's own custom properties.
- **Scope creep into adjacent limits**: "fixing" a Contributor's tick closing the todo for everyone ([spec §10.1](../specs/007-meeting-notebook-database.md#10-established-behaviour--documented-not-changed)), parsing due dates (10.2), or adding an automatic page-version check ([spec §9.3](../specs/007-meeting-notebook-database.md#9-deferred-follow-ups-and-accepted-limits)). Each has an "unchanged" check in the slice that touches it.

## What's out of scope

From [spec §1](../specs/007-meeting-notebook-database.md#1-overview--business-context): search and date filters, due-date parsing and reminders, editing notes or todos in the page, chat artifacts, and any Scribe app or MCP server change. From [spec §9](../specs/007-meeting-notebook-database.md#9-deferred-follow-ups-and-accepted-limits):
- No automatic template upgrades (9.3).
- No URL pointer stored by the app (9.4).
- No splitting the template into files (9.5).
- No regenerating AI action prompts in the page (9.6).
- No recovering legacy ticks (9.1).

A builder in `notebook.html` could touch 9.3 and 9.5 by accident, and one in `notebook.md` could touch 9.4.

---

# Part 2 — Build detail

*Prescriptive detail for the build agent. Reviewers can skim or skip.*

**Confidence markers used below:** `[verified]` = the planner read the code or docs and confirmed it. `[inferred]` = pattern-matched from a sibling implementation, not confirmed — the build agent should check before assuming.

## Shared conventions (all slices)

- **Architecture:** hexagonal inside the page ([spec §7.2](../specs/007-meeting-notebook-database.md#72-architectural-style)). The Rust workspace, `scribe-mcp` and the tray app's runtime code are unchanged.
- **Dependency direction:** Views → Notebook core → Record contracts; Views → Store adapter / Ask-Claude adapter → runtime (`window.claude.use`). The core never touches the DOM or the runtime. Adapters never render.
- **Module map:**
  - `plugins/scribe/skills/meeting-notes/notebook.html` · the page: tokens, shell markup, one IIFE script in named sections · depends on the artifact runtime only `[verified]`
  - `plugins/scribe/skills/meeting-notes/SKILL.md` · skill entry; points to `notebook.md` for publishing `[verified]`
  - `plugins/scribe/skills/meeting-notes/notebook.md` (new) · notebook find, create, write, import and upgrade procedure
  - `plugins/scribe/.claude-plugin/plugin.json` · version `[verified]`
  - `plugins/scribe/README.md`, `README.md` (root) · user docs `[verified]`
  - `apps/scribe-tray/src/meeting-notebook/` (new) · notebook test harness and specs · depends on `jsdom`, `vitest`, and the template via `?raw`
  - `apps/scribe-tray/vite.config.ts` · test runner config `[verified]`
  - `apps/scribe-tray/stryker.config.json` · mutation scope `[verified]`
- **Script section order in `notebook.html`:** Copy · Record contracts · Notebook core · Store adapter · Ask-Claude adapter · Viewer access adapter · Views · App. Each section is a contiguous block of function declarations inside one `(function scribeNotebook() { ... })();`. Section boundaries are shown by order and naming only, with no comment banners (Clean Code: comment convention).
- **Lexicon (from [context §4b](../specs/007-meeting-notebook-database-context.md#4b-slice-lexicon) — do not rename):**
  - Title `Scribe notebook`.
  - Collections `meetings`, `todos`, plus `meeting_index` and `notebook` (decisions 2 and 3).
  - Todo doc id `<meeting_id>--<todo_id>`.
  - Meeting fields `title`, `started_at`, `duration_seconds`, `type`, `summary`, `speakers`, `blocks`, `todo_ids`.
  - Index fields `title`, `started_at`, `duration_seconds`, `type`.
  - Todo fields `meeting_id`, `meeting_title`, `meeting_started_at`, `todo_id`, `text`, `owner`, `due`, `actions`, `done`, `done_at`.
  - Owner groups `mine`, `others`, `unassigned`. Filters `open`, `done`, `all`. Constant `MEETINGS_PER_PAGE = 20`.
  - Existing helpers keep their names: `element`, `records`, `textOf`, `meetingTitle`, `formatWhen`, `renderTodo`, `renderActions`, `buildActionButton`, `runAction`, `completionText`, `BLOCK_RENDERERS`, `renderEmptyState`, `renderMeetingCount`, `selectMeeting`.
- **Runtime contract (0.2.88) `[verified]` from the bundled type definitions:**
  - `claude.use(name)` resolves a namespace or `null`, never synchronously, and `null` after about 10 s when no viewer answers.
  - `db.collection(path)` / `db.doc(path)`; `query.where/orderBy/limit`, `.get()`, `.onSnapshot(next, error)` returns an unsubscribe.
  - `doc.update(data)` merges and rejects `invalid_argument` if the document is absent.
  - A rejection is `{code, message}`.
  - `user.can("data.write")` resolves `true | false | null`.
  - `sample(prompt)` resolves `{text, truncated}`.
- **Tests:**
  - New spec files under `apps/scribe-tray/src/meeting-notebook/`, named `*.spec.ts` (Incubyte: test files end with `.spec.ts`). `describe`/`it` names are business sentences.
  - Each test mounts a fresh page with `mountNotebook` and a runtime from `createFakeClaudeRuntime`, acts through the DOM, and asserts on DOM state or on the fake database's stored records (state over interaction).
  - Run `npm test` and `npm run lint` in `apps/scribe-tray` and check the exit code.
- **Version:** decision 11.

---

## Slice 5.1 — Show meetings from the notebook's database

**Key decision:** Rebuild the page's data flow around the store adapter (decisions 1, 2, 6, 8, 9). It removes the data island and `window.storage`, and builds the test harness all later slices use (decision 7). The visible layout and block rendering stay exactly as today.

**Structure**

- `plugins/scribe/skills/meeting-notes/notebook.html` (modify)
  - Shell: replace `<script type="application/json" id="scribe-meetings">` with nothing. Keep `#meeting-count`, `#sidebar` and `#meeting-pane`.
  - Copy: add `DATABASE_UNAVAILABLE_MESSAGE = "This notebook needs Claude to load. Open it on claude.ai while signed in."`. Change the `renderEmptyState` body copy to "Run /meeting-notes in Claude to add your first meeting." and keep `AI_UNAVAILABLE_REASON`, `ACTION_FAILED_MESSAGE`, `WORKING_LABEL`, `UNTITLED_MEETING` and `FALLBACK_MEETING_TYPE`.
  - Record contracts:
    - `COLLECTIONS = { meetings: "meetings", meetingIndex: "meeting_index", todos: "todos" }`
    - `todoDocId(meetingId: string, todoId: string): string`
    - `asMeetingSummary(snapshot: {id, data()}): MeetingSummary`
    - `asMeeting(snapshot): Meeting`
    - `asTodo(snapshot): Todo`. These normalise with `textOf` and `records` and never throw on bad fields.
  - Notebook core:
    - `orderNewestFirst(summaries: MeetingSummary[]): MeetingSummary[]`, by `started_at` descending, invalid dates last.
    - `todosOfMeeting(todos: Todo[], meeting: Meeting): Todo[]`, ordered by `meeting.todo_ids` and then `todo_id`.
  - Store adapter:
    - `resolveNotebookStore(): Promise<NotebookStore | null>`, from `claude.use("db")`.
    - `createNotebookStore(db): NotebookStore` with:
      - `watchMeetingIndex(onChange: (s: MeetingSummary[]) => void, onError: (e) => void): () => void`
      - `watchMeeting(meetingId: string, onChange: (m: Meeting | null) => void, onError): () => void`
      - `watchTodos(onChange: (t: Todo[]) => void, onError): () => void`
  - Ask-Claude adapter: `resolveAskClaude(): Promise<((prompt: string) => Promise<unknown>) | null>`, using `use("sample")` only.
  - Views:
    - Keep the existing renderers. `renderMeeting(meeting: Meeting, todos: Todo[])` now takes todos separately.
    - New `renderUnavailableState(): HTMLElement` mirrors `renderEmptyState`.
    - New `patchMeetingList(summaries)` replaces the list's children only.
  - App:
    - `state = { summaries: [], selectedId: null, selectedMeeting: null, todos: [], askClaude: null, unwatchSelected: null }`
    - `start(): Promise<void>`
    - `showSummaries(summaries)`
    - `selectMeeting(id: string | null)` subscribes to the selected meeting and unsubscribes the previous one.
    - `showSelectedMeeting(meeting)`
    - `showTodos(todos)`
    - `enableClaudeActions(askClaude)`
  - Delete: `readMeetings`, `loadTicked`, `saveTicked`, `tickedStorageKey`, `storedText`, `tickedIds`, `applyTicked`, `ignoreFailure`, `state.ticked`, and the `window.claude.complete` branch.
- `plugins/scribe/.claude-plugin/plugin.json` (modify) — `version` `0.2.0` (decision 11).
- `apps/scribe-tray/vite.config.ts` (modify) — `server.fs.allow: [".", "../../plugins/scribe/skills/meeting-notes"]`. `[verified]` by probe: without it the `?raw` import fails with "Denied ID"; with it the import loads.
- `apps/scribe-tray/stryker.config.json` (modify) — add `"!src/meeting-notebook/**"` to `mutate`. The folder holds only test support; the inline script is not mutable by Stryker.
- `apps/scribe-tray/src/meeting-notebook/jsdom.d.ts` (new) — `declare module "jsdom"` with only what is used:
  - `class JSDOM { constructor(html: string, options?: { url?: string; runScripts?: "dangerously"; virtualConsole?: VirtualConsole; beforeParse?: (window: Window) => void }); readonly window: Window }`
  - `class VirtualConsole { on(event: "jsdomError", listener: (error: Error) => void): this }`
  - `[verified]` against `node_modules/jsdom/README.md` (v29.1.1 ships no types; `@types/jsdom` is not installed and pulls in `@types/node`).
- `apps/scribe-tray/src/meeting-notebook/fakeClaudeRuntime.ts` (new)
  - `createFakeClaudeRuntime(options?: { database?: "available" | "unavailable"; canWriteData?: boolean | null; askClaude?: "available" | "unavailable"; answer?: string }): FakeClaudeRuntime`
  - `FakeClaudeRuntime = { claude: { use(name: string): Promise<unknown> }; database: FakeDatabase }`
  - `FakeDatabase`:
    - `seed(collection: string, id: string, data: object): void`
    - `write(collection: string, id: string, data: object): void`, a server-side write that notifies subscribers.
    - `read(collection: string, id: string): object | undefined`
    - `failNextUpdate(code: string): void`
    - `db: { collection(path), doc(path) }` implementing `where` (`==` only), `orderBy`, `limit`, `get`, `onSnapshot`, `update`.
  - Snapshots are delivered on a later macrotask, never synchronously.
- `apps/scribe-tray/src/meeting-notebook/mountNotebook.ts` (new)
  - `mountNotebook(runtime: FakeClaudeRuntime, options?: { storedFilter?: string }): Promise<MountedNotebook>`
  - `MountedNotebook = { document: Document; window: Window; errors: Error[]; settle(): Promise<void> }`
  - It uses `url: "https://notebook.test/"` so `localStorage` exists, and `beforeParse` installs `window.claude`, stubs `Element.prototype.scrollIntoView` (jsdom does not implement it `[verified]`, a jsdom limitation) and seeds `localStorage` when asked. A `VirtualConsole` collects `jsdomError` into `errors`.
- `apps/scribe-tray/src/meeting-notebook/notebookRecords.ts` (new) — test-data builders `aMeeting(overrides)`, `aSummaryOf(meeting)`, `aTodo(meetingId, overrides)` returning record bodies in the lexicon shape.

**Collaborators:** The App section calls the adapters once in `start` and hands the results to Views. Views call core functions. Only the store adapter calls `db`, and only the ask-Claude adapter calls `sample`. The test files depend on the harness, and the harness depends on `jsdom`; nothing in `src/` production code imports from `meeting-notebook/`.

**Control flow:**
- `start`: render the header count as empty, then await `resolveNotebookStore`.
  - When the store is `null`, render the unavailable state and stop.
  - Otherwise subscribe to the meeting index and to the todos, and separately await `resolveAskClaude`, passing the result to `enableClaudeActions`.
- On an index snapshot: order it, update the count, and patch the list.
  - With no selection, select the newest.
  - With a selection that is no longer present, select the newest.
  - With no summaries, render the empty state.
- On selecting a meeting: unsubscribe the previous selection and subscribe to the selected meeting. On its snapshot, ignore it unless `state.selectedId` still matches, then render the detail with `todosOfMeeting`.
- On a todos snapshot: store the todos and re-render only the selected meeting's todo panel (slice 5.4 refines this into an in-place patch).
- On any subscription error: render the unavailable state once and log via `logFailure`.
- No write anywhere in this slice.

**Tests:** new `apps/scribe-tray/src/meeting-notebook/meetingsFromDatabase.spec.ts`, at integration level through the DOM:
- "lists meetings newest first and opens the newest" (5.1.1)
- "shows a meeting's speakers, summary, blocks and todos" (5.1.2), with one block of each kind
- "adds a meeting written while the page is open without losing the selection" (5.1.3)
- "invites you to add a meeting when the notebook is empty" (5.1.4)
- "explains that the notebook needs Claude when the database is unavailable", which asserts the message and `errors` empty (5.1.5)
- "shows markup in a meeting title as plain text" (5.1.6)
- "runs an AI action through Claude and shows the answer", plus "disables AI actions with a reason when Claude is unavailable" (5.1.7)

**Standards that bite here:**
- Clean Code: small functions (under 20 lines), separation of concerns, comment convention (no comments).
- Incubyte: complexity of 5 or less, `.spec.ts` naming, business-language test names.
- SonarJS: disabled-auto-escaping (`textContent` only, never `innerHTML`).
- code-craft: functional core and imperative shell.

**Quality checks**

- **No path reads meeting data from the page itself: the `scribe-meetings` island, `readMeetings` and every `window.storage` call are gone (decision 1).**
- The script declares nothing on `window`: one IIFE wraps every declaration (decision 6).
- Only Store-adapter functions reference `db`, and only Ask-Claude-adapter functions reference `sample` (spec 6.3.2).
- No `innerHTML`, `insertAdjacentHTML` or `outerHTML` anywhere in the template (AC 5.1.6).
- No database write call exists in this slice's template code (spec §7.2 anti-patterns).
- A snapshot for a meeting other than `state.selectedId` never renders (context trap 5.1-2).
- The fake database delivers snapshots asynchronously and implements only methods named in the 0.2.88 contract.
- Every new function in the template stays under 20 lines with at most 5 branches.
- `npm test` and `npm run lint` in `apps/scribe-tray` exit 0.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **User-facing flow.** Publish the template as a scratch artifact with the database declared, seed two meetings and their todos through the artifact database tool, open it, and confirm the newest is selected. Write a third meeting and confirm it appears without a reload.
3. **Accessibility.** Same session: tab-order traversal through the meeting list and the AI action buttons, and focus visibility on every interactive control.

---

## Slice 5.2 — Add a meeting to the one notebook from any chat

**Key decision:** The skill finds the notebook through the marker record (decision 3), writes the meeting, its index summary and its todos in one batch, and merges into surviving todos so done state survives (decision 4). It declares the access rules on creation (decision 5). The procedure lives in the new `notebook.md` (decision 10).

**Structure**

- `plugins/scribe/skills/meeting-notes/SKILL.md` (modify)
  - Replace "Build or update the notebook" with a short section: build the records described below, then follow `notebook.md` beside this file.
  - Replace the *Rules* lines "Never create a second notebook in the same conversation" and the `<`-escape rule with "Never create a second notebook while one exists" and "Never write to a notebook that declares no database".
  - Remove the `capabilities: {sample: {}}` paragraph.
  - Keep pick the call, speakers, passes, shape, todos, and the rules on sensitive screens and screenshots unchanged.
  - Keep the field definitions for the meeting body (`title` … `blocks`) and the todo body (`id` → `todo_id`, `text`, `owner`, `due`, `actions`) in SKILL.md, since note-shaping owns them.
- `plugins/scribe/skills/meeting-notes/notebook.md` (new), in sections:
  1. **Find the notebook.**
     - List your own artifacts and keep those titled exactly `Scribe notebook`, newest first.
     - For each, read the database document `notebook/meta`. A refusal saying the artifact does not declare the database means a legacy notebook: skip it. The first that answers is the notebook.
     - With none, create it (section 2). `[verified]`: the database tool refuses on a legacy notebook with "does not declare the database capability".
  2. **Create the notebook.** Publish `notebook.html` from this folder unmodified, titled `Scribe notebook`, with exactly these capabilities:
     ```json
     {"db": {"rules": [
       {"path": "notebook", "write": "admin"},
       {"path": "meetings", "write": "admin"},
       {"path": "meeting_index", "write": "admin"},
       {"path": "todos", "write": "interact"}]},
      "sample": {}, "user": {}}
     ```
     `[verified]` rule grammar and levels against `db.d.ts`. Then create `notebook/meta` with `{"schema": 1, "created_at": <now ISO>}`.
  3. **Write the meeting.**
     - Read `meetings/<id>`, `meeting_index/<id>`, and the `todos` where `meeting_id` == `<id>`, noting each version.
     - Then send one batch:
       - `set` `meetings/<id>` (pinned when it exists)
       - `set` `meeting_index/<id>` with `title`, `started_at`, `duration_seconds`, `type` (pinned when it exists)
       - For each todo whose id exists: `update` with every todo field except `done`/`done_at` (pinned)
       - For each new todo: `set` with `done: false`, `done_at: null`
       - For each existing todo not in the new list: `delete` (pinned)
     - Never touch any other meeting's records.
  4. **When a write is refused.**
     - On a version conflict, re-read the same documents and send the batch once more. If refused again, reply in one line that the notebook changed while writing and to try again.
     - On a document-count or size cap, reply in one line that the notebook is full. If the result shows some writes applied, delete the documents this run created before replying.
  5. **Reply.** List `meeting_index` (page through with the cursor) and count it. Reply `Added <title>. The notebook holds N meetings.`, adding the link when the notebook was just created.
  6. **When you cannot publish or write.** If no artifact tools exist, or a publish or database call is refused for lack of support, give the notes as markdown and say the notebook needs a Claude that can publish artifacts.
- `plugins/scribe/README.md` (modify)
  - *Requirements*: add "A Claude that can publish artifacts (claude.ai/artifact links) for the notebook".
  - *Commands*: `/meeting-notes` adds the call to your one Scribe notebook from any chat.
- `README.md` (root, modify) — the same one-line change in "Install the Claude plugin".

**Collaborators:** Claude's artifact tool (list, publish) and the artifact's database tool (get, query, batch, list), named generically in prose. Section 3 depends on the record shapes defined in `SKILL.md` and the lexicon in Shared conventions. The page (5.1) reads what this writes.

**Control flow:** find → (none? create, then marker) → read the existing records → build the batch (sets, merge updates, creates, deletes) → send → refused? (conflict: one retry; cap: compensate and report) → count the index → reply. The unsupported-tools guard comes first and short-circuits to markdown.

**Tests:** no automated test, because the behaviour is prose executed by Claude. Covered by the scripted manual run in Verification step 2, which exercises 5.2.1 to 5.2.6 and 5.2.10. 5.2.7 to 5.2.9 are checked by reading `notebook.md` against the quality checks.

**Standards that bite here:**
- Least privilege (spec 6.2.2).
- Incubyte: Conventional Commits.
- code-craft: fail fast at boundaries (the unsupported-tools guard first).
- Skill voice (context §2): second person, imperative, a one-line reply, *Rules* as "Never …" lines.

**Quality checks**

- **`notebook.md` writes a meeting, its index summary and all todo changes in a single batch, and finds the notebook only through `notebook/meta`, never by "exists in this conversation" (AC 5.2.1, 5.2.3, 5.2.4).**
- No write for a surviving todo mentions `done` or `done_at` (invariant 4.4).
- Every delete in section 3 is scoped to todos whose `meeting_id` equals the meeting being written (invariant 4.5).
- The capabilities JSON matches decision 5 exactly and appears only in the create step (AC 5.2.2).
- The conflict path retries exactly once (AC 5.2.7).
- No step reads the notebook page to check its version ([spec §9.3](../specs/007-meeting-notebook-database.md#9-deferred-follow-ups-and-accepted-limits) unchanged).
- `SKILL.md` stays at about 150 lines or fewer, and its existing sections on picking the call, speakers, screenshots and sensitive screens are unchanged.
- `claude plugin validate --strict plugins/scribe` and `claude plugin validate --strict .` pass.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green.
2. **User-facing flow.**
   - In Claude Code with `--plugin-dir ./plugins/scribe`, run `/meeting-notes` in chat A on call X: a new notebook is created, the link is given, and the reply says 1 meeting.
   - In chat B on call Y: the same notebook, 2 meetings, and no new artifact in your artifact list.
   - Tick one of X's todos in the page using the artifact database tool (`update` `done: true`), then re-run X in chat C: the tick survives, and the meeting count stays 2.
   - Confirm the two legacy notebooks were not written.
3. **Accessibility.** Not applicable: this slice has no page UI of its own.

---

## Slice 5.3 — Wear the Incubyte theme

**Key decision:** Re-value the existing custom properties from the spec's token table and add a heading font token. Load Fraunces and Inclusive Sans from Google Fonts, and restyle `.button` as a lime pill with an arrow on AI actions. Structure and behaviour are unchanged ([context §4a](../specs/007-meeting-notebook-database-context.md#4a-slice-imitation-targets)).

**Structure**

- `plugins/scribe/skills/meeting-notes/notebook.html` (modify)
  - `<head>`: `<link rel="preconnect" href="https://fonts.googleapis.com">`, `<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>`, and `<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,400;9..144,600&family=Inclusive+Sans:wght@400;500;600&display=swap">`. `[verified]` incubyte.co loads Fraunces `opsz,wght@9..144,400` and Inclusive Sans `wght@400;500` from that host; the 600 weights are an addition `[inferred]` available on Google Fonts.
  - `:root` light values:
    - `--surface #EBFFF6`
    - `--text #1A1D23`
    - `--text-muted #4A5A55`
    - `--accent #014D43` (focus ring, checkbox, selected bar, badge border)
    - `--on-accent #EBFFF6`
    - `--accent-soft rgb(1 77 67 / 0.08)`
    - `--recording #B3261E`
    - New tokens: `--header-band #014D43`, `--on-header #EBFFF6`, `--button-fill #D3FE73`, `--on-button #014D43`, `--decor #40A578`, `--font-heading: "Fraunces", ui-serif, Georgia, serif`
    - `--font-body: "Inclusive Sans", -apple-system, "Segoe UI", sans-serif`
  - Dark block values:
    - `--surface #062925`
    - `--header-band #0C3A34`
    - `--text #EBFFF6`
    - `--text-muted #A8CFC4`
    - `--accent #D3FE73`
    - `--on-accent #014D43`
    - `--recording #FF8A80`
    - `--button-fill #D3FE73`
    - `--on-button #014D43`
  - Then:
    - `.notebook__header` uses `--header-band`, and its title, count and mark use `--on-header`.
    - The `prefers-reduced-transparency` fallback maps the band to an opaque `--header-band`.
    - Headings (`.notebook__title`, `.meeting__title`, `.panel__heading`, `.block--heading`, `.empty__title`) use `--font-heading`.
    - `.button` gets `border-radius: 999px`, `background: var(--button-fill)` and `color: var(--on-button)`, keeping `min-height: 32px` and the disabled and `forced-colors` rules.
    - New `.button__arrow`: a 16px round chip holding an inline `svg` arrow, `aria-hidden="true" focusable="false"`.
  - Views: `buildActionButton(action, outputElement, reasonId)` appends the arrow chip after the label text. The Copy button gets none.
- `apps/scribe-tray/src/meeting-notebook/contrast.ts` (new) — `contrastRatio(foreground: string, background: string): number` (WCAG relative luminance) and `themeTokens(html: string, scheme: "light" | "dark"): Record<string, string>`, which reads `--*` values from the template's `:root` and dark `:root` blocks.

**Collaborators:** CSS only, plus one view function (`buildActionButton`). Tests read the template text through `?raw`.

**Control flow:** none beyond the arrow append. The token cascade does the rest.

**Tests:** new `apps/scribe-tray/src/meeting-notebook/incubyteTheme.spec.ts`:
- "body and muted text stay readable on the page in light and dark" (5.3.3): `--text` and `--text-muted` against `--surface` are at least 4.5.
- "header text stays readable on the green band" (5.3.3): `--on-header` against `--header-band` is at least 4.5.
- "AI action buttons keep readable labels" (5.3.3): `--on-button` against `--button-fill` is at least 4.5.
- "focus ring is visible against the page" (5.3.3): `--accent` against `--surface` is at least 3.
- "AI action buttons show an arrow hidden from screen readers" (5.3.4), through `mountNotebook`.
- These are small, deterministic, parameter-free tests, one assertion each.

**Standards that bite here:**
- Design Standards: verify accessibility at the component and every call site (WCAG 2.2 AA); check the rendered UI.
- code-craft: no magic values outside tokens.

**Quality checks**

- **`#40A578` appears only as `--decor`, and `--decor` is used for no text, focus ring or sole control boundary (AC 5.3.3).**
- Custom-property names that existed before keep their names ([context §4b](../specs/007-meeting-notebook-database-context.md#4b-slice-lexicon)).
- The `body` background is set explicitly from `--surface` in both schemes (AC 5.3.1).
- Font stacks end in system serif or sans fallbacks (AC 5.3.2).
- `:focus-visible`, `forced-colors`, `prefers-reduced-motion`, `prefers-reduced-transparency` and the 640px block are all still present and effective (AC 5.3.5).
- The only external host in the template is Google Fonts.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green.
2. **User-facing flow.** Open the published scratch notebook in light and in dark system mode. Fonts render, the header band is deep green, and the AI buttons are lime pills with an arrow.
3. **Accessibility.** Same session: focus visibility on every interactive control in both modes, forced-colours mode keeps button boundaries, and reduced transparency shows an opaque band.

---

## Slice 5.4 — Tick a todo done for everyone

**Key decision:** A tick writes `done` and `done_at` to the todo record through the store adapter. The checkbox is patched in place from snapshots (decision 9). A per-todo write lock keeps one write in flight and saves the viewer's latest choice. Viewers who cannot write get disabled checkboxes up front through the viewer-access adapter (decision 5).

**Structure**

- `plugins/scribe/skills/meeting-notes/notebook.html` (modify)
  - Copy: `TODO_READ_ONLY_REASON = "You can view this notebook but not tick todos."` and `TODO_NOT_SAVED_MESSAGE = "Couldn't save that change. Try again."`.
  - Store adapter: `NotebookStore.setTodoDone(todoDocId: string, done: boolean, doneAt: string | null): Promise<void>`, which calls `db.doc(todos/<id>).update({done, done_at})` `[verified]`, since `update` merges.
  - Viewer access adapter: `resolveCanWriteTodos(): Promise<boolean | null>` via `use("user")` then `can("data.write")`. `null` means unknown.
  - Core: `todoDoneState(todos: Todo[], todoDocId: string): boolean`.
  - Views:
    - `renderTodo(todo: Todo, scope: string)` sets `data-todo-doc-id` and builds ids from `scope`, which 5.5 uses.
    - `patchTodoChecks(root: ParentNode, todos: Todo[])` sets `checked` from records, skipping checkboxes with a pending write.
    - `lockReadOnlyChecks(root: ParentNode, reasonId: string)`
  - App:
    - `state.canWriteTodos: boolean | null`
    - `state.pendingTicks: Map<string, boolean>`
    - `toggleTodoDone(todoDocId: string, done: boolean): Promise<void>`
    - `flushTick(todoDocId: string): Promise<void>`
  - Delete: `toggleTodo` (superseded).

**Collaborators:** Views call `toggleTodoDone` from the checkbox `change` event. App calls `store.setTodoDone`. The todos snapshot from 5.1 calls `patchTodoChecks` instead of re-rendering the panel.

**Control flow:**
- `toggleTodoDone`:
  - Record the latest choice in `pendingTicks`.
  - If a write for this id is in flight, return.
  - Otherwise loop: take the latest choice, write it with `doneAt` = now as ISO or `null`, and stop when the latest choice equals what was just written.
- On rejection: clear the pending entry, set the checkbox back to the record state from `state.todos`, and show `TODO_NOT_SAVED_MESSAGE` in a `role="alert"` line beside the todo.
- `canWriteTodos === false` → disable every checkbox with `aria-describedby` to the reason. `null` → keep them enabled and let a refused write follow the failure path.
- No write is issued from a snapshot or render path.

**Tests:** new `apps/scribe-tray/src/meeting-notebook/todoDone.spec.ts`:
- "ticking a todo marks it done with the time it was ticked", which asserts the stored record (5.4.1)
- "unticking reopens the todo and clears the time" (5.4.1)
- "a todo ticked elsewhere shows as done here" via `database.write` (5.4.2)
- "a view-only reader sees todos but cannot tick them", which asserts disabled plus reason and no stored change (5.4.3)
- "a tick that fails to save is undone with a message" via `failNextUpdate` (5.4.4)
- "rapid ticks save the last choice" with three changes before settle; the stored `done` equals the last (5.4.5)

**Standards that bite here:** Clean Code: command-query separation (`toggleTodoDone` returns nothing meaningful); Incubyte: complexity of 5 or less (split the loop into `flushTick`); WCAG 4.1.2 (disabled state exposed, reason linked).

**Quality checks**

- **At most one `setTodoDone` call per todo is in flight at any moment (AC 5.4.5).**
- No reference to `window.storage` or `localStorage` for done state (context trap 5.4-1).
- A failed write never leaves the checkbox showing an unsaved state (AC 5.4.4).
- A Contributor's tick still closes the todo for everyone ([spec §10.1](../specs/007-meeting-notebook-database.md#10-established-behaviour--documented-not-changed) unchanged).
- Snapshot handling patches checkboxes in place and never calls `renderMeeting` (decision 9).

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green.
2. **User-facing flow.** In the scratch notebook, tick a todo, reload, and confirm it stays ticked. Open a second browser window and confirm it shows ticked there too.
3. **Accessibility.** Same session: the checkbox is reachable and toggles with Space. As a view-only share, the checkbox is disabled and its reason is announced.

---

## Slice 5.5 — See every todo in a Todos tab

**Key decision:** Add an ARIA tab list in the header with a Meetings panel (today's body) and a Todos panel. The Todos panel renders core-grouped todos with the shared `renderTodo` scoped to `todos`, a three-way filter remembered in `localStorage` behind try/catch, and meeting links. A link switches to the Meetings tab, selects the meeting (subscribing directly even when it is beyond the loaded page), then scrolls, highlights and focuses the todo.

**Structure**

- `plugins/scribe/skills/meeting-notes/notebook.html` (modify)
  - Shell: in the header, `<div role="tablist" aria-label="Notebook views">` with two `button role="tab"` (`#tab-meetings`, `#tab-todos`), each with `aria-controls`. `#panel-meetings` wraps the existing `.notebook__body`, and `#panel-todos` is a new `role="tabpanel"` section. `--header-height` grows to fit the tabs.
  - Copy: `TODOS_TAB_LABEL = "Todos"`, `NO_TODOS_MESSAGE = { open: "No open todos", done: "No done todos", all: "No todos yet" }`, `GROUP_HEADINGS = { mine: "Mine", others: "Waiting on others", unassigned: "Unassigned" }`, `FILTER_STORAGE_KEY = "scribe-notebook-todo-filter"`.
  - Core:
    - `ownerGroupOf(owner: string): "mine" | "others" | "unassigned"` (`Me` → mine; empty or `Unassigned`, trimmed and case-insensitive → unassigned).
    - `orderTodosNewestMeetingFirst(todos: Todo[]): Todo[]` (meeting `meeting_started_at` descending, then `todo_id` natural order).
    - `filterTodos(todos: Todo[], filter: "open" | "done" | "all"): Todo[]`
    - `groupTodosByOwner(todos: Todo[]): { mine: Todo[]; others: { owner: string; todos: Todo[] }[]; unassigned: Todo[] }` (others sorted by owner name).
    - `countOpenTodos(todos: Todo[]): number`
  - Preference adapter: `readTodoFilter(): "open" | "done" | "all"` and `saveTodoFilter(filter): void`. Both use try/catch around `localStorage` and fall back to `open`.
  - Views:
    - `renderTabs(activeTab)`, `activateTab(tab: "meetings" | "todos", focus: boolean)` with arrow-key and Home/End handling.
    - `renderTodoFilter(current)` as three `button`s with `aria-pressed`.
    - `renderTodosPanel(todos: Todo[], filter)`, `renderTodoGroup(heading, todos)`, `renderTodoMeetingLink(todo)`, `updateTodosTabCount(n)`.
  - App: `state.activeTab`, `state.todoFilter`, `openTodoInMeeting(todo: Todo): Promise<void>`, and `highlightTodo(todoDocId: string)`, which adds `.todo--highlight` for 2 s (no animation under reduced motion) and focuses the checkbox in `#panel-meetings`.
  - CSS: `.tabs`, `.tab[aria-selected="true"]` imitating `.session[aria-current]`, `.filter`, `.todo-group__heading` imitating `.sidebar__heading`, `.todo--highlight`. The 640px rules keep the tabs in the header and make the todo panel full width.

**Collaborators:** The todos snapshot (5.1, 5.4) also calls `updateTodosTabCount` and, when the Todos panel is visible, re-renders it. 5.4's `patchTodoChecks` keeps both panels' checkboxes in step. `openTodoInMeeting` reuses `selectMeeting`.

**Control flow:**
- Tab activation: set `aria-selected` and `tabindex`, toggle `hidden` on the panels, and render the Todos panel on first open.
- Filter change: save the preference, then re-render the panel.
- `openTodoInMeeting`:
  - Activate the Meetings tab and select the meeting.
  - Await the selected meeting's first render (the snapshot for that id).
  - Then `highlightTodo`.
  - If the meeting no longer exists, show `NO_TODOS_MESSAGE`-style feedback, "That meeting is no longer in the notebook", inline under the link.
- Empty groups are not rendered. When every group is empty, show the filter's empty message.

**Tests:** new `apps/scribe-tray/src/meeting-notebook/todosTab.spec.ts`:
- "the Todos tab shows how many todos are still open" (5.5.1)
- "the tabs are announced as a tab list and switch with the arrow keys" (5.5.1)
- "your own todos come first, then others by name, then unassigned", with an owner `"Unassigned"` and an empty owner both under Unassigned (5.5.2)
- "todos from the newest meeting come first within a group" (5.5.3)
- "the todo list opens on Open and remembers Done for next time" via `storedFilter` (5.5.4)
- "each todo links to its meeting and shows its due phrase and actions" (5.5.5)
- "following a todo's meeting link opens that meeting and focuses the todo", including a meeting beyond the first 20 (5.5.6)
- "ticking a todo in one tab updates the other and the open count" (5.5.7)
- "an empty filter says there is nothing to show" (5.5.8)
- 5.5.9 is covered by CSS, verified in the browser.

**Standards that bite here:**
- WCAG 2.1.1 / 2.4.3 / 4.1.2 (tab pattern keyboard support, focus order, roles).
- Design Standards: carry over every handler and aria attribute when reusing `renderTodo`.
- Clean Code: SRP (core grouping has no DOM).
- Incubyte: complexity of 5 or less (owner grouping via `ownerGroupOf` lookups, not nested ifs).

**Quality checks**

- **Grouping, ordering and filtering live in Notebook-core functions that touch no DOM, and the Todos panel calls them rather than reimplementing them (NFR 6.3.2).**
- No element id is duplicated when the same todo renders in both panels: ids are scoped by `scope` (context trap 5.5-3).
- Owner `"Unassigned"` and an empty owner both land in Unassigned (AC 5.5.2).
- Nothing sorts or groups by `due` ([spec §10.2](../specs/007-meeting-notebook-database.md#10-established-behaviour--documented-not-changed) unchanged).
- The filter preference is never written to the database, and storage failures fall back to Open silently (AC 5.5.4).
- `scroll-padding-top` follows the new header height so the highlighted todo is not hidden (AC 5.5.6).

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green.
2. **User-facing flow.** In the scratch notebook, open Todos, switch filters, reload and confirm the filter is remembered, then follow a meeting link and confirm the meeting opens with the todo highlighted. Repeat at under 640px wide.
3. **Accessibility.** Same session: arrow keys move between tabs, Tab moves into the panel, the filter buttons announce their pressed state, focus lands on the todo checkbox after a jump, and focus is visible throughout.

---

## Slice 5.6 — Show older meetings 20 at a time

**Key decision:** Paging is a pure slice of the ordered, fully loaded meeting index (decision 2). "Show older meetings" increments a page count and appends the next 20 rows to the list and the mobile picker without re-rendering what is there.

**Structure**

- `plugins/scribe/skills/meeting-notes/notebook.html` (modify)
  - Copy: `SHOW_OLDER_LABEL = "Show older meetings"`, `MEETINGS_PER_PAGE = 20`.
  - Core: `pageOfMeetings(ordered: MeetingSummary[], pagesShown: number): { shown: MeetingSummary[]; hasOlder: boolean }`.
  - Views:
    - `buildShowOlderButton(onPress)`, placed in `.sidebar` after both `.meeting-list` and the select wrapper so it shows at every width.
    - `appendMeetingRows(summaries)` and `appendMeetingOptions(summaries)` extend the existing `buildMeetingButtons`/`buildMeetingSelect` output.
  - App: `state.pagesShown` (starts at 1) and `showOlderMeetings()`.
  - `renderMeetingCount` keeps receiving `state.summaries.length`, the total.

**Collaborators:** The index snapshot from 5.1 calls `pageOfMeetings` with `state.pagesShown`. 5.5's `openTodoInMeeting` selects a meeting outside the shown page without changing `pagesShown`.

**Control flow:** press → `pagesShown + 1` → compute the page → append only the rows not yet shown → keep `aria-current` and the select value → hide the button when `hasOlder` is false. Index snapshots keep `pagesShown` and patch the list.

**Tests:** new `apps/scribe-tray/src/meeting-notebook/olderMeetings.spec.ts`:
- "a long notebook shows the newest 20 meetings and offers older ones" (5.6.1)
- "showing older meetings adds the next 20 and keeps the open meeting" (5.6.2)
- "with exactly 20 meetings there is nothing older to show" (5.6.3)
- "the header counts every meeting, not just the ones shown" (5.6.4)
- "the narrow-screen picker offers the same older meetings" (5.6.5)

**Standards that bite here:** Clean Code: small functions; Incubyte: complexity of 5 or less; WCAG 2.4.3 (focus stays on the button or the next new row after a press).

**Quality checks**

- **At exactly 20 meetings no "Show older meetings" button exists in the DOM (AC 5.6.3).**
- The header count is computed from the full index, never from the rows shown (AC 5.6.4).
- Pressing the button never removes or re-creates existing rows, so the selection and scroll position are kept (AC 5.6.2).
- The button is outside `.meeting-list`, so it is reachable under 640px (AC 5.6.5).

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green.
2. **User-facing flow.** Seed 45 meetings in the scratch notebook and press the button twice: 20, then 40, then 45, and the button disappears. Repeat at under 640px with the picker.
3. **Accessibility.** Same session: the button is keyboard reachable, and focus is visible and sensible after each press.

---

## Slice 5.7 — Import old notebooks and upgrade the page

**Key decision:** Add two procedures to `notebook.md`, both reusing 5.2's find step and record shapes:
- **Import** reads each legacy notebook's data island through the artifact read tool, skips meeting ids already in `meeting_index` or seen earlier in this import, and writes one batch per meeting with every todo open.
- **Update** reads the notebook page (required before republishing to a URL) and republishes the unmodified template with the full capabilities from decision 5.

Deleting legacy notebooks is offered per notebook and done only on explicit confirmation.

**Structure**

- `plugins/scribe/skills/meeting-notes/notebook.md` (modify), two new sections:
  - **Import old notebooks.**
    - Find the notebook (create it if absent), and list `meeting_index` ids.
    - For each legacy notebook (titled `Scribe notebook`, refused by the database tool), read it and take the JSON array inside `<script type="application/json" id="scribe-meetings">`.
    - For each meeting not yet seen, map it to the 5.2 records: `todos[].id` → `todo_id`, `done: false`, `done_at: null`, and `meeting_index` from its fields. Send one batch per meeting.
    - Stop at a cap refusal, keeping earlier batches.
    - Reply with the added and skipped counts, then list each legacy notebook with its meeting count and ask which to delete.
    - Delete only the ones named in your answer.
  - **Update the notebook page.** Find the notebook, read its page, and publish `notebook.html` unmodified to the same URL with the decision 5 capabilities. Reply that the page is updated and every meeting is kept.
- `plugins/scribe/skills/meeting-notes/SKILL.md` (modify) — one line under the notebook pointer: "To import old notebooks or update the notebook page, follow the matching section of `notebook.md`." The skill `description` frontmatter gains "import old Scribe notebooks or update the notebook page".
- `plugins/scribe/README.md` (modify) — *Commands*: "Ask Claude to import your old Scribe notebooks, or to update your notebook page after a plugin update."

**Collaborators:** The artifact tool (list, read, publish with URL, delete) and the database tool (list, batch), named generically. Reuses 5.2's sections 1 to 4 for finding, the record shapes and refusal handling.

**Control flow:**
- Import: find → existing ids → for each legacy notebook, for each meeting: skip a seen id, batch-write, and stop on a cap → report the counts → ask about deletes → delete only the confirmed ones.
- Update: find → read the page → publish to the URL → reply.

**Tests:** no automated test (prose). Covered by Verification step 2.

**Standards that bite here:** destructive-action discipline (explicit per-item confirmation; context trap 5.7-1); least privilege (no change to the declared rules except restating them); Incubyte: Conventional Commits.

**Quality checks**

- **No step deletes an artifact without the owner naming it in a reply to the per-notebook question (AC 5.7.3).**
- Imported todos are written with `done: false` and `done_at: null` (AC 5.7.2).
- De-duplication covers ids already in `meeting_index` and ids seen earlier in the same import (AC 5.7.1).
- Update publishes to the existing URL and never creates a new artifact (AC 5.7.4).
- A cap refusal stops the import and keeps the earlier batches (AC 5.7.5).
- No automatic upgrade check is added to the normal `/meeting-notes` run ([spec §9.3](../specs/007-meeting-notebook-database.md#9-deferred-follow-ups-and-accepted-limits) unchanged).
- `claude plugin validate --strict plugins/scribe` passes.

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green.
2. **User-facing flow.**
   - In Claude Code with the plugin, ask to import the old notebooks. Both legacy notebooks' meetings appear once each (the 4 from 21 Oct are shared, so 8 distinct plus any already present), and the counts are reported.
   - Decline deletion and confirm both legacy artifacts remain.
   - Ask to update the notebook and confirm the same URL now serves the current template with every meeting and tick intact.
3. **Accessibility.** Not applicable: no page UI changes.

---

## Hard floors

- **Slice 5.2** — authorization: it declares the database access rules that decide who can rewrite notes and tick todos. Floor tier `≥ excellent`.
- **Slice 5.7** — migrations: it migrates data from legacy notebooks and deletes artifacts on confirmation. Floor tier `≥ excellent`.

## Escalation log

- 2026-10-21 — planning — caught before build, a spec deviation. Decisions 2 and 3 add two record kinds beyond [spec §3.2](../specs/007-meeting-notebook-database.md#32-kinds-of-data): `meeting_index/<id>` for the list and count, and `notebook/meta` as the notebook marker. [Spec §7.4](../specs/007-meeting-notebook-database.md#74-data-flow)'s "paged meeting list" subscription becomes a whole-index subscription. No AC changes. Heal the spec's §3.2 and §7.4 when plan-implementer starts.
- 2026-10-21 — planning — caught before build: [spec NFR 6.3.1](../specs/007-meeting-notebook-database.md#63-clean-code-and-tests-clean-code-incubyte-engineering-standards)'s "rewrite-keeps-done covered by automated tests" cannot hold, because the rule is skill prose. It is met by construction (decision 4) and checked by 5.2's scripted manual run. Heal the NFR wording with the above.
