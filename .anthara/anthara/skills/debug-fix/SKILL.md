---
name: debug-fix
description: Systematic debugging for regulated codebases. Establishes a tight red/green reproduction loop before theorising, checks Fabric org memory for known gotchas, diffs the broken path against a working analogue, ranks 3-5 falsifiable hypotheses and probes them one variable at a time, audits the root cause before fixing, lands the fix behind a failing regression test with compliance guardrails, and persists a diagnosis record. All debug instrumentation is tagged and provably removed, and never carries PHI, payment data, or credentials. Use when the user runs /anthara:debug-fix, says "debug this", "why is this failing", "fix this bug", "tests are broken", "this endpoint returns 500", or describes a specific failure symptom they want diagnosed.
argument-hint: <symptom description, error message, failing test name, or URL to a failing CI run>
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, AskUserQuestion, Agent, Skill, mcp__plugin_anthara_anthara-fabric__search_memories, mcp__plugin_anthara_anthara-fabric__add_shared_memory, mcp__plugin_anthara_anthara-fabric__get_relevant_standards
---

# /anthara:debug-fix

Systematic debugging for regulated codebases. Read and analyse first, change code last. Almost every debugging failure comes from jumping to a fix before understanding the problem — so this skill makes the understanding checkable at three points and refuses to proceed past them.

## Three gates

These are not guidelines. Each one blocks the next phase of work.

| Gate | You may not | Until |
|---|---|---|
| **1 — Loop** | form a hypothesis or read for a cause | one named command exists, has been run, and went red on the reported symptom |
| **2 — Hypotheses** | run a probe or touch product code | 3-5 ranked hypotheses exist, each with a mechanism and a falsifiable prediction |
| **3 — Cause** | write the fix | the root cause is stated in one sentence, names the code that produces it, and has survived an audit |

State plainly when you pass a gate. If you cannot pass one, that is the finding — report it rather than working around it.

## Hard constraints

Absolute, and they hold even mid-investigation, even locally, even "just for a minute":

- **No regulated content in instrumentation.** No ePHI, payment data, credentials, tokens, whole request or response bodies, or full query strings — in a log line, an error message, a test fixture, or a diagnosis record. Log identifiers, event names, and shape. Details in `references/instrumentation.md`.
- **No control weakened to gain visibility.** Never bypass an auth guard, disable row-level security, loosen validation, skip signature verification, or widen a permission as a debugging shortcut.
- **Read-only against data.** `SELECT` only, on every environment. Never `UPDATE`, `DELETE`, `DROP`, `ALTER`, or `TRUNCATE` while investigating.
- **All instrumentation is temporary and provably removed.** Tagged going in, grepped to zero coming out.

## Red flags — stop and restart

Read these before you begin, not after. Each one means the process has already gone wrong; return to the step named.

| Red flag | Return to |
|---|---|
| Editing product code before the loop is red | Step 3 |
| Reasoning from the first line of the stack trace only | Step 4 |
| Holding exactly one explanation | Step 6 |
| A "hypothesis" with no prediction that could refute it | Step 6 |
| Changing two things at once | Step 7 |
| Proposing a fix you cannot explain in one sentence | Step 8 |
| Declaring it fixed because the code looks right | Step 10 |
| Widening a timeout, adding a sleep, or a retry to make a failure stop | Step 6 — this is a symptom fix |
| Making the broken path match a working one without knowing why | Step 8 |
| "One more attempt" after two failed fixes | the escalation ladder |
| Refactoring, tidying, or improving code adjacent to the fix | Scope discipline |
| A `DEBUG-` tag still in the tree | Step 10 |

## Rationalisations

The reasoning that precedes most bad debugging, and what is actually true:

