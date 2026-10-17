# Canonical Anthara plan skeleton

Use verbatim. Section order is fixed. Read at write time (Steps 4–6 of `SKILL.md`).

The plan serves two audiences, so it has two parts. **Part 1 — Review this** is written for a human deciding whether to approve or redirect the approach: plain language, higher-level, no schema jargon — the reviewer understands the overall approach before (or instead of) reading implementation detail. **Part 2 — Build detail** is the prescriptive contract the build agent executes. Nothing load-bearing may live only in Part 1 — the executor reads Part 2. Every `§`, slice, and decision citation is a markdown link per the §-links operating principle.

Confidence markers, used throughout Part 2: `[verified]` = the planner read the code or docs and confirmed it; `[inferred]` = pattern-matched from a sibling implementation, not confirmed — the build agent checks before assuming. Never write an unverified external fact in confident voice.

````markdown
# Plan: <NNN>-<slug>

> Spec: `<path>` — or `none — planned from <ticket id | prompt>` (standard tier) · Context: `<path>` · Authored: <YYYY-MM-DD>
> Risk: <LOW|MODERATE|HIGH> (<one-line reason>) · Hard floors: <none | slices N, M> · Fabric standards: <reachable | unreachable — note the degradation>

---

# Part 1 — Review this

*Everything a reviewer needs to approve or redirect the approach. If you only read one part, read this one.*

## What we're doing

<2-4 short paragraphs, plain language: the user-visible goal; the core approach and WHY it must be that way — state the constraint that forces it, not just the choice; any deliberate exception to the approach, named as deliberate.>

## How it breaks down

<1-2 sentences on the slicing strategy — what slice 1 establishes, how later slices build on it, where the order comes from.>

| # | Slice | Delivers | Why this tier | Tier |
|---|-------|----------|---------------|------|
| 1 | <name> | <one line> | <one line — leverage, difficulty, or consequence> | `<best|excellent|fast>` |

<Cross-slice notes a reviewer should know — e.g. "slices 2 and 3 are near-identical; 3 reuses 2's shape rather than inventing a second one.">

## Decisions made

**1. <The decision as a declarative sentence>.** <2-4 sentences of plain-language reasoning: the constraint, the consequence, which slices it lands in.> *Rejected: <the alternative, and what would have gone wrong>.*

**2. <...>**

<These numbered decisions are the plan's argument. Per-slice Key decision lines in Part 2 cite them by number instead of re-explaining.>

## Where this could go wrong

- **<The riskiest thing in the plan>** — <why it's risky, and which Part 2 quality check guards it>.
- <2-5 items total, honest, concrete: risky queries, divergence risks, spec §9 items adjacent to a slice that must not be fixed in passing, caught-before-build corrections (link the escalation log).>

## What's out of scope

<One short paragraph or list, drawn from the spec's §1 *Deliberately out of scope* and §9 *Deferred follow-ups and accepted limits*, each cited as a link. What a reviewer might assume is included but isn't — and which deferred items a builder could plausibly touch by accident.>

---

# Part 2 — Build detail

*Prescriptive detail for the build agent. Reviewers can skim or skip.*

**Confidence markers used below:** `[verified]` = the planner read the code or docs and confirmed it. `[inferred]` = pattern-matched from a sibling implementation, not confirmed — the build agent should check before assuming.

## Shared conventions (all slices)

- **Architecture:** <the named style and dependency direction the whole plan obeys; what this work leaves unchanged>.
- **Dependency direction:** <one or two lines — who depends on whom, one-way>.
- **Module map:** <one line per module this plan touches — `path` · responsibility · depends on. From the context file's §1 Affected modules, placed within the spec's §7.3 boundaries; every path `[verified]`.>
- **Lexicon (from context §4b — do not rename):** <the shared names every slice uses verbatim: components, hooks, factories, params, enums, cycles>.
- **Tests:** <the plan-wide test convention and its justification — new files vs extending which existing ones>.

---

## Slice <N> — <verb-led name>

**Key decision:** <2-4 sentences: what this slice achieves and the approach it takes — the slice-level overview, citing Part 1 decisions by number rather than re-explaining them. A reader gets the shape of the slice from this line alone.>

**Structure**

- `<path>` (<new | modify>) — <symbols with method names and signatures (argument types, return type); never bodies. Mark every externally-sourced fact `[verified]` or `[inferred]`.>

**Collaborators:** <what this slice constructs and calls, and the dependency direction it must respect.>

**Control flow:** <the shape only — branches, guards, error paths, as an outline.>

**Tests:** <new file vs extend which existing file; which ACs each case covers; test level per project convention.>

**Standards that bite here:** <the specific craft principles and pack rules that constrain THIS slice, cited (e.g. `Clean Code: SRP`, `HIPAA 164.312(b)`, an org complexity ceiling).>

**Quality checks**

- <one-line verifiable assertion of a property the finished slice must have> (AC x.y.z)
- <flag the single highest-risk assertion in bold when one clearly dominates>

**Verification (slice complete when these pass)**

1. **Tests.** Run the full test suite — all tests green. Tests broken by this slice's changes get fixed inside this slice, not deferred.
2. **Migrations.** Apply all migrations against the project's database; they apply without error. <omit when the slice does not change schema>
3. **User-facing flow.** Drive the slice's primary user-observable behavior end-to-end in the browser session — what the ACs promise actually happens in the running app. <omit when the slice has no UI surface>
4. **Accessibility.** Same session: tab-order traversal, focus visibility on every interactive control, aria per the project's design system. <omit when the slice has no UI surface>

<Optional — only for genuine coder latitude or environment-specific facts (see the *Avoid deferring decisions* principle):>
**Open at build time (AC x.y.z):** <the open point, why it can only be resolved at build time, and the exact probe the builder runs to resolve it.>

---

## Hard floors

<`None. <one line on why no slice touches a designated high-risk category>` — or one line per floored slice: the slice, the triggering category, floor tier `≥ excellent`.>

## Escalation log

<Empty at authoring. `/anthara:plan-implementer` appends one-way tier bumps: date, slice, `<from> → <to>`, reason. Caught-before-build corrections recorded here too, with a note on what the confidence markers should have prevented.>
````
