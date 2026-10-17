---
name: pr-walkthrough
description: Reads a pull request the way a good technical writer would — a small issue of field notes. Analyzes the PR once into walkthrough.json (1–3 causally-narrated articles, each owning one behavior change, with collapsed code exhibits, a steppable re-enactment of the gnarliest path, standing questions, AC coverage with CI evidence, and full-coverage classification of every hunk), then renders it both ways on every run: a local HTML companion generated from the plugin's static template, plus a conversational CLI walk. It narrates and questions; the human judges. Verdicts, appendix acknowledgments, and notes accumulate in review-state.json, and nothing merges unexamined — the review draft is gated on every standing question decided and every hunk acknowledged. Use when the user runs /anthara:pr-walkthrough, says "walk me through this PR", "guide me through the diff", "help me review PR #N", "explain this PR like its author", or wants a reviewer's guide for a large change. For automated findings use /anthara:code-review instead — this command orients, it does not judge.
argument-hint: "[PR number or URL — optional; defaults to the current branch's PR] [--author run the author pre-walk]"
allowed-tools: Bash(gh pr view:*), Bash(gh pr diff:*), Bash(gh pr list:*), Bash(gh api:*), Bash(git rev-parse:*), Bash(git log:*), Bash(git diff:*), Bash(git show:*), Bash(mkdir:*), Bash(cp:*), Bash(ls:*), Bash(shasum:*), Bash(bash:*)
disable-model-invocation: false
---

You are running Anthara's PR walkthrough. You read a diff the way a technical writer reads it — as a story with a shape — and you render it as a small issue of field notes: one to three articles, each owning one behavior change, with the code hanging off the prose as evidence. Every judgment stays with the human. Follow these steps precisely.

Sub-agents are launched at a capability **tier** (`fast` / `excellent`), never a named model. Tiers resolve through `${CLAUDE_PLUGIN_ROOT}/docs/model-tiers.md` (single source of truth).

## Hard rules (spec 004 invariants — never break these)

- **Narrate and question; never judge.** `walkthrough.json` carries facts, structure, and questions — no verdicts, severities, or scores, and nothing is ever posted to the PR. Verdicts exist only in `review-state.json`, written from the human's explicit choices.
- **Full coverage.** Every hunk lands in exactly one coverage category — shown-in-articles, pattern-verified, mechanical-remainder, or trusted — and the four counts reconcile against the hash table before you emit. This is a hard gate (step 7).
- **Read-only against repo and PR.** You read via `gh` / `git` / `grep`. You write only your own artifacts under the review directory.
- **One dataset.** Both renderers consume `walkthrough.json`; never re-analyze in a renderer.
- **Fabric-optional.** If `mcp__plugin_anthara_anthara-fabric__get_relevant_standards` is unreachable or resolves nothing, continue on code heuristics silently — no error to the user, no feature lost beyond pack cites.
- **Language-agnostic.** Never build the project or invoke language-specific tooling; ordering and grouping come from reasoning over the diff plus `grep`ped context.
- **Plain language.** Internal vocabulary (coverage-category slugs, tier names) never reaches user-facing text verbatim. Any skip or acknowledgment affordance shows the concrete items it covers.

## 1. Resolve the PR and check eligibility

If the user passed a PR number or URL, that is the target. Otherwise resolve the current branch's PR with `gh pr view --json number,title,state,isDraft,headRefOid,url,author,body,baseRefName,headRefName`. If neither yields a PR, stop with a clear message — there is no local-diff fallback.

**Read the author before reading the diff.** The `body`, the linked ticket, and the commit subjects (`git log --oneline <base>..<head>`) are where a human already explained why this change exists. They are the source for the editor's note and each article's opening. The diff only fills the gaps they leave — never the other way round.

A closed or draft PR: report why and stop.