| The thought | The reality |
|---|---|
| *"The error message tells me what's wrong."* | It tells you where it surfaced. The cause is upstream of that, usually in a different file. |
| *"I don't need to reproduce it, I can see the bug."* | Then you cannot prove the fix worked, and you will not know if you fixed something else. |
| *"Reproducing this properly will take too long."* | Longer than the three speculative fixes that follow? Build the loop. |
| *"It's obviously the cache / the race / the environment."* | A class of cause is not a cause. Name the mechanism or discard it. |
| *"I'll add logging everywhere and see what shows up."* | Untargeted logging produces volume, not signal — and in this codebase it produces risk. One probe, one question. |
| *"Let me try this and see."* | Trial-and-error is the last resort, not the first. If you are guessing, go back and read. |
| *"The test is probably flaky."* | Five runs and a classification, or it is not flaky — it is intermittent and you have not found the variable. |
| *"This is close enough, tests pass."* | Tests passing is necessary, not sufficient. A symptom fix passes tests too, and comes back. |
| *"I'll clean up the debug logs before committing."* | That intention is not a mechanism. Tag them and grep to zero. |

## The loop

Copy this checklist and tick items as you complete them:

```
Debug progress:
- [ ]  1. Symptom understood and classified
- [ ]  2. Org memory checked for known gotchas
- [ ]  3. GATE 1 — tight loop established (one command, run, red)
- [ ]  4. Everything read; assumptions verified; regression window found
- [ ]  5. Working analogue found and diffed (or absence stated)
- [ ]  6. GATE 2 — 3-5 ranked hypotheses with predictions, shown to the developer
- [ ]  7. Probed one at a time; ledger updated
- [ ]  8. GATE 3 — root cause articulated and audited
- [ ]  9. Regression test written and failing; fix written; standards honoured
- [ ] 10. Verified; instrumentation grepped to zero; record written
```

### Step 1 — Understand the symptom

Classify what was reported:

- **Error signals** — stack traces, error codes, HTTP status codes, exception names, log lines.
- **Behavioural signals** — *"page is blank"*, *"button does nothing"*, *"data is stale"*, *"login redirects to the wrong place"*.
- **Test signals** — test name, assertion message, `FAIL` output.
- **CI signals** — a failing run URL, the workflow, and the stage that failed.
- **Regression signals** — *"this worked yesterday"*, *"broke after the deploy"*, *"started after PR #N"*. These unlock bisect in Step 4.

If the symptom is too vague to build a loop from, ask exactly one closed question via `AskUserQuestion`: the exact error text, or the failing test name.

### Step 2 — Consult org memory

Before investigating code, search Fabric for prior knowledge about this failure area:

```
mcp__plugin_anthara_anthara-fabric__search_memories("<service> <error keyword>")
mcp__plugin_anthara_anthara-fabric__search_memories("<module> gotcha")
mcp__plugin_anthara_anthara-fabric__search_memories("<test file> flaky")
```

Look for prior incidents with the same cause, documented architectural traps (DI wiring, interceptor races, seed-versus-build conflicts, secret rotation), and environment-specific divergence.

A memory hit is a **ranked hypothesis, not a verdict** — it enters Step 6 at the top of the list and still needs a prediction and a probe. Memory can be stale; the code may have moved. Never skip the loop in Step 3 because memory looked confident.

If fabric is unreachable, note it and continue.

### Step 3 — Establish the tight loop · GATE 1

Build one named command that is fast, deterministic, and red on this bug and green when it is fixed. Run it. Show the red output. Then minimise it until it is red for exactly one reason.

Read **`references/reproduction.md`** for the repro ladder, the minimisation technique, flake classification, and the CI-versus-local checklist.

If you cannot build a loop, stop and follow that file's hand-back protocol. Do not fix an unreproducible bug.

### Step 4 — Read everything

Before theorising:

- **The full error** — the entire trace, every frame, and any `cause` chain. The first line is where it surfaced.
- **The logs** — application, server, build. Read the lines before the failure, not just the failure.
- **The browser** — for UI symptoms: console, network status codes and bodies, the DOM state. Load `browser-verification`.
- **The code** — trace the real execution path from entry point to failure. Read it; do not infer it from names.
- **The test** — its setup, act, and assert. Sometimes the test is wrong and the code is right.
- **The regression window** — if this is a regression, run `git bisect run` with the Step 3 command rather than reading commits one by one. Then read the introducing PR's stated intent.

