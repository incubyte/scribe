---
name: code-review
description: Automated, confidence-scored code review for a pull request — Anthara's flagship review skill. Fans out specialised sub-agents in parallel to audit CLAUDE.md adherence, scan for shallow bugs, read git blame and history, review prior-PR comments on the touched files, check in-file code comments, and verify integration / dangling-work. When Fabric MCP's `mcp__plugin_anthara_anthara-fabric__get_relevant_standards` resolves active compliance packs for the diff (HIPAA / PCI / SOC 2 / FDA SaMD / WCAG / OWASP / Clean Code), one parallel compliance reviewer per resolved pack joins the fan-out. Every finding is independently confidence-scored 0–100; only findings ≥ 80 are posted as a single `gh pr comment`. Works on any repo, with or without packs configured. Use whenever the user runs `/anthara:code-review`, says "review this PR", "code-review the pull request", "review PR #N", "audit this PR", "review my PR", "run a code review", "compliance-review this change", or has just opened or pushed to a PR and wants automated feedback.
argument-hint: "[PR number or URL — optional; defaults to the current branch's PR] [--min-confidence N — post findings scoring ≥ N; default 80]"
allowed-tools: Bash(gh issue view:*), Bash(gh search:*), Bash(gh issue list:*), Bash(gh pr comment:*), Bash(gh pr diff:*), Bash(gh pr view:*), Bash(gh pr list:*), Bash(gh api:*), Bash(git rev-parse:*), Bash(git log:*), Bash(git blame:*), Bash(git diff:*), Bash(git show:*)
disable-model-invocation: false
---

You are running Anthara's code review on a pull request. This works on any repo — packs configured or not. The review fans out specialised sub-agents in parallel (CLAUDE.md adherence, shallow bug scan, git blame/history, prior-PR review comments, in-file code comments, integration / dangling-work). When Fabric MCP resolves active compliance packs for the diff, one compliance reviewer per resolved pack joins the fan-out. Every finding feeds the same 0–100 confidence pipeline and the same 80-threshold filter before anything is posted.

Follow these steps precisely.

Sub-agents below are launched at a capability **tier** — `fast` or `excellent` — never a named model. Tiers resolve to a concrete model via `${CLAUDE_PLUGIN_ROOT}/docs/model-tiers.md` (the single source of truth).

## 1. Eligibility check

Launch a `fast`-tier sub-agent to check whether the PR (a) is closed, (b) is a draft, (c) does not need a code review (automated bot PR, dependency bump, single-line typo, etc.), or (d) already has a code review comment authored by you on an earlier run. If any of those, **stop here and report why**. Do not proceed.

If the user passed a PR number or URL as an argument, that's the target. Otherwise resolve the PR from the current branch with `gh pr view --json number,title,state,isDraft,url`.

## 2. Collect CLAUDE.md paths

Launch a `fast`-tier sub-agent to return file _paths_ (not contents) of every CLAUDE.md that could apply to this PR:

- The repo root `CLAUDE.md` (if present)
- Any `CLAUDE.md` in directories the PR modifies (walk upward from each changed file to the repo root)

This list feeds both step 4a (CLAUDE.md reviewer) and step 5 (confidence scoring).

## 3. Summarize the PR

Launch a `fast`-tier sub-agent to view the PR (`gh pr view --json title,body,files`, `gh pr diff`) and return a concise summary of the change: what it does, the surface it touches, and any user-facing behavior change. Keep this under 200 words — downstream agents will read it.

## 3.5. Discover active compliance packs (Anthara extension)

Use Fabric MCP's `mcp__plugin_anthara_anthara-fabric__get_relevant_standards` with the PR title, body, and the list of changed file paths as inputs to discover which compliance packs apply. The MCP tool returns one or more packs (e.g. `hipaa`, `pci`, `soc2`, `fda-samd`, `wcag`, `owasp`, `clean-code`) with their rule sets.

