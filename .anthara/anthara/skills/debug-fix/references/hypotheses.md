# Hypotheses — ranking, prediction, and probing

Read this at Step 6 of `/anthara:debug-fix`, once a tight loop exists and you have read the code.

## Contents

- Why competing hypotheses
- The hypothesis grammar
- How to rank
- The composition rule
- Showing the list to the developer
- Designing the probe
- The hypothesis ledger
- Probing in parallel
- When every hypothesis is refuted

## Why competing hypotheses

A single explanation is not a hypothesis — it is a conclusion you have not tested. The first plausible story about a bug is wrong often enough that committing to it is the most expensive mistake available: every subsequent read is biased toward confirming it, and contrary evidence gets explained away.

Holding three to five candidates at once removes that bias mechanically. You are no longer asking *"is this it?"* (a question that answers yes too easily) but *"which of these survives?"* — and each probe eliminates candidates whether it passes or fails.

## The hypothesis grammar

Every hypothesis has exactly two parts. Both are required.

**Mechanism** — a specific, causal statement about what is happening, naming the code that does it:

> The join in `findDispatches` matches on `user_id` against `accounts`, so tenants whose account id differs from their user id resolve zero rows.

**Prediction** — an observable consequence that would be *different* if the hypothesis were false:

> If true, seeding an account whose id differs from its user id makes the query return zero rows, and forcing the join onto `users` makes the same query return one.

A statement with no prediction is not a hypothesis. Discard it, or sharpen it until it makes one. The usual offenders:

- *"Something is wrong with the caching."* — no mechanism, no prediction. Discard.
- *"It is probably a race condition."* — a class, not a mechanism. Sharpen: which two operations, on which shared state, in which order?
- *"The environment is broken."* — untestable as written. Sharpen into a specific missing variable or unreachable service, which *is* testable.
- *"It is flaky."* — a description of the symptom, not a cause. Classify it against the flake table in `reproduction.md` instead.

## How to rank

Rank on two axes, and order the probe queue by their combination:

- **Prior likelihood** — how probable this mechanism is, given the code you just read, the regression window, and what org memory returned in Step 2.
- **Cost to falsify** — how cheaply this hypothesis can be killed. A `SELECT`, a `git show`, or reading one function is nearly free; standing up a second tenant is not.

**Probe the cheapest informative hypothesis first, not the most likely one.** A near-free probe that eliminates a mid-ranked candidate buys more than an expensive probe that confirms your favourite. What you are maximising is information per unit of effort, and elimination is information.

Present the list ordered by that combination, with both axes visible:

```
H1  [likely · cheap]     <mechanism> → predicts <observable>
H2  [likely · costly]    <mechanism> → predicts <observable>
H3  [possible · cheap]   <mechanism> → predicts <observable>
H4  [unlikely · cheap]   <mechanism> → predicts <observable>
```

## The composition rule

Three to five hypotheses. Fewer than three usually means you stopped at the first story; more than five means they are not yet distinct.

At least one must challenge an assumption rather than the code — *"my model of how this works is wrong."* These are the ones that actually resolve long debugging sessions: the file being executed is not the file being edited, the module is not importing what its name suggests, the test is asserting the wrong thing, the framework's default is not what you believe it is. The assumption checklist in Step 4 is the source for these.

Spread the candidates across layers where the evidence allows. Four hypotheses that all live inside the same function are one hypothesis wearing four hats.

## Showing the list to the developer

Surface the ranked list **before probing**, via one `AskUserQuestion` offering the top candidates as options plus the escape hatch. This is the single highest-value interaction in the whole skill: the developer often re-ranks it instantly from knowledge that is nowhere in the code — *"we migrated that table last week"*, *"that queue has always double-delivered"*, *"nobody has run that path since the rewrite"*.

Keep it closed and cheap. One question, the ranked mechanisms as options, no essay.

If the developer does not respond or is not present, proceed on your own ranking and **say that you did**. Never block the loop waiting for a re-rank — but never silently skip the offer either.

## Designing the probe

One hypothesis, one probe, one variable.

- **The probe must be able to refute.** If both outcomes leave the hypothesis alive, it is not a probe — it is a demonstration. Redesign it.
- **Smallest possible intervention.** Read a value, print a value, hardcode one intermediate, comment out one branch. Not a refactor.
- **Change one thing.** Two simultaneous changes mean a green loop tells you nothing about which mattered.
- **Prefer observation over mutation.** Reading state, querying data (`SELECT` only), and inspecting the network answer more questions than editing does, and leave nothing to clean up.
- **Revert the probe before the next one.** Probes are not cumulative. A stack of half-reverted probes is indistinguishable from a broken codebase.
- **Instrumented probes follow the tag protocol** in `instrumentation.md` — tagged, compliance-safe, and removed before the fix lands.

## The hypothesis ledger

Keep a running table. It is how you avoid re-probing a dead candidate, and it becomes §4 of the diagnosis record verbatim.

| # | Mechanism | Prediction | Probe | Result |
|---|---|---|---|---|
| H1 | … | … | … | refuted — returned 1 row, not 0 |
| H2 | … | … | … | **confirmed** — join key is `user_id` |
| H3 | … | … | not probed (H2 confirmed first) | — |

Record refutations as carefully as confirmations. "We proved it is not the cache" is durable knowledge, and it is exactly what stops the next engineer repeating the session.

## Probing in parallel

Serial is the default: probe, learn, re-rank, probe again. Each result should change what you do next, and that feedback is lost when everything runs at once.

Fan out `hypothesis-prober` agents only when **all four** conditions hold:

1. Three or more hypotheses remain live.
2. They are genuinely independent — no probe's design depends on another's result.
3. Every probe is **read-only**: reading code, running an existing scoped test, `SELECT`, `curl`, `git show`, inspecting the browser.
4. No probe needs instrumentation.

Bound the fan-out at four. Spawn at the `excellent` tier, or `best` when the mechanism is subtle. Any probe that must edit a file — including adding a tagged log line — stays serial in the main session, because parallel agents editing the same working tree corrupt each other's results.

Each prober returns confirmed / refuted / inconclusive plus its evidence. Fold every return into the ledger, then re-rank what is left; a fan-out that confirms two hypotheses at once means they were not independent, and both need re-probing serially.

> This is a deliberate, bounded exception to the fan-out anti-pattern in `ARCHITECTURE.md` §7 — see ADR 3. It is permitted here because the probes are independent, read-only, and short-lived, which is precisely the shape the review fleet was not.

## When every hypothesis is refuted

A fully refuted list is progress, not failure. Four things are no longer true, and that is real ground gained.

Return to Step 4 and re-read with the new constraints in hand. Then generate a fresh set, and deliberately widen the layer: if all four candidates lived in the service, the next set should reach into the framework, the wiring, the data, the test harness, and the environment.

Two consecutive fully-refuted rounds means the symptom is probably not where you are looking. Go back to Step 3 and question the repro itself — a loop that is red for a reason unrelated to the reported bug will refute every correct hypothesis about the actual one.
