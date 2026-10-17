---
name: ai-ergonomics
description: Domain expertise for evaluating how well code supports LLM-assisted development. Covers context-window friendliness (file / function size, nesting depth), explicit-over-implicit (types, magic strings, conventions, configuration), self-documenting module boundaries, test-as-spec coverage, and CLAUDE.md quality. Loaded by `code-orienteer` to flag AI-unfriendly areas in the task-context file, and by `reviewer` to catch new code that erodes AI-ergonomics. Loaded as context by other agents — not user-invokable.
---

# ai-ergonomics

How comfortable is the codebase for an LLM to navigate, understand, and generate correct code in? Code that's ergonomic for AI is also better for humans — the emphasis here is on what specifically helps or hinders LLM tools (Claude Code, Copilot, Cursor) and how reviewer agents should flag erosion.

## Why this matters

Every file an agent reads consumes tokens. Implicit conventions are invisible to AI. Tests are the best spec an LLM has. CLAUDE.md is the project's instruction manual. When any of these slips, the agent works with a partial view and falls back to generic patterns — leading to hallucinations, missed dependencies, and code that doesn't fit.

## Five dimensions to check

### 1. Context-window friendliness

LLMs work with a finite context window. Large files and long functions push the agent into partial-view mode.

- Files > 500 lines deserve scrutiny; files > 1000 lines are a strong signal to split.
- Functions > 50 lines force the LLM to track too much state — it loses variables, forgets early conditions, hallucinates return values.
- Nesting > 4 levels is hard to reason about correctly; LLMs fail silently here more than humans do.

**Recommend:** extract focused modules; break long functions into composed smaller ones; flatten nesting with early returns and guard clauses.

### 2. Explicit over implicit

LLMs generate better code when contracts are visible. Implicit conventions that exist only in developers' heads cause hallucinations.

- **Types and interfaces.** `function process(data: any)` gives the LLM nothing. `function calculateDiscount(order: Order): Money` tells it exactly what to generate.
- **Magic strings and numbers.** `"active"` in 12 places instead of `Status.ACTIVE` — the LLM will guess wrong strings.
- **Implicit conventions.** *"All services must call `validate()` before `save()`"* — unless it's in a type, a base class, or documentation, the LLM won't know.
- **Configuration.** Defaults buried in code instead of constants / config files are invisible to discovery.

**Recommend:** add types to public-API signatures; extract repeated values as named constants; document implicit conventions at the declaration site or in `CLAUDE.md`; use the type system to enforce patterns when possible (e.g., a `ValidatedOrder` type that can only come from calling `validate()`).

### 3. Self-documenting module boundaries

An agent should understand a module's purpose and interface without loading the entire codebase.

- **Index / barrel files.** Does each module export a clear public API, or does everything leak?
- **Dependency direction.** Can the agent tell what this module depends on? Circular dependencies force loading everything.
- **One-paragraph module docs** at the top of each module. LLMs read these.
- **Cohesion.** A `utils/` with 30 unrelated functions is an anti-pattern — the agent has to read all 30 to find the one it needs. Break into focused modules (`utils/dates.ts`, `utils/money.ts`).

### 4. Test-as-spec coverage

Tests are the best specification an LLM can read.

- **Critical paths tested.** Untested behavior = no guardrail for the agent.
- **Test names as requirements.** `test('should return 404 when user not found')` is a spec. `test('test1')` is noise.
- **Readability.** Tests with shared state, complex setup, or implicit assertions are opaque to the agent.
- **Proximity.** Co-located tests are easier for the agent to discover than tests in a separate tree.

### 5. CLAUDE.md and documentation quality

`CLAUDE.md` is the project's instruction manual for LLMs.

- **Exists?** Missing = zero project-specific guidance; agent falls back to generic patterns.
- **Accurate?** Stale docs are worse than missing ones — the agent follows wrong instructions confidently.
- **Actionable?** *"Write clean code"* doesn't help. *"camelCase for functions, PascalCase for types, kebab-case for file names"* does.
- **Key patterns documented.** Architecture, testing conventions, error handling, naming — the things agents need most.

## How to use this skill in your work

### When loaded by `code-orienteer`

After walking the affected modules, flag AI-ergonomic issues in the task-context file's *Patterns to follow* or *Known dragons* sections. Concrete examples to surface (when found):

- *"`apps/api/src/orders/order.service.ts` is 1,400 lines — splitting it would help downstream agents work on isolated concerns."*
- *"No types on `apps/api/src/dispatch/dispatch.service.ts` (`findExisting`); add `DispatchRecord | null` return type."*
- *"`CLAUDE.md` does not document the service-layer convention — agents may invent alternatives."*

These flags are advisory, not blocking. They tell slice-builder what to be careful about.

### When loaded by `reviewer`

A new code review pass: did the diff erode AI-ergonomics in the touched modules? Specific findings to raise:

- New function over 50 lines without justification.
- New `any` typed parameter or return where a concrete type was available.
- New magic string used in 3+ places without a named constant.
- New module without an index file or with circular dependencies.
- Test added with a vague name (`test('should work')`).

Severity: usually **medium** (worth addressing but rarely production-blocking). HIGH only when the erosion is structural (e.g., introducing a circular dependency).

## Anti-patterns the reviewer should catch

- Functions that pass through many layers of `any`-typed objects.
- Tests that exercise behavior without asserting it ("didn't throw" tests).
- Modules added to `utils/` with no clear single concept.
- Magic strings hardcoded in multiple files.
- Inheriting a project pattern, then breaking it without `CLAUDE.md` update.

These compound: each one alone is a minor nit; together they make the codebase opaque to the next agent. Flag them while the diff is small.