If Fabric MCP is unreachable or returns no packs, **fall back** to: scan the repo for an `ARCHITECTURE.md`, the most recent spec under `docs/specs/`, and any project `CLAUDE.md`. If those documents name packs (HIPAA / PCI / SOC 2 / FDA / WCAG / OWASP / Clean Code), use those. If nothing is named, proceed with **only the always-on roles** in step 4 — skip the compliance fan-out entirely. Do not invent packs.

Record the resolved pack list. It controls how many compliance reviewers spawn in step 4.

## 4. Parallel review fan-out

Launch all of the following as **independent, parallel `excellent`-tier sub-agents**. Each returns a list of `{issue, reason}` items — `issue` is a short description of what's wrong, `reason` is why it was flagged (CLAUDE.md adherence, bug, git history, prior-PR comment, in-file comment, integration, or a specific pack rule). The 0–100 confidence scorer in step 5 is what ranks the issues; reviewers do not.

### Always-on reviewers (run every time)

**Agent A1 — CLAUDE.md adherence.** Audit the diff against every CLAUDE.md from step 2. Each finding cites which CLAUDE.md and quotes the specific instruction violated. CLAUDE.md is _guidance for Claude when writing code_ — not every line in it applies during review. Skip rules that are clearly about authoring style rather than the product invariants.

**Agent A2 — Shallow bug scan.** Read the diff only (no surrounding context unless required to confirm a bug). Focus on real bugs that will hit in production: off-by-one, null/undefined deref, races, mishandled errors, broken invariants, wrong return values. Skip nitpicks, style preferences, missing tests as a general complaint, and anything a linter or typechecker would catch.

**Agent A3 — Git history context.** Run `git blame` and `git log` on the modified files. Find bugs the diff introduces _in light of history_ — e.g. a guard removed that was added to fix a prior incident, a regex tightened that was loosened deliberately, a workaround re-removed.

**Agent A4 — Prior-PR comments.** Use `gh pr list --state merged` and `gh api repos/{owner}/{repo}/pulls/{N}/comments` to find prior PRs that touched the same files. Read review comments on those PRs; flag any that also apply to the current diff (recurring issues the team has called out before).

**Agent A5 — In-file comments.** Read comments inside the modified files (NOTE / TODO / WARN / "do not …" / "must …" / `@invariant` / etc.). Flag any change that violates guidance written into the code.

**Agent A6 — Integration & dangling work (Anthara always-on).** Apply review-craft's integration focus: any new component, route, handler, subscriber, background job, config key, env var, or feature flag introduced by the diff must be referenced from a code path that actually runs in production — not just defined. Examples of dangling work: a UI component never mounted, a webhook handler with no route registration, an env var added to schema but never read, a feature flag defined but never checked, a spec slice's `Reached from` Host with no static evidence in code that it leads to the slice's Entry (older specs write this as a `<Host> → <Entry>` Reachability line).

### Compliance reviewers (one per pack from step 3.5)

For each pack returned by `mcp__plugin_anthara_anthara-fabric__get_relevant_standards`, spawn one `excellent`-tier sub-agent. Hand the agent:

- The PR summary (step 3)
- The diff
- The pack's rule set (returned by `mcp__plugin_anthara_anthara-fabric__get_relevant_standards`)
- The active spec under `docs/specs/` (if present) — to read locked decisions and bound controls

Each agent walks every rule in the pack and checks the diff against it. Findings include:

- **Presence violations** — code does something the pack forbids (e.g. plaintext PHI in logs → HIPAA 164.312(b); secrets in source → OWASP A07; storing CVV → PCI 3.2).
- **Absence findings** — code does _not_ do something the pack requires on this surface (e.g. missing audit log on a regulated mutation → HIPAA 164.312(b); missing access-control check on a protected route → OWASP A01; missing focus management on a modal → WCAG 2.4.3).

Each compliance finding **must** include `rule_cite` as a short, ASCII-only identifier:

