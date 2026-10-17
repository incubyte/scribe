---
name: plan-implementer
description: Canonical plan executor. Implements an execution plan one merged build agent per slice (code + tests together, at each slice's plan-assigned model tier), then reviews and verifies once at the end. Use when the user runs /anthara:plan-implementer, says "implement the plan", "build the plan", "run the plan", or has been handed off from /anthara:planner. Reads a plan at `docs/plans/<NNN>-<slug>-plan.md`.
argument-hint: <plan path, or empty to find the most recent plan under docs/plans/>
allowed-tools: Read, Bash, Agent, Skill, Glob, Grep, mcp__plugin_anthara_anthara-fabric__get_relevant_standards
---

# /anthara:plan-implementer

Turn a finished plan into reviewed, verified code in three moves: **implement → review → verify**. The plan and spec already did the thinking; this skill executes cheaply.

One `slice-builder` spawn per slice at the slice's plan-assigned tier, then — once, at end-of-spec, over the whole diff — a single specialized reviewer plus one `pack-reviewer` per active compliance pack, all in parallel. No per-slice reviewer fleet. No separate build-then-review split per slice.

## Step 1: Locate the plan and its artifacts

- Plan: the argument, or Glob `docs/plans/*-plan.md` and take the most recent (confirm via `AskUserQuestion` if several).
- From the plan's filename stem (`<stem>-plan.md`), locate the spec (`docs/specs/<stem>-spec.md`) and context file (`docs/specs/<stem>-context.md`). The stem may or may not carry a `<NNN>-` prefix — it follows the repo's convention, and the three files share it verbatim.
- Ensure a context file exists at the current HEAD. If missing or stale (HEAD shifted since it was stamped), spawn the existing `code-orienteer` agent to (re)generate it — same reuse contract `/anthara:plan-implementer` uses. Do not write orientation logic here.
- Read `${CLAUDE_PLUGIN_ROOT}/docs/model-tiers.md` and resolve the `excellent` tier to its concrete subagent `model` value.
- Read the spec's `## Changelog` and the plan's `## Escalation log`. When one records a deviation the other does not, the two artifacts disagree about the current approach — `AskUserQuestion`: re-run `spec-heal` for the missing side / re-plan via `/anthara:planner` / stop. Never proceed on either file's original text. Skip when the plan header says `Spec: none`.
- The plan has two parts: `# Part 1 — Review this` (the approach, decisions, and risks in plain language — useful orientation) and `# Part 2 — Build detail` (the execution contract: shared conventions, per-slice entries, hard floors, escalation log). Read both; **execute from Part 2** — nothing load-bearing lives only in Part 1, and its slice one-liners are never slice definitions. Facts marked `[inferred]` in Part 2 are unconfirmed — the builder verifies them before relying on them; `[verified]` facts were confirmed by the planner.

Surface a one-line summary — *"Implementing plan `<NNN>-<slug>` — N slices, each on its plan-assigned tier, review at end."*

## Step 2: Implement (one spawn per slice, at the plan's per-slice tier)

Build slices **in plan order, one spawn per slice**, each at that slice's plan-assigned model tier. A sub-agent's model is fixed for its whole run, so honoring the plan's per-slice tiers requires one `slice-builder` spawn per slice — this is the mechanical binding that keeps model choice driven by the plan, not improvised at spawn time.

The next slice to work is the first in plan order with any unticked criterion, not simply the next one you have not visited. A slice whose criteria were unticked mid-run has a gap between its code and its criteria: the builder reads what already exists and closes it, rather than starting the slice over. This is also how a run resumes after an interrupt — the spec's checkboxes are the state, and there is no separate state file.

For each slice, in plan order:

1. **Read the slice's tier from the plan** (planner field 8 — `Agent → model tier`; for the implementer use the tier the plan assigned the slice's build agents). Resolve that tier alias to its concrete subagent `model` value via `${CLAUDE_PLUGIN_ROOT}/docs/model-tiers.md` — the single source of truth for the tier→model mapping (never restate the mapping here).
2. **Spawn `slice-builder`** (Agent tool, `subagent_type: anthara:slice-builder`, **`model` = the value resolved in step 1 — pass it explicitly; never inherit the orchestrator's own session model**). Pass: the slice ID, the plan path, the spec path, the context-file path, and `ARCHITECTURE.md` / `design.md` when present.
3. The agent implements **that one slice**: writes the code designed for testability, writes the plan-specified tests, runs the slice's scoped test set to green, ticks the slice's ACs, commits the slice. It does **not** review — that is Step 3. Wait for it to hand back, then proceed to the next slice.

**The model is data from the plan, not a choice made here.** Do not default to the session's model. Do not run every slice on the same tier unless the plan assigned every slice the same tier. Each slice runs at the tier the plan assigned it, resolved through `${CLAUDE_PLUGIN_ROOT}/docs/model-tiers.md` — a `fast` slice and a `best` slice run on different models even within the same build.

Pass any deviation already recorded for a slice to every later spawn of it, so the build agent reads the `## Escalation log` override rather than the original entry.

If a hand-back proposes a deviation, or the user redirects the approach between slices, run **Deviations** below before spawning anything else.

If a slice cannot reach green within its retry budget: surface the diagnostic and `AskUserQuestion` — re-spawn the implementer to retry the failing slice / accept partial and proceed to review on what landed / cancel. Do not build a later slice on top of a failed earlier one.

## Deviations (any time during Steps 2-4)

An approach change agreed mid-run must reach the artifacts before the next slice is built, or a later session reads the original approach and reverts it. You do not write the artifacts yourself — `spec-heal` does.

- **Sources.** A user redirect — the common case. It always lands here, in the main session: a spawned `slice-builder` runs cold and never hears the user. Also a HIGH finding in Step 4 whose fix changes a field the plan named for its slice.
- **Gate.** One `AskUserQuestion` — *accept and record* / *keep the plan, retry the slice as planned* / *cancel the run*. One call per deviation; fold any open sub-question into the same call rather than taking a second turn.
- **Spawn.** On accept, spawn `spec-heal` (Agent tool, `subagent_type: anthara:spec-heal`, `model` = the `fast` tier resolved via `${CLAUDE_PLUGIN_ROOT}/docs/model-tiers.md`). Pass the slice id, the spec path (or `none`), the plan path, and the change as **before → after, why, criteria affected, slicing changed**. It is a cold context: it writes what you tell it, so a vague summary produces a vague record.
- **Verify.** Re-read the spec's `## Changelog` and the plan's `## Escalation log`. If either lacks the entry, surface it and stop — do not build the next slice on an unrecorded change.
- **Continue.** Resume the slice loop, which now reads the healed artifacts. When the change altered the slicing itself, hand off to `/anthara:planner` via `Skill` (context note carrying `Plan:`, `Spec:`, the recorded deviation, `Process tier`, `Risk`) and stop; the planner alone re-authors Part 1 and Part 2.

## Step 3: Review at the end (single reviewer + one pack-reviewer per active pack, once)

Run review **once** over the whole diff — never per slice.

**First, pull the relevant compliance packs.** Call `mcp__plugin_anthara_anthara-fabric__get_relevant_standards` (the repo's raw remote origin URL, task slots built from the diff's surface — summary, affected file-path globs, data touched) and partition the returned rules by pack (`hipaa`, `owasp`, `wcag`, `pci`, `clean-code`, others). Each pack with ≥1 rule gets its own compliance reviewer. When fabric is unreachable, spawn no pack-reviewers and note `compliance: SKIPPED — fabric unreachable` in the run summary — never drop compliance silently.

**Then spawn all reviewers in one message so they run in parallel:**

- **Spawn `reviewer`** (Agent tool, `subagent_type: anthara:reviewer`, `scope=all`). Pass: `mode=first-pass`, the final diff (`git diff` against the pre-implementation marker), paths to spec / ARCHITECTURE.md / context, and the **plan's hard-floor markings** (which slices/surfaces are floored) so the reviewer enforces security + architecture unconditionally on floored surfaces at a tier ≥ `excellent`. It covers the always-on dimensions in one pass: architecture, integration, code-quality, test-quality, ai-ergonomics, accessibility/design.
- **Spawn one `pack-reviewer` per pack with ≥1 rule** (Agent tool, `subagent_type: anthara:pack-reviewer`, `focus=<pack>` where `<pack>` is the pack's name exactly as `get_relevant_standards` returned it (e.g. `Health Tech Compliance`, never a short label of your own), `scope=all`). Pass: `mode=first-pass`, the pack's rules verbatim as `get_relevant_standards` returned them (every rule's `rule_id`, full text and pack name, unedited: the reviewer cites the `rule_id` of each rule it flags, and fabric links the finding to the org's rule by that id), the final diff, and paths to spec / ARCHITECTURE.md / context. Each walks every rule in its pack against the diff.

Surface a one-line status — *"Reviewing: always-on reviewer + compliance packs <list> (N rules)."* — then wait for all reviewers to hand back findings. Do not aggregate or dedupe by hand: every review source ends with an `anthara.findings/v1` block, and the findings hook validates it, merges findings that share a fingerprint, and records each incident `F<n>` in this session's ledger `.anthara/findings/<YYYY-MM-DD>-<session-id-8>.json` (kept out of git by its own `.gitignore`). Review sources may run in the foreground or in the background (`run_in_background: true`); the hook records every pass when the source stops, either way. The exact `[Anthara-Fix #F<n>] …` todo subjects come from the hook: a foreground hand-back returns them in its hook context; for a background source, read them from the `todo_subject` field of each ledger incident that has no `task_id` yet, and the Stop gate also lists every such subject verbatim. Never invent or reword a subject — a todo without its hook-issued `#F<n>` is not tracked. Work them HIGH first. A source listed under the ledger's `unparsed` returned no valid block even after one retry; name it in the run summary.

## Step 4: Verify and resolve

- Run the full test suite once as a Bash one-shot — the end-of-spec gate (per `test-execution`). It catches cross-slice damage the per-slice scoped runs cannot.
- **Browser verification (only when a slice's Verification carries the user-facing-flow step).** A UI slice is one whose plan Part 2 entry's **Verification** includes steps 3–4; for a plan authored before Verification lived in the plan, read the spec's slice Verification block instead. After the suite gate, spawn `browser-verifier` (Agent tool, `subagent_type: anthara:browser-verifier`, `mode=first-pass`). Pass: the plan path, the spec path, the UI slice IDs to verify, the context file (its §8a Design system), and the base URL of the already-running app. It drives the running app in a browser (chrome-devtools-mcp primary, Claude-in-Chrome fallback) and returns findings in the reviewer's severity shape plus a persisted report at `docs/specs/<NNN>-<slug>-verification.md`. Skip this entirely for backend-only / MCP-only specs. If no browser MCP is connected or the app is not running, it degrades to a manual checklist and reports `SKIPPED` — surface that in the summary; do not block hand-off.
- **Act on findings — reviewer, pack-reviewer, and browser-verifier findings share ONE resolution loop, driven by the hook-returned `[Anthara-Fix]` todos:** create exactly the todo subjects the hook issued (hook context, the ledger's `todo_subject` fields, or the Stop gate's list — per Step 3), one per incident, before starting any fix; a background re-review or re-verify issues the subjects of its new incidents the same way. HIGH → re-spawn `slice-builder` (or surface to the user) to fix, complete its todo, then **re-run the source that raised it** — the affected reviewer(s) (`reviewer` or the flagging `pack-reviewer`) in `mode=re-review` over the same full scope it reviewed first time (never scoped down to the fixed finding — the ledger reads every finding that source omits as cleared; the findings hook appends the run's incident list to every review agent's prompt, so the re-run names each incident it restates by id — never paste that list yourself), or the `browser-verifier` in `mode=re-verify` (naming just the fixed finding) for a browser finding; MED → fix if trivial and complete its todo, else edit the todo's subject to end in `(deferred)`, complete it, and record it in the spec's `## 9. Deferred follow-ups and accepted limits` as a deferred defect — a known problem nobody is awaiting an answer on is not an open question; add §9 when the spec lacks it (added-comment findings are always trivial — remove the comment via rename/extract, never defer it). After a batch's MEDIUM fixes are done, re-run each source that raised one of them once, in `mode=re-review` (or `mode=re-verify` for the browser-verifier), so a trivial fix can reach `fixed` rather than stopping at `claimed_fixed`; LOW → create its todo, then complete it immediately with the subject edited to end in `(deferred)`. Never delete a fix todo except to drop it deliberately — completing with `(deferred)` is how a finding is set aside.
- **When the session has no TaskCreate tool** (the default on Opus 5 and Sonnet 5 and later, unless the user opts in with `CLAUDE_CODE_ENABLE_TODO_TOOLS=1`), run the same loop without todos. Fix the finding and re-run its source as above: the ledger marks an incident with no todo `fixed` once every source that raised it re-runs without reporting it, provided its file changed after the finding was first seen. Defer a finding by ending your reply with one line per incident, `[Anthara-Defer #F<n>] <reason>`, or drop a false positive with `[Anthara-Drop #F<n>] <reason>`; the Stop hook records each as `deferred` or `dropped`. Still record deferred MEDIUM findings in the spec's §9.
- **Close every source once at the end.** After the last fix, re-run every source that raised any incident still `open` or `claimed_fixed` — including sources that raised only LOW findings — once, in `mode=re-review` (or `mode=re-verify`), so no fixed finding is left unconfirmed.
- **A browser finding is closed only when re-verification passes** — never on "code looks right" or a green unit suite alone; that is the point of verifying in a browser. If re-verification cannot execute (app down, browser MCP disconnected, rebuild didn't settle), the verifier returns `unverified (fix applied, not confirmed)`; the incident stays `claimed_fixed` — claimed, not verified, never `fixed` — and you list it explicitly as unverified in the summary — never auto-pass it.
- A HIGH fix that changes a field the plan named for its slice is a deviation: run **Deviations** before the fix is built, not after.
- Surface the run summary: slices completed, ACs ticked, test result, browser-verification result (or `SKIPPED`/`UNVERIFIED` items), the findings ledger's `totals` line and path quoted verbatim (never recounted by hand), what (if anything) remains.

## Hand off

Tell the user: build complete (or stopped at slice X), suite status, the findings ledger's `totals` line and path quoted verbatim from the ledger (never recounted), branch ready for manual review / PR. Do not pronounce it ready to merge — the user owns that call.

A recap is available on demand via the `recap` agent when the user asks for a walkthrough; do not spawn it unprompted.

## Notes

- One `slice-builder` per slice at its plan-assigned tier; one `reviewer` plus one `pack-reviewer` per active compliance pack, once at end-of-spec over the whole diff, all in parallel — no per-slice reviewer fleet, no separate build-then-review split per slice.
- Browser verification (`browser-verifier`) also runs once at end-of-spec, only for UI specs, over the running app — the browser analogue of the single end-of-spec reviewer. Its findings share the reviewer's resolution loop; a browser finding closes only when re-verification confirms it in the browser (never on unit-green alone). It assumes the dev server is already running and degrades to a manual checklist (never a silent pass) when no browser MCP is connected.
- Each slice's build runs at **its plan-assigned tier** (a sub-agent's model is fixed per run, so the per-slice tier is honored by spawning once per slice). The tier is read from the plan and resolved through `${CLAUDE_PLUGIN_ROOT}/docs/model-tiers.md`, never the orchestrator's own session model — that is what keeps a `fast` slice on its own tier even when the orchestrating session runs at the `best` tier.
