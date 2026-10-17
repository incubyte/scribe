---
name: test-craft
description: Domain expertise for writing excellent tests. Covers pyramid bias, test independence, determinism, meaningful assertions, behavior-as-documentation, compliance assertions on regulated surfaces, and the reuse-over-reinvent rule (extend existing tests when possible — do not write fresh tests when the established suite covers the behavior). Loaded by slice-builder when authoring a slice's tests after its code, and by the reviewer when verifying test-suite dimensions. Loaded as context by other agents — not user-invokable.
---

# test-craft

Domain expertise loaded by slice-builder when writing a slice's tests for its ACs, and by the reviewer when verifying the test suite. Tests are the documentation of behavior; this skill says what good documentation looks like.

## Principles

- **Pyramid bias.** Unit > integration > e2e. Default to unit; justify any non-unit choice in a one-line comment near the test or in its description.
- **One AC, at least one test; usually one test per AC.** Tests map 1:1 to ACs in the spec. The AC's `[ ]` checkbox flips when the test goes green.
- **Tests are independent.** Any test runnable alone, any order, any subset. No setup that mutates global state without a teardown.
- **Tests are deterministic.** No real time (`Date.now()` is mocked or wrapped); no real randomness; no real network; no flaky retries.
- **Meaningful assertions.** *"didn't throw"* is not an assertion. The assertion describes the observable behavior the AC promises.
- **Test names describe behavior — nothing else.** Read the test list, you should understand what the slice does. *"signs in via Google OAuth and lands on /meetings"* not *"test sign in"* — and never an AC or slice ID (*"AC 5.2"*, *"5.1.1: …"*). IDs rot when the spec renumbers and say nothing about behavior; the AC↔test mapping lives in the plan's test strategy, not the name.
- **Arrange / Act / Assert.** Three clear phases. Most tests have one act; tests with multiple acts are usually two tests.
- **Behavior-driven boundaries.** The system under test is a *behavior*, not a class. A unit test can exercise multiple classes when they collaborate to deliver one behavior.
- **Edge and adversarial cases are first-class.** Every slice carries at least one error-path AC and a test for it. Adversarial paths (malformed input, concurrent access, denial conditions) are not afterthoughts.
- **Coverage is the floor, not the ceiling.** High coverage with weak assertions is worse than moderate coverage with strong ones.
- **No logic in tests.** No `if` / `for` / `while` / expressions in test bodies. The test reads top-to-bottom — arrange, act, assert. Logic in a test means you're testing the logic itself, not the code under test.
- **DAMP over DRY in test code.** Descriptive And Meaningful Phrases beat Don't Repeat Yourself when test readability is at stake. Inline a literal or a short setup when it makes the test read linearly; extract helpers only when duplication outweighs the indirection cost.
- **Test sizes are categorical, not vibes.** Small (no network / disk / external process), Medium (localhost services or containers), Large (real systems). The size constrains parallelism, stability, and CI tier. Default Small; justify upward.
- **State over interaction verification.** Assert what's true *after* the action, not which methods were called. On mocks, verify only state-changing calls (writes / publishes / dispatches); reads / queries do not need verification. Use interaction-based verification only when the interaction IS the user-observable behavior (publishing to a queue, firing an event).
- **Fakes preferred over mocks for collaborators you own.** A fake is a working in-memory implementation; a mock is a stub returning canned responses. Fakes survive refactors; mocks couple tests to call patterns. Reserve mocks for collaborators you do not own (third-party SDKs) and for behaviors that ARE interactions.
- **Test behavior, not structure.** Tests guard against regressions in what the system *does*, not what the codebase *looks like*. Each assertion derives from the AC's text, never from reading the implementation it covers — a test reverse-engineered from the code passes whatever that code does. File-existence assertions, AST import scans, config-key greps, and README content checks are one-time migration checklists, not regression guards.

## Compliance assertions

When the slice touches regulated content (PHI / payment / consent / accessibility), the test suite includes assertions for the active pack's rules. Concrete examples:

- **HIPAA access control** — three-user RLS fixture: User A writes a transcript; assert User B cannot read; assert User C with a share grant CAN read.
- **HIPAA audit logging** — assert the `audit_log` table has the expected entry with `(timestamp, principal, action, target_id)` after the action.
- **HIPAA error leakage** — when an error fires on a PHI-bearing endpoint, assert the response and log carry no PHI.
- **OWASP A01 broken access control** — request with no JWT returns 401; request with another user's JWT returns 404 (not 403; prevents enumeration).
- **WCAG accessibility** — axe-core runs against rendered UI in the test; assert zero violations at the configured level.
- **PCI cryptography** — sensitive fields absent from logs, headers, responses; tokens unpredictable.

The pack rules retrieved via `mcp__plugin_anthara_anthara-fabric__get_relevant_standards` for the slice are the source of truth for which assertions to write.

## Reuse over reinvention

**Read the context file's Test Conventions section before writing any test.** If existing tests cover the AC's behavior, extend them with the new assertion rather than writing a parallel test. If existing fixtures (database setup, user factories, RLS-fixture, etc.) match the slice's needs, use them.

When existing tests are close but not exact:

- **Extend, do not duplicate.** Add the new assertion as a separate `it` / `test` within the existing `describe` block.
- **Refactor existing tests minimally** if needed — but only as much as the slice's ACs require. Reviewer agents flag large-scale test refactor as out-of-slice change.

When in doubt about whether to extend or write fresh, surface the choice in the slice handoff summary. Defaulting to fresh is a defect; default to extend.

## Anti-patterns to avoid

- **Assertion-free tests** — calling code without `expect`. Looks like a test; isn't one.
- **`.only` in committed tests** — slips through review. CI must fail on `.only`.
- **Time-dependent tests** — `expect(getDate()).toBe(today)`. Mock the clock.
- **Real-network tests** — slow, flaky, breaks CI. Use fixtures or fakes.
- **Snapshot-only tests for behavior** — a snapshot tells you when output changed, not whether the change was correct. Snapshots fit stable formats (UI render, serialization), not logic.
- **Tests that depend on order** — passes alone but fails with siblings. Hidden state.
- **Mocking what you don't own.** Mock the adapter (your code); don't mock the SDK underneath.
- **Tautological tests** — `expect(add(2, 2)).toBe(2 + 2)`. Verifies behavior, not the implementation restated.
- **Test files duplicating production logic.** When a test reproduces what the code does, the assertion has lost meaning.
- **Change-detector tests** — a test that fails on every code change without catching a real bug. If you cannot articulate "this catches bug X," delete it.
- **Hairy `setUp` / `beforeEach`** — page-long setup means the test is exercising too many behaviors at once. Split the test or extract a test-data builder.
- **Function- or class-named `describe` blocks.** Use a meaningful description of behavior, not the function or class name. `describe('computeDedupKey', ...)` and `describe('UserService', ...)` rot when the symbol gets renamed or absorbed; `describe('email deduplication is case-insensitive')` and `describe('user creation rejects malformed emails')` describe what the system does.
- **Absence-verification tests** — asserting a file was deleted, a symbol removed, or a config key is gone. These verify a migration happened, not that the system works. Skip them; code review covers this.

## How to use this skill in your work

When slice-builder loads this skill, it should:

1. **Read the task context file's Test Conventions and Reusables sections FIRST** (from `docs/specs/<NNN>-<slug>-context.md`). What test framework, what fixtures, what conventions are already in play.
2. **For each AC in the slice, decide test level (unit / integration / e2e).** Default unit; justify upward in a one-line comment when the AC genuinely needs higher (e.g., RLS verification needs a real DB).
3. **Check for existing tests in the area BEFORE authoring fresh.** Extend, don't duplicate.
4. **Map tests 1:1 to ACs.** Each test's name describes the behavior the AC promises (`unauthenticated visit redirects to /sign-in`) — no AC or slice IDs in the name. The plan's test strategy pins which ACs each test covers; that mapping, not the test name, is what ties a test back to its AC.
5. **Compliance ACs get compliance assertions** drawn from the pack rules in `mcp__plugin_anthara_anthara-fabric__get_relevant_standards`. Do not skip.
6. **Surface decisions in the handoff summary** — which tests were extended vs. fresh, what level was chosen and why.