**Verify assumptions.** The bug is often in the gap between what you assume and what is true:

- Is the file you are reading the file being executed? Is the build current?
- Is the environment variable set, and to the value you expect?
- Are you on the right branch, against the right database, on the right port?
- Is the service actually running? Is the dependency reachable?
- Is the module importing what its name suggests? Is the provider registered in the scope you think?
- Is the test running against the code you think it is?

### Step 5 — Find the working analogue

Find something nearby that does the same kind of thing and works — a sibling method, a sibling module, the same feature for a different type, or this file at a commit where it worked. Read it completely, then enumerate every difference and give each a verdict.

Read **`references/pattern-analysis.md`** for where to look, the difference ledger, and the classes of difference that most often matter.

If no analogue exists, say so and move on. Do not manufacture one.

### Step 6 — Rank the hypotheses · GATE 2

Generate 3-5 candidates. Each needs a **mechanism** naming the code that produces the behaviour, and a **prediction** that would come out differently if it were false. Discard anything that cannot be refuted. At least one candidate must challenge an assumption rather than the code.

Rank by prior likelihood and cost to falsify, then show the ranked list to the developer via one `AskUserQuestion` before probing — they frequently re-rank it instantly from knowledge that is nowhere in the code. Proceed on your own ranking if they are unavailable, and say that you did.

Read **`references/hypotheses.md`** for the grammar, the ranking method, probe design, the ledger, and the conditions under which probes may be fanned out in parallel.

### Step 7 — Probe

One hypothesis, one probe, one variable. Prefer observation over mutation; prefer a connected tool over a log line. Revert each probe before the next.

Any added instrumentation carries a session tag (`[DEBUG-<xxxx>]`) and obeys the hard constraints above — read **`references/instrumentation.md`** before the first probe.

Update the hypothesis ledger after every probe, recording refutations as carefully as confirmations. Re-rank what remains.

When three or more live hypotheses are independent and every probe is read-only, you may fan out up to four `hypothesis-prober` agents in parallel at the `excellent` tier (`best` when the mechanism is subtle). Probes that write anything stay serial in this session.

### Step 8 — Articulate and audit the root cause · GATE 3

State the cause in one sentence: the mechanism, the code that produces it, and why the bad state arose.

- Weak: *"The query returns null so I'll add a null check."*
- Strong: *"`findDispatches` joins on `user_id` against `accounts`, so any tenant whose account id differs from its user id resolves zero rows — the null is downstream of a wrong join key."*

Then trace once more: ask "why?" at each layer until the next answer is outside the change's scope. A symptom fix will break again; a cause fix will not.

**Audit the claim.** For any cause that is non-obvious, spans modules, or sits on a regulated surface, spawn `root-cause-auditor` at the `excellent` tier with the claim, the ledger, and the evidence. It tries to refute the cause from a cold context. If it succeeds, return to Step 6 — an unaudited cause on a regulated surface is a guess with good grammar.

If you cannot articulate the cause, return to Step 4. You have not found it.

### Step 9 — Fix, behind a failing test

**The regression test comes first.** If the tight loop from Step 3 is already a committed test asserting this behaviour, it is the regression test. If it is not — a `curl`, a browser drive, a `SELECT` — write one now at the layer the cause lives in. Confirm it fails for the stated cause, then fix. A bug that had no test coverage must not leave the session without it, or nothing stops its return.

Before writing the fix, check the surface. If it touches ePHI, payment data, authentication, authorization, audit-logged operations, or retention, call `mcp__plugin_anthara_anthara-fabric__get_relevant_standards` with that surface area and treat what it returns as hard constraints. Preserve existing access control, audit logging, and encryption posture — a fix must never relax a control to succeed. If the fix changes error handling on a regulated path, confirm no regulated content reaches the message or the log.

Write the **minimal** change that addresses the cause. Per `code-craft`, add no comments — if you are explaining the fix, rename or extract instead; the sole exception is a constraint the code cannot express, such as an upstream bug being worked around. Leave pre-existing comments untouched.