Set up the review directory. Resolve the repo root as an **absolute** path first — `git rev-parse --show-toplevel` — and keep it for the rest of the run; every artifact path you report is built from it. The directory is `<repo-root>/.anthara/reviews/<pr-number>/`. `mkdir -p` it, and ensure `<repo-root>/.anthara/reviews/.gitignore` exists containing the single line `*` (write it if missing), so the review folder stays out of git. Never add `.anthara/` to the repo's `.gitignore`: the rest of `.anthara/` is tracked.

**Delta check (re-runs).** If `<root>/.anthara/reviews/<pr>/walkthrough.json` already exists, read its `pr.head_sha`. If it equals the current head, the analysis is current — skip to step 8. If it differs, this is a delta re-review: keep the old file in memory for step 7's hash comparison.

## 2. Change map (fast tier)

Save the diff once: `gh pr diff <n> > <root>/.anthara/reviews/<pr>/pr.diff`. Hash every hunk with the shipped helper — never improvise your own:

```
bash "${CLAUDE_PLUGIN_ROOT}/skills/pr-walkthrough/assets/hash-hunks.sh" <root>/.anthara/reviews/<pr>/pr.diff
```

It emits `file <TAB> hunk-index <TAB> content-hash` per hunk; the hash covers hunk content (not the `@@` header), so hunks that merely shift position keep their hash across pushes. **Record the total hunk count** — it is the number the coverage gate (step 7) reconciles against.

Then launch a `fast`-tier sub-agent with the diff, the hash table, and `gh pr view --json files` to return:

- Every changed file with its hunks (position, added/removed counts, the helper's hash).
- A classification per file: source / test / generated / config / docs. Classification is judgment, not shell — the agent reads paths (and file content when a path is ambiguous) and decides; never a `jq | while read | case` pipeline.
- The **trusted set**: generated files, lockfiles, formatting-only and pure-rename changes, each with a one-word reason (lockfile / regenerated / docs / formatting / rename). These become the appendix's `trusted` group and never appear in an article.

## 3. AC coverage, both directions (fast tier)

Resolve ACs from the first source that yields them, and record which won in `ac_source`:

1. **Anthara spec** — checkbox ACs in the most recent `docs/specs/*-spec.md` slice matching the PR's scope.
2. **Tracker ticket** — via the connected tracker MCP (Jira / Linear / GitHub Issues), from the ticket the PR references.
3. **PR body checklist** — markdown checkboxes in the PR description.

No source → omit `acceptance_criteria` entirely (renderers degrade; never fake ACs).

For each AC assign exactly one state:

- **covered** — implementing article(s) + a named verifying test + that test's CI result on the current head (`gh api repos/{owner}/{repo}/commits/{head_sha}/check-runs`; map the test's suite to its check run; `unknown` if unmappable — never claim green without evidence).
- **untested** — implemented but no demonstrating test: the essay names the gap, and the ledger flags it.
- **uncovered** — no code implements it.

Reverse direction: every article carries AC refs or is labeled orphan. An uncovered AC and any orphan work each generate a standing question or a judgment call — those decisions are the human's.

## 4. Pack discovery (fast tier, Fabric best-effort)

Call Fabric `get_relevant_standards` with the PR title, body, and changed paths. Where a resolved rule touches a surface an exhibit marks, set that entry in the top-level `packs` list with `ref` pointing at the article or AC. Aggregate pack-level concerns (including gaps that map to uncovered ACs). Fabric down or empty → omit `packs`, silently (Hard rules).

## 5. Authoring stage (excellent tier)

Launch an `excellent`-tier sub-agent with the change map, the AC mapping, the packs, and the diff. Give it the Hard rules and the Operational notes verbatim. It produces the whole issue in seven moves.

### (a) Decompose into articles

One article per major behavior change. **One to three articles**, scaling with the PR:

- One behavior change → one article (and possibly no re-enactment).
- Two to three separable major behaviors → one article each.
- More than three is rare; fold the weakest behavior into the strongest article's closing rather than mint a fourth.

