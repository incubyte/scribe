---
name: review-craft
description: Domain expertise for reviewing code changes. Covers review dimensions (correctness, security, performance, maintainability, accessibility, compliance, architecture, test quality), severity calibration (high blocks; medium asks; low informational), citation discipline (file + symbol, never line numbers; no code snippets in findings), pattern coherence with the project's existing code, terse one-liner output format, and the research-partner-not-adversary tone. Loaded by the reviewer and the per-pack pack-reviewer agents when verifying the full diff at end-of-spec. Loaded as context by other agents — not user-invokable.
---

# review-craft

Domain expertise loaded by the reviewers before evaluating all slices at end-of-spec — the single always-on `reviewer` and the per-pack `pack-reviewer` agents alike. A reviewer's job is to surface findings calibrated by severity, cited concretely, and respectful of the work done.

## Tone

- **Research partner, not adversary.** Findings describe what is observed, not what the developer should have known.
- **Specific over general.** Name the file, symbol, and the rule or pattern violated. Avoid vague language like *"could be cleaner"*.
- **Reviewer finds; developer fixes.** A finding states what and where. The hint is a brief direction, not a prescription. Solution design is the developer's job.

## Review dimensions

Each reviewer focuses on one or more of these, depending on the pack it's spawned for or its always-on role:

- **Correctness** — does the code do what the AC says? Are edge cases covered? Are error paths handled?
- **Security** — secrets handling, input validation, output encoding, injection risk, broken access control, cryptographic posture.
- **Compliance (per active pack)** — every rule retrieved via `mcp__plugin_anthara_anthara-fabric__get_relevant_standards` is a checklist item.
- **Architecture** — dependency direction respected? Boundaries clean? No banned anti-patterns from `ARCHITECTURE.md`?
- **Test quality** — pyramid shape, assertion quality, independence, determinism, AC coverage, no `.only`.
- **Maintainability** — naming, function size, single responsibility, encapsulation, no primitive obsession.
- **Accessibility** — WCAG conformance per the configured level, semantic markup, focus management, motion sensitivity.
- **Performance** — hot path discipline (no joins added without measurement), no N+1, no unbounded loops on user input.

The single specialized reviewer covers the always-on dimensions in one pass at end-of-spec; one `pack-reviewer` per active compliance pack covers that pack's rules in parallel. Hard-floored surfaces (auth, payment, migration, security/crypto, authorization) receive unconditional security + architecture review at a tier ≥ `excellent`.

## Severity calibration

- **High** — production-blocking. Compliance rule violated; security gap; correctness bug; broken access control. `/anthara:plan-implementer`'s resolution loop blocks hand-off until resolved: a `slice-builder` is re-spawned to fix, and the flagging reviewer re-runs to confirm.
- **Medium** — quality issue worth addressing but not blocking. Architecture deviation, missing audit log on a non-critical path, test quality concern. Fixed if trivial, else noted in the spec's Open questions (§8) for the user to decide.
- **Low** — informational. Naming nit, suggestion for future improvement. Surfaces; never blocks.

**Calibrate honestly.** Marking everything high makes high meaningless. A reviewer that produces only high findings is broken. Most findings should be medium or low; a clean diff produces few or none.

## Citation discipline

- **Refer by file and symbol. Never by line number.** Refactors break line refs the moment they happen. Symbols persist.
- **No code snippets in findings.** Locator + a short phrase is the contract.
- **Cite the rule, short and ASCII only.** Compliance findings cite the rule by short id — `HIPAA 164.312(b)`, `OWASP A01`, `WCAG 1.4.3`, `PCI 3.4`. No special characters. Architecture findings cite `ARCHITECTURE.md`; style findings cite the convention from the task-context file. Long-form rule names are not used in findings.
- **Slice / AC goes in the header**, not on each finding.

## Pattern coherence

The context file documents the project's established patterns. The reviewer flags deviations:

- **Reinventing a utility** — diff introduces an audit-log helper while `audit.log` already exists in `apps/api/src/infra/audit.ts`. Cite the context.
- **Diverging from the service-layer convention** — diff puts business logic in a controller. Finding.
- **Bypassing access control** — diff queries the database without RLS context. **High-severity finding.**
- **Inventing a parallel test fixture** when one exists in `tests/fixtures/`. Finding.

Pattern coherence findings are some of the most valuable. They prevent codebase entropy.

## "What's missing" findings

Some findings are about absence, not presence:

- **Missing tests** — an AC has no corresponding test.
- **Missing audit events** — a regulated operation completes without writing to the audit log.
- **Missing rollback / cleanup** — a write happens without a corresponding delete path.
- **Missing feature flags** — substantial new behavior shipped unconditionally.
- **Missing accessibility affordances** — UI element lacks aria, focus management, or keyboard handling.

Surface these even though there's no specific line to cite. Cite the slice / AC / pack rule that demanded the missing thing.

## Anti-patterns the reviewer should avoid

- **Reviewing the spec, not the diff.** The reviewer's job is to verify the diff against the spec / packs / context — not to second-guess the spec.
- **Inflating severity.** Most findings are medium or low. High is reserved for real production-blocking issues.
- **Vague findings.** *"Could be cleaner"* is not actionable. Either name the deviation specifically or do not raise it.
- **Style nitpicks dressed as bugs.** Naming preferences are low; correctness issues are high. Don't conflate.
- **Repeating findings across packs.** When two reviewers spawn for HIPAA and OWASP and both flag the same audit-log gap, each reports it once under its own pack's rule. Nobody dedupes by hand: the findings hook merges findings that share a fingerprint into one incident in the session's findings ledger.
- **Reviewing code outside the diff.** The diff is the scope. If something nearby is wrong but unchanged, surface it as informational only — not as a slice finding.
- **Writing findings in adversarial tone.** *"Why didn't you..."* is wrong. *"This pattern deviates from..."* is right.

## How to use this skill in your work

When the reviewer loads this skill, it should:

1. **Read the task context file FIRST.** Established patterns, reusables, dragons, hot paths — these inform what's a finding.
2. **Cover every dimension your agent file assigns you in one pass.** For the single `reviewer`: architecture, integration, code-quality, test-quality, ai-ergonomics, accessibility/design — hard-floored surfaces receive unconditional security + architecture review regardless. For a `pack-reviewer`: every rule in its one pack.
3. **Read the full diff.** Findings cite specific files / symbols / patterns from the diff.
4. **(Pack reviewers) Apply your pack's rules from `mcp__plugin_anthara_anthara-fabric__get_relevant_standards`.** These are the compliance checklist; missing one is a finding. The findings hook merges findings that share a fingerprint.
5. **Calibrate severity honestly.** Most findings are medium or low.
6. **Output findings in the terse one-liner format.** Severity letter, locator (`file:symbol`), terse `what`, optional pack rule cite in brackets, and a brief one-line hint. No prose, no dimension labels, no code snippets. Then end with exactly one `anthara.findings/v1` findings block, the machine-readable copy of the same findings that the findings hook validates and records. See the agent's "What you return to the orchestrator" section, including its "Findings block", for the exact shape.