- **Standards with formal IDs.** Use the standard's own citation form. Examples: `HIPAA 164.312(b)`, `PCI 3.4`, `OWASP A01`, `WCAG 1.4.3`, `SOC 2 CC6.1`, `FDA SaMD 5.6`.
- **Clean Code** has no external IDs. Cite the principle by short name from this fixed set: `Clean Code: SRP`, `Clean Code: OCP`, `Clean Code: LSP`, `Clean Code: ISP`, `Clean Code: DIP`, `Clean Code: DRY`, `Clean Code: YAGNI`, `Clean Code: KISS`, `Clean Code: naming`, `Clean Code: function-length`, `Clean Code: error-handling`, `Clean Code: encapsulation`. If the issue touches a Clean Code concern not in this set, omit the bracketed cite and quote the relevant CLAUDE.md line instead (use the `(CLAUDE.md says "...")` form).

Long prose rule names go in the body of the finding, not in the cite.

Every compliance reviewer additionally consults Fabric's project memory via `mcp__plugin_anthara_anthara-fabric__search_memories`. Run all four queries; findings feed step 5 like every other finding (no severity labels — the scorer ranks). Skip the queries silently if Fabric is unreachable.

- `mcp__plugin_anthara_anthara-fabric__search_memories("[DECISION", scope="shared", limit=20)` — locked decisions the diff deviates from.
- `mcp__plugin_anthara_anthara-fabric__search_memories("[COMPLIANCE-CONTROL pack=<this-pack>", scope="shared", limit=20)` — bound controls the diff touches without producing the evidence the control requires.
- `mcp__plugin_anthara_anthara-fabric__search_memories("[ARCHITECTURE", scope="shared", limit=1)` — the project's locked architectural style; flag dependency-direction or pattern drift.
- `mcp__plugin_anthara_anthara-fabric__search_memories("[BOUNDARY", scope="shared", limit=10)` — module boundaries the diff crosses inappropriately.

## 5. Confidence scoring

For **every** finding from step 4, launch a parallel `fast`-tier sub-agent that takes:

- The PR diff and summary
- The finding (description + reason + reviewer source)
- The CLAUDE.md path list (from step 2) and, for compliance findings, the pack's rule text (from step 3.5)

The scorer assigns a single integer 0–100. Use this rubric verbatim:

- **0** — Not confident at all. False positive that doesn't survive light scrutiny, or a pre-existing issue.
- **25** — Somewhat confident. Might be real, may also be a false positive. Couldn't verify it's a real issue. If stylistic, it was not explicitly called out in the relevant CLAUDE.md or pack rule.
- **50** — Moderately confident. Verified the issue is real, but it might be a nitpick or rare in practice. Relative to the rest of the PR, not very important.
- **75** — Highly confident. Double-checked; very likely a real issue that will be hit in practice. The existing approach is insufficient. Directly impacts functionality or is directly mentioned in the relevant CLAUDE.md / pack rule.
- **100** — Absolutely certain. Double-checked; definitely real and will happen frequently in practice. Evidence directly confirms it.

For CLAUDE.md findings: the scorer must verify the cited CLAUDE.md _actually_ calls out this specific issue. If the CLAUDE.md text doesn't directly support the finding, score ≤ 50.

For **compliance findings**: the scorer must verify the cited pack rule _actually says_ what the finding claims it says. If the rule text doesn't support the finding, score ≤ 50. Compliance findings score ≥ 75 only when the rule unambiguously applies to the code surface touched by the diff _and_ the gap is concrete (a specific missing audit event, a specific input not validated, a specific permission not checked) — not a generic "audit logging should exist somewhere".

## 6. Filter

The minimum posting confidence is **80**, unless the invocation arguments include `--min-confidence N` (an integer 0–100) — then the minimum is N. Drop every finding scoring below the minimum. The scoring rubric in step 5 never changes; only this posting threshold does. If nothing remains, post the "no issues" comment in step 8 — do not skip the comment, the user expects acknowledgement.

## 7. Re-check eligibility

Launch a `fast`-tier sub-agent to repeat the eligibility check from step 1. PRs can close, draft, or be reviewed by a teammate while the fan-out runs. If the PR has become ineligible, stop and report — do not post.

## 8. Post the review comment

Use `gh pr comment <N> --body-file -` (or `--body <…>`) to post **one** comment in the format below. Honour these rules:

- Keep the comment brief. No headers beyond what the template specifies.
- No emojis.
- The trailing 🤖 attribution line and the reaction-request line are default-on. Omit **both** when the invoking context (workflow prompt, caller instructions) explicitly says not to post an attribution footer.
- When the minimum confidence (step 6) is below 80, group the numbered findings under two bold lines — `**High confidence (score ≥ 80)**` first, then `**Lower confidence (score < 80)**` — and append each finding's score in parentheses at the end of its description, e.g. `(confidence 65)`. At the default threshold of 80, keep the flat numbered list.
- Every finding **must** link to a code permalink with full SHA + line range (see the link format below).
- Compliance findings carry the pack rule cite inline, parenthesized, the same shape CLAUDE.md citations use.
- If multiple findings collapse to the same root cause, post once with both cites in the parentheses (e.g. `(HIPAA 164.312(b) + OWASP A09 say "<…>")`).

### Comment template — issues found

```
### Code review

Found <N> issues:

1. <brief description of bug> (CLAUDE.md says "<verbatim quote>")

<permalink>

2. <brief description of compliance gap> (HIPAA 164.312(b) says "<verbatim quote>")

<permalink>

3. <brief description of bug> (bug due to <file:symbol and short reason>)

<permalink>

🤖 Generated with [Claude Code](https://claude.ai/code)

<sub>- If this code review was useful, please react with 👍. Otherwise, react with 👎.</sub>
```

### Comment template — no issues

```
### Code review

No issues found. Checked for bugs, CLAUDE.md compliance, and active compliance packs (<comma-separated pack list, or "none" if step 3.5 found nothing>).

🤖 Generated with [Claude Code](https://claude.ai/code)
```

### Permalink format — exact

```
https://github.com/<owner>/<repo>/blob/<full-sha>/<path>#L<start>-L<end>
```

- Full SHA only. Do **not** wrap `$(git rev-parse HEAD)` — the rendered Markdown will not execute shell. Resolve the SHA yourself (`gh pr view --json headRefOid` for the PR head) and embed it as a literal string.
- `#L` notation, not `:`.
- Provide at least one line of context above and below the cited line range (e.g. for a bug on line 5–6, link `L4-7`).
- Repo owner / name must match the PR's repo, not the parent.

## What counts as a false positive (filter aggressively)

- Pre-existing issues — only flag what _this_ diff introduces or changes.
- Looks-like-a-bug-but-isn't.
- Pedantic nitpicks a senior engineer wouldn't raise.
- Issues a linter / typechecker / compiler will catch (missing imports, type errors, formatting). CI runs those separately — don't duplicate.
- Generic quality complaints (insufficient test coverage in general, "could use better documentation") unless CLAUDE.md or a pack rule explicitly demands them on the touched surface.
- CLAUDE.md / pack rules that the code explicitly silences with an inline ignore comment (`// eslint-disable …`, `// hipaa-ignore: …`).
- Changes in functionality that are likely intentional and directly related to the PR's stated purpose.
- Real issues on lines the user did not modify in this PR.

## Anthara additions to the false-positive list

- Findings from a compliance reviewer whose pack does **not** actually apply to this surface (e.g. a WCAG reviewer flagging a backend-only PR). The pack list comes from `mcp__plugin_anthara_anthara-fabric__get_relevant_standards` — if the diff is clearly outside the pack's scope, score ≤ 25.
- Compliance findings that restate a generic principle without identifying a concrete surface (e.g. "should log audit events" with no specific mutation pointed to) — score ≤ 50.
- Pattern-coherence findings about code outside the diff — informational only, do not post.

## Operational notes

- Use `gh` for every GitHub interaction (fetch PR, diff, blame, comments, posting). Do **not** use `WebFetch` for PRs.
- Do **not** run the build, tests, or typecheck. CI handles those — they are not in scope.
- Make a todo list at the start of the run (eligibility → CLAUDE.md paths → summary → pack discovery → fan-out → scoring → filter → re-check → post).
- Every cited bug **must** include a permalink. A finding without a permalink is not posted.
- The full SHA in permalinks must be the PR head SHA at the moment of posting. Re-resolve it just before formatting the comment so it matches the latest push.
- One comment per run. Never split into multiple comments.