Surface the risk via `AskUserQuestion` before applying when the fix touches auth, payment, or a data migration, and offer to land it on a separate branch for review.

### Step 10 — Verify, clean up, record

In this order:

1. **Remove instrumentation first.** `grep -rn "DEBUG-<xxxx>" .` must return nothing. Verifying instrumented code verifies the wrong code.
2. **Run the regression test** — it must now pass, scoped, per `test-execution`.
3. **Run the related suite** — broader than one file, narrower than the whole suite, covering the modules the fix touches.
4. **Confirm in the real world** — browser behaviour for a UI fix; a `SELECT` for a data fix; the CI-specific condition for a CI-only failure.
5. **Re-run the original loop from Step 3** — it must be green. This is the claim you actually made.
6. **Write the diagnosis record** — see **`references/diagnosis-record.md`** for when it is required and the canonical skeleton. It lands at `docs/diagnoses/<NNN>-<slug>-diagnosis.md`, in the same PR as the fix.
7. **Write the Org Memory entry** if the lesson is durable — non-obvious cause, architectural or external-system quirk, environment-specific trap. Confirm with the developer before writing to shared memory, and link the record's path.

Never declare it fixed on the strength of the code looking right. Report what you ran and what it returned.

## Escalation ladder

Count **distinct fix attempts** — a fix attempt is a change intended to resolve the bug, not a probe. Say the count out loud as it rises.

- **After 1 failed attempt** — the cause was wrong, not the fix. Return to Step 6 with what the failure taught you. Do not adjust the same fix.
- **After 2 failed attempts** — stop changing code. Return to Step 4 and re-read with fresh eyes, then re-enter Step 5 looking for an analogue you skipped.
- **After 3 failed attempts** — **stop and question the design.** Three failures on a correctly-articulated cause usually means the cause is real but the structure makes it unfixable in place: a boundary in the wrong location, state owned by two modules, an invariant nothing enforces. Surface this to the developer via `AskUserQuestion` as an architectural conversation with your evidence, and do not attempt a fourth patch without their direction. If the fix belongs in a design change, that is a spec or plan, not a patch — route to `/anthara:planner`.
- **When analytical approaches are exhausted** and the cause is still unknown, say so explicitly. Recommend permanent compliance-safe observability at the divergence point to capture the next occurrence, or pairing with someone who knows the area.
- **When the same approach has failed twice**, stop repeating it. Retrying is not debugging.

## Scope discipline

Fix the bug; nothing more.

Do not refactor surrounding code, rename unrelated symbols, add features, upgrade dependencies, tidy imports, or fix adjacent defects you noticed on the way. Each of those is separate work — note it and hand it back as a follow-up in the diagnosis record's §9.

A fix PR that also improves three other things cannot be reviewed as a fix, cannot be reverted cleanly if the cause was wrong, and buries the one change that matters.

## References

Read at the step named; each is self-contained.

- `references/reproduction.md` — the tight-loop definition and gate, the repro ladder, minimisation, `git bisect`, flake classification, CI-versus-local divergence, and the cannot-reproduce hand-back. Step 3.
- `references/pattern-analysis.md` — finding and diffing a working analogue, the difference ledger, high-yield difference classes. Step 5.
- `references/hypotheses.md` — hypothesis grammar, ranking, probe design, the ledger, parallel probing conditions. Steps 6-7.
- `references/instrumentation.md` — the `DEBUG-` tag protocol, cleanup proof, what may and may never be logged, condition-based waiting, tool-first debugging. Step 7.
- `references/diagnosis-record.md` — when a record is required, its location, the canonical skeleton, evidence scrubbing, and its relationship to Org Memory. Step 10.

Agents spawned by this skill: `hypothesis-prober` (Step 7, parallel read-only probes), `root-cause-auditor` (Step 8, adversarial audit of the cause).

Skills loaded as context: `test-execution` (scoped runs at Steps 3 and 10), `browser-verification` (UI symptoms at Steps 4 and 10), `code-craft` (the fix in Step 9).
