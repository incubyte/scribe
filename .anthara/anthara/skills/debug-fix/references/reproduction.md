# Reproduction — the tight loop, minimisation, and the regression window

Read this at Step 3 of `/anthara:debug-fix`, before any theorising.

## Contents

- What "tight" means
- The gate
- The repro ladder
- Minimisation
- Finding the regression window with bisect
- Intermittent and flaky failures
- CI-versus-local divergence
- When you cannot reproduce

## What "tight" means

A loop is not tight merely because it exists. Three properties, all required:

- **Fast** — seconds, not minutes. A two-minute loop means four probes an hour and a strong pull toward guessing.
- **Deterministic** — same input, same result. A loop that is red 7 times in 10 cannot falsify anything.
- **Sharp** — red on *this* bug and green when *this* bug is fixed. A loop that is red for three unrelated reasons tells you nothing when it turns green.

Write the loop down as **one named command**. Not a paragraph of steps, not "then click through the UI" — one command a person could paste. If it genuinely takes three commands, wrap them into one.

## The gate

You may not form a hypothesis, read for a root cause, or edit a line of product code until:

1. One named command exists.
2. You have run it at least once.
3. It went red on the reported symptom.

State the command and its red output before moving on. Skipping this gate is not debugging — it is pattern-matching against an error string.

## The repro ladder

Climb from the top. Stop at the first rung that yields a tight loop — do not build a browser harness when a unit test would do.

1. **An existing failing test** — the ideal. Run it scoped: `pnpm vitest run <file>`, `pytest <file> -q`, or the project's equivalent. Never the whole suite (see `test-execution`).
2. **An existing test, narrowed** — the same file with a name filter so the loop covers only the failing behaviour.
3. **A new unit test at the failing layer** — the smallest test that asserts the reported behaviour. This becomes the regression test in Step 9, so write it as one from the start.
4. **A new integration test** — when the bug lives in the wiring (module imports, DI, guard registration, migration order) rather than inside a function.
5. **A one-line HTTP probe** — `curl -sS -i <url>` against the running service. Assert on status *and* body; a 200 carrying an error envelope is still the bug.
6. **A scripted client call** — a short script exercising the SDK or queue producer when the failure needs an authenticated or specifically shaped payload.
7. **A database observation** — a `SELECT` that returns the wrong row or count. Read-only, always.
8. **A browser drive** — for UI symptoms, via a browser MCP. Load `browser-verification`, and treat a console exception as a red loop even when the page renders correctly.
9. **A CI re-run** — last resort for CI-only failures. Slow, so pair it with the divergence checklist below rather than looping on it.
10. **A human-in-the-loop trigger** — you print the exact steps and the developer performs them. Legitimate, but the slowest loop there is; use it to *characterise* the bug while you build a faster rung.

## Minimisation

A repro you have not shrunk is a repro you do not understand. Once red, cut until it is red for exactly one reason:

- **Shrink the input.** Delete fields, rows, and array entries while it stays red. The last surviving field is usually the subject.
- **Shrink the path.** Call the failing unit directly instead of through the controller. If it still fails, everything upstream is exonerated.
- **Shrink the setup.** Remove fixtures, seeds, and mocks one at a time. A repro that needs a 40-line fixture is hiding its cause inside that fixture.
- **Shrink the assertion.** Assert the single value that is wrong, not the whole response shape.

Stop when removing anything more makes it green. Record the minimised repro — it is the most valuable artifact of the session, and it goes into the diagnosis record verbatim.

## Finding the regression window with bisect

When the symptom is a regression (*"this worked yesterday"*, *"broke after the deploy"*, *"started after PR #N"*), do not read commits one at a time. Binary-search them:

```bash
git bisect start <known-bad-ref> <known-good-ref>
git bisect run <the named command from the gate>
git bisect reset
```

`git bisect run` needs the command to exit non-zero on failure — most test runners already do. This is why the gate demands one command: it turns the regression window into a mechanical search rather than a reading exercise.

- Pick the known-good ref from evidence (the last green CI run, the tag that shipped), never from memory.
- If the build itself changes across the window, wrap the command so a build failure exits with `125` — bisect then skips that commit instead of scoring it.
- Once bisect names the commit, name the **pull request and its stated intent**. A commit that broke something it never meant to touch is a strong signal about the real coupling.
- Run `git bisect reset` always, even when interrupted. Leaving the repo detached is its own bug report.

Bisect names *where* the behaviour changed. It does not name the root cause — the introducing commit is evidence for Step 8, not a verdict.

## Intermittent and flaky failures

Run the loop **five times** before calling anything flaky. Then classify it, because each class has a different investigation:

| Pattern | Likely class | Where to look |
|---|---|---|
| Fails alone, passes in the suite (or the reverse) | Test-order dependency | Shared module state, singletons, a DI container reused across tests |
| Fails under load or parallelism | Race condition | Unawaited promises, concurrent writes to one row, missing transaction boundary |
| Fails at particular times or dates | Uncontrolled clock | Real time read inside the code under test, timezone-naive comparisons, month and DST boundaries |
| Fails on slower machines or in CI | Timing assumption | Fixed sleeps, arbitrary timeouts, an assertion that ran before the effect landed |
| Fails on the first run after a reset | Ordering or seeding | Migration order, seed-versus-build conflict, a cache warmed by a previous run |
| Fails at a fixed rate independent of anything | External dependency | Retried HTTP, a mock interceptor consumed once, connection-pool exhaustion |

A flaky loop is not a tight loop. Make it deterministic **first** — pin the clock, force serial execution, stub the network — then debug the real bug. Do not investigate through the noise.

**Never widen a timeout to make a flake go away.** That converts a race condition into a slower race condition. Replace the wait with a condition poll — see `instrumentation.md`.

## CI-versus-local divergence

When it fails in CI and passes locally, the bug is usually in the difference, not in the code. Work this list before re-running the pipeline:

- **Environment variables** — CI has no `.env`. Which value is absent or silently defaulted?
- **Secrets** — present locally, unset or expired in the runner.
- **Services** — is the database, Redis, or queue actually up in CI, and on which host and port?
- **Ordering and parallelism** — CI often shards, or runs in a different order than a local watch run.
- **Clock and locale** — runners are UTC; the developer machine probably is not.
- **Filesystem** — case sensitivity (Linux runner, case-insensitive developer machine) and path separators.
- **Install state** — a lockfile-exact install in CI versus drifted local dependencies; a stale build artifact locally.
- **Migrations and seeds** — CI starts empty and migrates; local carries months of accumulated state.
- **Network** — CI may have no outbound access, so an unmocked call fails loudly rather than succeeding quietly.

Read the full CI log and identify **which stage** failed — install, build, lint, typecheck, test, deploy. A "test failure" that is really a build failure sends the whole investigation the wrong way.

## When you cannot reproduce

Say so explicitly, and stop. Do not fix speculatively: an unreproducible bug cannot be verified fixed, so any change made is unfalsifiable.

Then do the one thing that is still useful — make the *next* occurrence diagnosable.

1. List the rungs attempted and what each produced.
2. Name precisely what would be needed — an environment, a request ID, a failing payload, a log window, access to the tenant.
3. Add **permanent, compliance-safe observability** at the divergence point: a structured log or metric carrying correlation IDs, not a temporary probe, and never regulated content. Permanent observability is a product change, so keep it minimal and mention it explicitly in the hand-back.
4. Hand back with that list, and ask via `AskUserQuestion` for the specific artifact that would unblock the loop.
