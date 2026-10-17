# Pattern analysis — diffing the broken path against a working analogue

Read this at Step 5 of `/anthara:debug-fix`, after reading the failing path and before ranking hypotheses.

## Contents

- Why an analogue is worth finding
- Where to look for one
- Read it completely
- The difference ledger
- Analogue classes that pay off most
- When no analogue exists
- The trap: copying instead of understanding

## Why an analogue is worth finding

Somewhere nearby, something very similar works. That working thing is a free oracle: it encodes every convention, every piece of required wiring, and every unstated assumption the broken path is supposed to satisfy — without anyone having to remember what those are.

Reasoning about broken code alone means asking *"what could be wrong here?"*, which has an unbounded answer space. Diffing against a working analogue asks *"what is different here?"*, which has a finite, enumerable one. That is why this step usually collapses the hypothesis set from vague to specific in a single pass.

## Where to look for one

In descending order of value:

1. **The same file or class** — a sibling method that does the same kind of thing and works. The strongest analogue there is: same conventions, same imports, same test harness.
2. **A sibling module in the same repo** — the other controller, the other guard, the other workflow definition, the other migration.
3. **The same feature for a different type** — the working path for `Account` when `Dispatch` is broken. Especially strong when both were written by the same hand at the same time.
4. **A sibling repo in the multi-repo** — `api`, `fabric`, `coding-service`, and `web` each solve wiring, auth, retry, and configuration problems in their own idiom. Cross-repo analogues are weaker (different stacks, different conventions) but still informative for architectural questions.
5. **This same file, at a commit where it worked** — `git show <good-ref>:<path>`. When the regression window from `reproduction.md` is known, this is often the single most direct analogue available, because the only differences are the ones the window introduced.
6. **The test suite** — a passing test that exercises a nearly identical path. What its setup does that yours does not is frequently the whole answer.

## Read it completely

Read the analogue end to end. Do not skim it, and do not stop at the first difference you find.

Skimming defeats the technique, because the value is in the differences you were *not* looking for. The import you would not have questioned, the decorator two lines above the method, the base class it extends, the module that registers it, the fixture its test pulls in — these are exactly where the cause hides, precisely because nobody thinks to check them.

Read the analogue's wiring too, not just its body: how it is registered, what module declares it, what its test file sets up.

## The difference ledger

Enumerate **every** difference between working and broken. Then mark each one — and the marking is the discipline, because the temptation is to dismiss things silently.

| # | Difference | Verdict |
|---|---|---|
| 1 | analogue is registered in the module's `providers`, broken path is not | **might matter — untested** |
| 2 | analogue takes an explicit tenant argument, broken path reads it from context | **might matter — untested** |
| 3 | analogue's test seeds two rows, ours seeds one | cannot matter — assertion is on shape, not count |
| 4 | different variable naming | cannot matter — cosmetic |

Three verdicts only:

- **cannot matter** — and you can state why in one clause. If you cannot, it is not this verdict.
- **might matter — untested** — a hypothesis candidate. Carry it into Step 6 with a prediction attached.
- **is the cause** — reserved for a difference you have actually probed and confirmed. Never assigned on inspection alone.

"Probably irrelevant" is not a verdict. Every difference gets one of the three, with a reason.

## Analogue classes that pay off most

In practice, the differences that turn out to matter cluster in a few places:

- **Registration and wiring** — a provider, controller, handler, route, or subscriber that was written but never registered. It compiles, it type-checks, it is simply never called. The analogue's module declaration reveals it immediately.
- **Dependency injection** — the module imports a different module than its name suggests, or a token is provided in the wrong scope, so the injected instance is not the one under test.
- **Decorator and middleware order** — guard before interceptor, validation before transformation. The analogue has them in the order that works.
- **Transaction and connection scope** — the analogue runs inside a transaction or against a specific connection; the broken path does not, so its writes are invisible to its own reads.
- **Migration and schema drift** — the analogue's table has a constraint, index, or default the new one lacks.
- **Mock and interceptor setup** — the analogue's test registers the interceptor per-call; yours registers it once and the second call goes to the real network.
- **Configuration and environment reads** — the analogue reads config through the shared accessor with a default; the broken path reads it raw and gets `undefined`.
- **Declarative definitions** — for workflow YAML, pipeline stages, and pack rules, the working sibling defines the exact schema and required keys. Diffing two declarations is faster than reading the engine that consumes them.

## When no analogue exists

Say so, and move on to Step 6 without one. Genuinely novel code — the first integration of its kind, a greenfield module, a brand-new step type — has no analogue, and that is a real finding worth stating: it means the conventions this code should follow have not been established anywhere, which is itself a plausible root cause.

Do not manufacture an analogue. A superficially similar path that solves a different problem produces a difference ledger full of noise and sends the hypothesis set in the wrong direction.

## The trap: copying instead of understanding

The failure mode of this technique is finding a difference and immediately making the broken path match the analogue, then declaring victory when the loop turns green.

That is a symptom fix wearing a convincing disguise. Making two things identical can turn a loop green for reasons unrelated to the actual defect — and it silently imports the analogue's own assumptions, which may not hold in the new context.

The difference is a hypothesis. It still needs a prediction, a probe, and an articulated mechanism explaining *why* that difference produces *this* symptom. Only then is it the root cause, and only then does the fix follow.
