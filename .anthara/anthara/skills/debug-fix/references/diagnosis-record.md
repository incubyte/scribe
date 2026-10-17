# The diagnosis record — the audit artifact of a debugging session

Read this at Step 10 of `/anthara:debug-fix`, when the fix is verified.

## Contents

- What it is and why it exists
- When it is required
- Where it lives
- The canonical skeleton
- Scrubbing the evidence
- Relationship to Org Memory

## What it is and why it exists

A committed markdown file recording how one defect was diagnosed: the symptom, the reproduction, the hypotheses considered and eliminated, the root cause, the fix, its compliance impact, and the evidence that it works.

It serves three readers, in this order:

- **The next engineer** who meets the same symptom. The refuted hypotheses save them the hours you already spent.
- **The reviewer** of the fix PR, who needs to know why *this* change is the right one rather than the first one that made the tests pass.
- **The auditor**, for whom "we investigated and remediated" is an assertion and this file is the evidence. HIPAA, SOC 2, PCI, and FDA SaMD regimes all ask for records of defect investigation and remediation on controlled surfaces. A green test run is not that record.

A conversation transcript is not a substitute: it is not committed, not reviewable, and not linked to the change.

## When it is required

**Required** when any of these hold:

- The fix touches a regulated surface — ePHI, payment data, authentication, authorization, audit logging, encryption, or data retention.
- The defect reached a shared environment (staging, production, or a customer tenant).
- Diagnosis took more than one probe cycle, or more than one hypothesis had to be eliminated.
- The root cause lies outside the file that showed the symptom.
- A fix attempt failed at least once before landing.
- The escalation ladder was reached — three failed attempts, or an architectural stop.

**Optional** when the defect was a typo, a local logic slip, or otherwise obvious from a single read, the fix was one line, and nothing regulated was touched. In that case skip the file, and consider only the Org Memory write.

When in doubt, write it. It costs a few minutes and it is the artifact nobody can reconstruct later.

## Where it lives

`docs/diagnoses/<NNN>-<slug>-diagnosis.md`

- Create `docs/diagnoses/` if absent, and make sure it is not git-ignored.
- `<NNN>` auto-increments across the files already in `docs/diagnoses/` — it is independent of the spec and plan numbering in `docs/specs/` and `docs/plans/`.
- `<slug>` is a short kebab-case description of the **symptom**, not the fix: `dispatch-returns-zero-rows`, not `fix-join-key`. The next engineer searches by what they are seeing.
- Committed with the fix, in the same pull request, so the reasoning and the change are reviewed together.

## The canonical skeleton

Use this structure. Sections are fixed; omit a section only when it genuinely does not apply, and say why rather than deleting the heading.

```markdown
> **Diagnosis** <NNN> · **Symptom** <one line> · **Status** resolved | mitigated | unreproducible
> **Surfaces** <modules touched> · **Packs** <active compliance packs, or none>
> **Fix** <PR link or commit sha> · **Debug tag** DEBUG-<xxxx> (cleanup verified)

# <NNN> — <symptom as a short title>

## 1. Symptom

What was reported, in the reporter's terms. The exact error text, status code, or observed
behaviour. Where it was seen (environment, tenant, CI run) and when it started.

## 2. Reproduction

The one named command, verbatim, and its red output. Then the minimised repro: the smallest
input, path, and setup that still fails. State which rung of the ladder it came from.

If the regression window was found by bisect, name the introducing commit and its pull
request here, with the stated intent of that change.

## 3. What was read

The code path traced, from entry point to failure. The working analogue diffed against, if
one existed, and the difference ledger's surviving entries. The assumptions checked, and
any that turned out to be false.

## 4. Hypotheses

| # | Mechanism | Prediction | Probe | Result |
|---|---|---|---|---|
| H1 | … | … | … | refuted — … |
| H2 | … | … | … | **confirmed** |

Record refutations. They are the part of this document with the longest useful life.

## 5. Root cause

One paragraph, mechanism-first, naming the code that does it. Why the bad state arose —
not where it surfaced. If the symptom and the cause are in different modules, say so
explicitly; that gap is the reusable lesson.

## 6. The fix

What changed and why that addresses the cause rather than the symptom. What was
deliberately left alone, and why. Anything considered and rejected.

## 7. Compliance impact

Standards consulted (via `get_relevant_standards`) and the surface they applied to.
Confirm explicitly: access control unchanged, audit logging preserved or extended,
encryption posture unchanged, no regulated content added to logs or error messages.

State "no regulated surface touched" when that is the case — an explicit negative is
evidence; silence is not.

## 8. Verification

The regression test added: its file, what it asserts, and confirmation that it failed
before the fix and passes after. The scoped suite run and its result. Browser or data
confirmation where applicable. The zero-hit instrumentation cleanup grep.

## 9. Prevention and follow-ups

Whether this class of defect can be prevented structurally — a type, a guard, a lint rule,
a fixture, a CI check, a test at a different layer. Follow-up items with owners, or an
explicit "none identified".
```

## Scrubbing the evidence

Everything pasted into this file is committed, pushed, and PR-visible. It is subject to the same rules as a debug log — see `instrumentation.md`.

- Never paste a raw request or response body, a full query string, SQL with bound parameter values, a token, or a stack trace whose message may embed a payload.
- Replace regulated values with their shape: `patientId=<uuid>`, `bodyKeys=[id,status]`, `rows=0`.
- Redact before pasting, not after. A commit that redacts a secret does not remove it from history.
- Reference tenants, customers, and users by identifier or role, never by name.

## Relationship to Org Memory

The record and the memory write are different instruments and are not interchangeable:

- **The diagnosis record** is the *evidence* — complete, committed, immutable, tied to the PR. Written for the reviewer and the auditor.
- **The Org Memory entry** is the *index* — one durable, atomic, searchable lesson that surfaces at Step 2 of a future session, before anyone knows this file exists. Written for retrieval.

Write the memory entry only when the lesson is durable: a non-obvious root cause, an architectural or external-system quirk, an environment-specific trap, a pattern likely to recur. Skip it for a one-off slip.

When both apply, the memory entry states the gotcha and its fix pattern in a few lines and **links to the record's path** so the reader can reach the full reasoning. Confirm with the developer before writing to shared memory. A memory entry with no record behind it is a claim; a record nobody can find is a filing cabinet.