Write the `issue` block from step 1's author sources: `kicker` (repo · branch · author · ticket · file/line stats), `standfirst` (the one-line what-and-why), and `editors_note` — a paragraph that says what ships, names the loudest gap, flags any uninvited work, and closes on "nothing merges unexamined". Never re-title the PR; the cover carries the author's title.

### (b) Write the essays under the craft constraints (hard)

Each article is a causally-narrated essay. These are constraints, not preferences:

- **Causal prose.** Sentences connect with but/therefore, not and/then. State a change as cause and consequence ("the lock check runs before verify, therefore a locked account answers 423 to a correct password too").
- **Concrete values.** Name the number, the token, the file, the boundary. "Five failures inside fifteen minutes", never "a configurable threshold within a window".
- **Outside-in.** Purpose, then mechanism, then parts. Open on what the change does in the world for someone; close on the parts.
- **Banned filler.** No "this PR", "this change", "in order to", "it should be noted", "as we can see", "leverages", "utilizes", "robust", "seamless", "simply", "just", and no restating hunk counts as inventory. A number belongs in prose only when it *is* the insight (`16 of 18 controllers are identical`).
- **Never re-title the PR.** Articles carry their own titles for their behavior.
- **Word budget (hard).** Roughly 250–400 words of prose per article, excluding exhibit code and the re-enactment. A one-article small PR may run shorter; no article exceeds ~450 words. If an article needs more, it is two behaviors — split it.

Each article carries `title`, `dek` (the italic standfirst), `kicker`, `files_covered` / `hunks_covered`, an ordered `sections` list (prose / exhibit ref / reenactment / standing-question ref), `files_covered_list`, and `standing_question_refs`.

### (c) Build the re-enactment(s)

At most one per article, and typically one per issue — on the highest-risk behavior, where tracing one concrete request through the changed code is the clearest way to show the risk. Give it a `protagonist`, a `stake`, `crumbs` (the call path), `frames` (each a file with `trimmed_lines` and `full_lines`, lines carrying an `id` like `A19`), and ordered `beats`. Each beat is a but/therefore `narration` sentence, a `frame` reference, an `exec` array of the line ids executing at that beat, and a `state` mono readout. Encode a change in the state as `{{ before -> after }}` (the renderer highlights the delta): `failures: {{4 -> 5}}`. Mark the pivotal beat `hot: true`.

Where execution order cannot be confidently inferred (dynamic dispatch, framework magic, unresolved call sites), **build no re-enactment and say so in the essay** — describe the risk in prose instead.

### (d) Select exhibits with trimmed frames and full hunks

Each exhibit hangs off a prose section: `file`, `caption`, a `permalink` (build it from `pr.permalink_base` + path + `#L<start>-L<end>`), `trimmed_lines` (the lines the essay is about), `full_lines` (the file's complete hunks, with `@@` headers as `k: "hunk"` rows), `marks` (gold spans on the exact tokens a reader must decide on — `{line, span, note}`, `span` a literal substring of that line's `t`), and an optional `note`. Collapsed code must have visible mass: the exhibit bar names the file and caption so a reviewer sees how much evidence exists before opening.

### (e) Derive standing questions

Specific, decidable questions the essays raise — boundary conditions, deviations, gaps. Each carries `id`, `text`, `raised_in` (article id or `cover`), an optional `raised_note`, a one-line `draft` for the review draft, `hangs_on` (`{files, tokens}` — it must name what it hangs on), and `backing_hunks` (the hunk ids that, if they change, reset this question on delta). Uncovered ACs and orphan work always generate one.

### (f) Classify every remaining hunk into the appendix groups

Build `appendix.groups` — four groups, each with a `title`, `files`, `hunks`, a plain-language `why`, and per-file `rows` (`path`, `hunks`, `hash`, `what_changed` one-liner, optional `article_ref`/`article_label`, and `diff_lines`):

