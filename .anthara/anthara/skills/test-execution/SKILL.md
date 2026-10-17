---
name: test-execution
description: Tech-agnostic guidance for running tests during a plan-implementer run with scope discipline. Three checkpoints per spec — slice-start regression check (slice-builder, scoped to prior slices' test files), slice-end verification (slice-builder, scoped to the current slice's test files, after its code and tests are written), full suite once at end-of-spec (orchestrator) as the final gate. No watch mode, no per-AC polling, no per-chunk re-invocations. Loaded by slice-builder in the slice loop, and by the debug-fix skill while iterating on a fix. Loaded as context by other agents — not user-invokable.
---

# test-execution

How agents *run* tests during the plan-implementer loop. Companion to `test-craft` (what to write). One sentence: scope each test invocation to the test files in play, and let the slice loop's structure determine when to run — three natural checkpoints, never anywhere else.

## Principle

Cold-start cost dominates a plan-implementer run's wall time when tests run frequently. The cheapest test invocation is the one not made; the next cheapest is one scoped to a small file set. Per-AC polling and continuous watch mode add complexity (stale-output detection, watcher lifecycle, scope management) for marginal benefit in this loop. Three structured checkpoints deliver the same cold-start economy without the infrastructure.

## Three checkpoints

### 1. Slice-start regression check (slice-builder, slice N ≥ 2)

Before writing slice N's code, run the test files modified in slices 1 through N-1 as a scoped one-shot.

- Skip for the very first slice — no prior tests to check.
- Scope to the test files prior slices touched. Determine via `git diff --name-only --diff-filter=AM <start-of-plan-implementer>..HEAD '*.test.*' '*.spec.*'` or via the orchestrator-tracked file list across slice handoffs.
- If failures appear, hand off to the orchestrator with the failure list. The orchestrator surfaces the regression *before* slice N gets built on top of broken state. Do not build new work over a broken prior suite.
- If green, proceed to write slice N's code.

### 2. Slice-end verification (slice-builder, every slice)

After writing all the slice's code in one batch and then its tests in one batch, run the slice's test files as a scoped one-shot.

- Run once. **Green:** batch-flip all the slice's ACs `[ ]` → `[x]` in spec + ticket; hand off.
- **Red:** read the output, identify which ACs failed by mapping the failing behavior-named tests back through the plan's test strategy (which pins which ACs each test covers, per `test-craft`), fix only the corresponding code, run again. Repeat within retry budget (default 5 per slice).
- Each fix iteration is one invocation. Tight retry budget — failing fast surfaces a diagnostic to the orchestrator faster than burning time on a stuck slice.

### 3. End-of-spec full-suite gate (orchestrator, once)

After the last slice's slice-builder hands off, the orchestrator runs the full test suite as one cold one-shot.

- Catches in-memory state pollution, hidden cross-slice damage, test-order dependence — defects that scoped runs cannot catch by definition.
- One invocation per plan-implementer run. Cheap insurance.

## Scope discipline

Every invocation runs a scoped file set — never the whole suite — except the end-of-spec gate.

### Per-stack scoped invocation hints

| Stack            | Scoped invocation example                                    |
|------------------|--------------------------------------------------------------|
| TS/JS (Vitest)   | `pnpm vitest run src/dispatch/dispatch.service.spec.ts`      |
| TS/JS (Jest)     | `pnpm jest src/dispatch/dispatch.service.spec.ts`            |
| Python (pytest)  | `pytest tests/dispatch/test_dispatch.py -q`                  |
| Go               | `go test ./internal/dispatch/...`                            |
| Rust             | `cargo test --test dispatch`                                 |
| JVM (Gradle)     | `./gradlew test --tests "com.example.dispatch.*"`            |
| Ruby (RSpec)     | `bundle exec rspec spec/dispatch/dispatch_spec.rb`           |
| .NET             | `dotnet test --filter "FullyQualifiedName~Dispatch"`         |

The actual command comes from:

1. The project's package / build manifest scripts (`package.json` `test` script, Gradle tasks, etc.).
2. The `docs/specs/<NNN>-<slug>-context.md` Test Conventions section captured by `code-orienteer`.
3. The framework's documented per-file invocation as the fallback.

## Invocation accounting

For a plan-implementer run with N slices, M ACs each, no retries:

- Slice-start regression checks (slice-builder): **N − 1** invocations (slice 1 skips)
- Slice-end verifications (slice-builder): **N** invocations (one per slice)
- End-of-spec full suite: **1** invocation
- Total: **2N** invocations

A 3-slice run pays 6 invocations minimum, 9–12 with typical retries (1–2 per slice). A 10-slice run pays ~20–30. Linear with slice count, not multiplicative with AC count — and scoped, so each invocation is cheap.

## What not to do

- **Don't run the whole suite per slice.** It pays cold-start cost for code far outside the slice's scope.
- **Don't pipe through `tail -n N`.** Stdout truncation hides earlier failures; read the full output and find the pass / fail summary line.
- **Don't poll per-AC.** slice-builder batch-writes code for all the slice's ACs; one invocation at slice end verifies them together. Test names describe behavior (per `test-craft`) and the plan's test strategy pins which ACs each test covers, so a failing test maps back to its AC on a re-run.
- **Don't start a watcher.** Continuous watch mode adds watcher-lifecycle complexity for marginal benefit in this loop.
- **Don't run tests in the reviewer.** The reviewer reads diffs and patterns; it does not execute tests.
- **Don't run a full-suite gate per slice.** Scoped slice-end is slice-builder's job; full-suite belongs only at end-of-spec.

## How agents use this skill

**slice-builder** (per slice):

1. If N ≥ 2: run prior slices' test files (scoped one-shot regression check). If failures, surface to orchestrator with failure list — do not build new work. If green, proceed.
2. Write all the slice's code for all ACs in one batch, designed so the tests in step 3 are easy to write — no per-AC iteration, no mid-batch test polling.
3. Write the slice's tests for all ACs in one batch, each assertion derived from its AC's text. Test names describe behavior; the plan's test strategy maps each test to its ACs, so slice-end output still traces back to ACs.
4. Run the slice's test files (scoped one-shot).
5. Green: batch-flip all the slice's ACs `[ ]` → `[x]` in spec + ticket. Hand off.
6. Red: read output → map failing tests to ACs via the plan's test strategy → fix → run again. Repeat within retry budget (default 5). Surface a diagnostic if budget exhausts.

**debug-fix** (tight loop + verify):

1. Establish the loop: **one named command**, scoped to the specific failing test file, run once and red. This is `debug-fix` gate 1 — nothing proceeds without it. Because it is one command with a non-zero exit, it doubles as the `git bisect run` predicate for a regression window.
2. Run it five times before calling anything flaky. A non-deterministic loop is not a loop; make it deterministic first.
3. Probe with it, one change at a time, reverting each probe before the next. Never run the full suite to check a probe.
4. Regression test: if the loop is not already a committed test asserting the buggy behaviour, write one at the layer the cause lives in and confirm it fails **before** the fix.
5. Remove all instrumentation, then verify — instrumented code is not the code you are verifying.
6. Verify: the regression test, then the related test suite as a scoped one-shot covering the modules the fix touches (broader than the single test, narrower than full suite), then the original loop command green.