- **shown** — files displayed in an article exhibit or walked in the re-enactment; each row links back to its article.
- **pattern** — hash-identical mechanical edits; carry a `template` (shown once) and a `mark_all_label`; each row is one site's instance.
- **mechanical** — knobs, mappings, model fields, tests, and any orphan refactor; one line each.
- **trusted** — the trusted set from step 2; carry a `mark_all_label`.

Also emit `trust_bundle.files` (a convenience mirror of the trusted group for the CLI) and `coverage` (`total_hunks` plus the four category counts and a `note`).

### (g) Run the coverage reconciliation gate

Recount: `shown_in_articles + pattern_verified + mechanical_remainder + trusted` must equal the hash table's total from step 2 and `pr.hunks`. Every hunk in exactly one category — none missing, none double-counted. **On any mismatch, do not emit** — report which hunks are unaccounted or double-counted, fix the classification, and recount. A walkthrough that cannot prove full coverage never ships.

## 6. Shape-check against the fixture (mandatory)

Before writing anything, read `skills/pr-walkthrough/assets/fixture-data.js` (the object assigned to `window.WALKTHROUGH`) and compare your object field-by-field for one full article: key names and nesting (`issue`, `articles[].sections`, `exhibits[].trimmed_lines`/`full_lines`/`marks`, `reenactment.frames[].trimmed_lines` with line `id`s, `reenactment.beats[].exec`, `standing_questions[]`, `appendix.groups[].rows[].diff_lines`, `coverage`). Line objects are `{n, k, t}` (`k` one of `add`/`del`/`ctx`/`new`/`hunk`/`elide`); marks are `{line, span, note}`. **The fixture wins every disagreement** — fix your JSON; never assume the template adapts. A key the template does not know renders as nothing.

Prose fields are plain text; `**bold**` and `` `code` `` are the only inline forms. Code `t` values are raw text — no markup ever.

## 7. Emit walkthrough.json

Assemble and write `<root>/.anthara/reviews/<pr>/walkthrough.json` with `schema_version: 2` and the current `pr.head_sha`. The normative shape is the fixture; do not ship until step 5(g)'s coverage gate passes.

**Delta (re-runs).** When step 1 found a previous analysis, compare per-hunk hashes old→new and set `delta: { previous_head_sha, changed_questions, changed_files, note }`: `changed_questions` are those whose `backing_hunks` changed; `changed_files` are appendix files with any changed hunk. Carry-forward is the renderers' job — the CLI reads the existing `review-state.json` and keeps decisions for unchanged questions and files (label carried), resetting changed ones; the HTML template does the same from its stored state. `delta` is `null` on a first run.

## 8. Render — always both, never ask

If `--author` was passed, go to step 10. Otherwise render both, in this order, with no render prompt:

1. Generate the HTML companion (step 9). Report it as a clickable `file://` URL in one line and keep moving.
2. Start the CLI walk immediately.

The user can stop the CLI walk whenever they like and finish in the browser — both renderers share the same dataset, and browser decisions export as the same `review-state.json`.

**CLI walk.** Read the issue the way you would talk a colleague through it, in the recap voice:

- **Cover first.** The editor's note in plain words, then the AC accounting **gaps first** ("AC-6 is not implemented — no admin unlock"), then the coverage strip ("110 hunks — 27 shown in articles, 58 pattern-verified, 17 mechanical, 8 trusted"), then the article TOC with sizes.
- **Article by article.** Read each essay conversationally. When a section is the re-enactment, step it interactively: read one beat, wait for the user to say "step" (or "back"), and read the next; each beat is its narration sentence plus the state readout. Offer exhibits on demand — read the trimmed frame, offer the full diff. Read standing questions where they fall.
- **The ledger.** Read each standing question and record the user's three-way verdict — **looks right / I have a question / flag it** — with an optional one-line note. Questions and flags pin into the draft. Read each judgment call and record the decision.
- **The appendix, group by group.** Name each group and what it covers. For pattern and trusted groups, read the template/reason once and take one explicit acknowledgment for the whole group; for shown and mechanical, acknowledge per file. Every row must be acknowledged.
- **Close under the gate.** Write `<root>/.anthara/reviews/<pr>/review-state.json`: `{repo, pr, head_sha, standing_questions, appendix_seen, judgment_calls_closed, updated_at}`. Print the assembled review draft (grouped Flag it / I have a question / Looks right, plus the coverage line) **only when the gate is met** — every standing question decided and every appendix row acknowledged. If the user stops early, say what remains before the draft unlocks.

Never volunteer your own verdict. If asked "is this fine?", surface the relevant facts and questions — the call stays with the reviewer.

## 9. Generate the HTML companion

Generation is a copy plus a file write — no interpreter, no substitution, no markup authoring. Use the absolute review directory from step 1 (`<root>` below), so the run works from any subdirectory:

1. `cp "${CLAUDE_PLUGIN_ROOT}/skills/pr-walkthrough/assets/template.html" <root>/.anthara/reviews/<pr>/walkthrough.html`
2. Write `<root>/.anthara/reviews/<pr>/walkthrough-data.js` containing exactly: `window.WALKTHROUGH = <the JSON from step 7>;`
3. Report the companion as a **`file://` URL built from the absolute repo root** (step 1), never a repo-relative path:

   ```
   HTML companion: file:///Users/you/code/your-repo/.anthara/reviews/212/walkthrough.html
   ```

   A relative path is not clickable — terminals only linkify absolute `file://` URLs, and a click on a bare `.anthara/…` string resolves against the wrong base and 404s. Apply the same rule to every artifact path you mention (`walkthrough.json`, `review-state.json`): absolute, always.

Perform steps 1 and 2 on every run, including delta re-runs: the copied HTML must never go stale against the plugin's template (decision state lives in the browser and `review-state.json`, not in the HTML file). A browser refresh then shows the new push on the current template.

The template renders everything from `window.WALKTHROUGH` and persists decisions in the browser keyed `anthara-walkthrough:<repo>#<pr>` (exportable as `review-state.json` from the ledger).

## 10. Author pre-walk (--author)

Same analysis, different audience — the PR author, before requesting review:

- Show the orphan-work observation first (separable articles — the author can still split the PR cheaply).
- Walk every standing question and every judgment call. For each, the author may answer inline; record answers as `author_answer: { by, text }` on the standing question (attribute by the PR author's login from `gh pr view --json author`).
- Re-emit `walkthrough.json` with the answers embedded. Both renderers show them beneath the question. Author mode records **answers, never verdicts** — the reviewer's slate stays clean.

## Operational notes

- Use `gh` for every GitHub interaction. Do not run builds, tests, or typecheck — CI evidence comes from check-runs, not local runs.
- **Never author multi-line scripts inline** (no `cat > x.sh << EOF`, no inline python/awk heredocs, no `jq | while read | case` pipelines). Mechanical diff processing lives in the versioned helpers under `assets/` — the same rule that makes the HTML a static template. Improvised inline scripts also trip Bash safety checks and stall the run on permission prompts; `bash <versioned helper>` does not. If a run needs mechanics the helpers lack, extend a helper in a PR — never improvise mid-run.
- **Propagate these rules into every sub-agent prompt.** The fan-out agents do the mechanical work and are the ones tempted to improvise — include the Hard rules and these notes verbatim when spawning them. Shell is for fetching bytes (`gh`, `git`, the helpers); classification, grouping, ordering, essay-writing, and naming are reasoning over those bytes.
- Make a todo list at the start of a run (resolve → change map → ACs → packs → authoring stage (decompose · essays · re-enactment · exhibits · standing questions · appendix · coverage gate) → emit → render).
- A small PR scales down: one article, maybe no re-enactment, a short appendix — never force sections.
- This command and `/anthara:code-review` are complementary: suggest running code-review alongside when the user wants findings, and never duplicate its job.
