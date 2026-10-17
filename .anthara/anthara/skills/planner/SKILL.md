---
name: planner
description: Authors the execution plan — the prescriptive "how" that lets a lower-tier model write good code. Runs on the best model, once per ticket. Given a spec, a ticket, or a prompt, it gathers code context (code-orienteer), then writes a per-slice plan carrying design structure (classes, method signatures, collaborators, control-flow outline), test strategy (new file vs modify existing), standards to honor (code-craft + get_relevant_standards), a single build-agent tier per slice, and per-slice quality checks. Applies hard-floor marks and persists to docs/plans/. Use when the user runs /anthara:planner, says "plan this", "make a plan", "plan the spec/ticket", or has been routed here by /anthara:spec-writer or /anthara:start. Hands off to /anthara:plan-implementer.
argument-hint: <spec path, ticket id/URL, or a prompt describing the work; optional ROUTED-FROM context note>
allowed-tools: Read, Write, Edit, Bash, Agent, Skill, WebSearch, WebFetch, Glob, Grep
---

# /anthara:planner

Author the **plan** — the prescriptive *how* of execution. The plan is the artifact that lets `/anthara:plan-implementer` run on cheaper model tiers: the strongest model makes the design and routing decisions **once, here**, and writes them down as a reviewable file the executor follows rather than re-deriving per slice.

Planner always runs on the `best` tier itself (`${CLAUDE_PLUGIN_ROOT}/docs/model-tiers.md`). It is the third link in the chain — `spec-writer → planner → plan-implementer` for full-tier work, `start → planner → plan-implementer` for standard-tier work — and is also directly invokable on any spec, ticket, or prompt.

## Operating principles

- **Plan = structure and decisions, never implementations.** The plan carries names, signatures, collaborators, control-flow shape, test strategy, standards. It never carries method bodies, algorithms, or written-out test code — those are the executing model's job. The dividing line is what keeps the plan cheaper to author than the code is to write; cross it and planning becomes as expensive as the work it was meant to make cheap.
- **Best model plans once.** Planner runs on `best`, authors the plan a single time, and persists it. plan-implementer does not re-plan or re-pick models; it follows the plan.
- **Models named only by tier.** The plan references `best` / `excellent` / `fast`; concrete model names live solely in `${CLAUDE_PLUGIN_ROOT}/docs/model-tiers.md`.
- **Self-sufficient.** When a `ROUTED FROM` context note is present (from start or spec-writer), use its process tier / risk / regulated-material signals. When absent (manual run), re-derive them from the spec's NFRs and packs and from code-orienteer's findings — planner runs on `best`, so it is capable of the judgment.
- **Every `§` reference is a clickable link.** Whenever the plan cites a section of the spec or the context file (`§5.2`, `§4b`), write it as a markdown link to that heading's anchor in the target file — `[spec §5.2](../specs/<NNN>-<slug>-spec.md#52-<heading-slug>)`, `[context §4b](../specs/<NNN>-<slug>-context.md#4b-<heading-slug>)` (the plan lives in `docs/plans/`, so spec/context links start with `../specs/`). Derive the anchor GitHub-style from the exact heading text: lowercase it, strip every character except letters, digits, spaces, and hyphens, then turn spaces into hyphens — `### 5.2 Ship the "Add to cart" button` → `#52-ship-the-add-to-cart-button`. A bare unlinked `§N` whose target has a real heading is a defect: the executor and the human reviewer must be able to jump straight to the cited section.
- **The plan defers to the context file for naming.** The plan owns *what to build* (structure). The task-context file owns *what the existing code calls things* (lexicon). Where they touch, the plan inherits names from the context file's lexicon so the executor never arbitrates a naming conflict.
- **Avoid deferring decisions; research instead of punting.** A plan that says *"the implementer picks"* or *"confirm the exact library error"* has pushed a decision onto a cheaper model — defeating the point of planning once on the strongest one. If you are unsure of a technical detail (a library's error type or API signature, an SDK idiom, a framework convention, a published advisory id for a fixture), **resolve it now**: `WebFetch` the official documentation or source, `WebSearch` for the authoritative answer, or read the installed dependency's source in the repo. Fold the resolved fact into the slice's Structure / Control flow / Test strategy so the executor inherits a decision, not a question. (Web research is independent of fabric — it works even when `get_relevant_standards` is unreachable; a fabric outage is never a reason to leave a verifiable fact unresolved.) Only two kinds of openness are legitimately left to plan-implementer: **genuine coder latitude** (a local stylistic choice with no downstream blast radius, where the invariant is fixed and only the idiom is open) and **environment-specific facts** (e.g. "can this CI provision a local DB?") that can only be answered by probing the machine at plan-implementer time. Everything else — anything a document or source file would settle — is resolved here, not punted.

- **Structure from the skeleton; naming from the spec.** Part 1 / Part 2, the per-slice fields, Hard floors and the Escalation log come from `references/canonical-plan-skeleton.md` verbatim — never from a neighbouring plan in `docs/plans/`, which may predate the current skeleton. The plan's filename reuses the spec's **stem** (the spec filename minus `-spec.md`) with `-plan.md`, so the two correlate whether or not the repo numbers its specs; the context file uses the same stem with `-context.md`. Wherever `<NNN>-<slug>` appears in these instructions it means that stem.

## Step 1: Locate and classify the input

Planner accepts three input kinds. Detect which:

- **A spec** — a path to `docs/specs/<NNN>-<slug>-spec.md` (or any Anthara spec: `# Spec:` header, hierarchical AC checkboxes in §5). Plan from the spec's slices, preserving spec slice order.
- **A ticket** — a tracker id/URL (Jira / Linear / GitHub). Fetch it via the connected tracker MCP; plan from its description + ACs.
- **A prompt** — a free-text description (standard-tier work with no spec). Plan directly from the prompt + code context.

Capture the spec's filename stem when a spec exists so the plan correlates with it — the plan is `<stem>-plan.md`, numbered or not per the repo's convention. For a ticket or prompt with no spec, derive a slug from the work description and follow whatever convention the repo's existing plans already use; with no existing plans either, create `docs/plans/` and start numbered at `001-<slug>-plan.md`.

**Read the `ROUTED FROM` context note if present.** When start or spec-writer invoked planner, the note carries `Process tier`, `Risk`, `Regulated material touched`, and `Codebase observations`. Use them. When absent (manual invocation), re-derive: read the spec's §6 NFRs / active packs and §7 architecture; infer risk from regulated-material touch and blast radius. Do not block on a missing note.

## Step 2: Gather code context (code-orienteer, once)

Spawn the existing `code-orienteer` agent (Agent tool, `subagent_type: anthara:code-orienteer`). It writes `docs/specs/<NNN>-<slug>-context.md` — reusables, patterns, dragons, test conventions, and the slice lexicon.

This single walk is **reused by `/anthara:plan-implementer`**: plan-implementer's Phase 0 checks *"context file exists + stamped HEAD matches current HEAD → reuse"*, so it will not re-walk when planner produced the file at the same HEAD. If HEAD shifts before plan-implementer runs, plan-implementer regenerates per its existing rule. `code-orienteer` is reused unchanged — no new orientation agent.

For a greenfield input with no existing code to orient against, the context file is sparse; the plan stands more on its own. That is expected — do not invent patterns that do not exist.

## Step 3: Ground in the craft skills and active standards

Before authoring, load the relevant domain expertise as context — these already encapsulate the principles, so the plan **cites** them rather than restating them:

- `code-craft` — naming, function design, single responsibility, error handling, encapsulation, categorical framing (clean-code / Uncle Bob principles live here).
- `architecture-craft` — the project's architectural style and dependency direction.
- `test-craft` — test pyramid, independence, meaningful assertions, reuse-over-reinvent.

When fabric MCP is reachable, call `mcp__plugin_anthara_anthara-fabric__get_relevant_standards(raw_remote_url, task_slots)` per slice to pull the surface-specific pack rules (HIPAA / OWASP / WCAG / PCI / clean-code / …) that apply to that slice's `surface_area` and `data_handled`. Fold those rules into each slice's **Standards to honor**. When fabric is unreachable, ground on the craft skills alone and note the degradation in the plan header.

**Resolve technical unknowns before authoring.** As you read the spec and context, list the external facts the slices depend on but you are not certain of — a library's error sentinels and API signatures, an SDK's idioms, a framework convention, a published advisory id needed for a test fixture. Resolve each now: `WebFetch` the official docs or the dependency's source on its repo, `WebSearch` for the authoritative answer, or read the installed dependency under the project's module cache / `vendor/`. This is independent of fabric — do it even when `get_relevant_standards` is unreachable. The goal is to enter Step 4 with these settled, so the plan states decisions rather than questions (see the *Avoid deferring decisions* operating principle).

## Step 4: Author the plan

Author **in spec slice order** (the spec — when present — is written before the plan, so the plan inherits and preserves that order). The plan must be **detailed enough that a lower-tier model writes good code from it**, yet **abstract enough that authoring it is not itself the expensive work** the cheaper executor was meant to avoid.

Read `references/canonical-plan-skeleton.md` and use it verbatim. The plan has **two parts for two audiences**:

**Part 1 — Review this** (first, for humans). Everything a reviewer needs to approve or redirect the approach, in plain language and at a higher level than the build detail — the reviewer understands the overall approach without diving into implementation:

- **What we're doing** — the goal, the core approach and the *constraint that forces it*, and any deliberate exceptions (named as deliberate).
- **How it breaks down** — the slicing strategy in a sentence or two, then the slice table (*Delivers / Why this tier / Tier*), plus cross-slice notes.
- **Decisions made** — the numbered decisions that are the plan's argument, each with its plain-language reasoning and the *rejected alternative*. Per-slice entries cite these by number instead of re-explaining.
- **Where this could go wrong** — the honest risk list, each item pointing at the Part 2 quality check that guards it. Always include scope-creep into the spec's §9 items that sit adjacent to a slice — a deferred defect in the same file as a slice's change will look like an obvious fix while that slice is open.
- **What's out of scope** — drawn from the spec's §1 *Deliberately out of scope* paragraph and its §9 *Deferred follow-ups and accepted limits*, each cited as a link. Name every §9 item a builder could plausibly touch in passing, so a reviewer can tell "not built" from "forgotten".

Author Part 1 **last**, after the slices and hard floors are done, so it summarizes real content rather than intent. Apply the §-links principle to every section, slice, and decision it mentions. Part 1 is written for the human reviewer; `/anthara:plan-implementer` and `slice-builder` may read it for orientation, but they execute from Part 2 — so never put a load-bearing detail *only* in Part 1.

**Part 2 — Build detail** (the executor's contract). Opens with the confidence-marker legend — `[verified]` = you read the code or docs and confirmed it; `[inferred]` = pattern-matched from a sibling, not confirmed, the builder checks before assuming; never state an unverified external fact in confident voice — then **Shared conventions (all slices)**, once:

- The architectural style and dependency direction the whole plan obeys (from `architecture-craft` and the spec's §7).
- The module map — each module the plan touches with its path, responsibility and what it depends on, drawn from the context file's Affected modules and placed within the spec's §7.3 boundaries. The spec draws boundaries by kind; the plan names the paths.
- The shared lexicon / naming conventions drawn from the context file, so names are consistent across slices and the executor never has to arbitrate.
- The plan-wide test convention, and whether fabric standards were available.

**Per-slice plan entry.** For each slice, record:

1. **Slice ref + order** — the spec slice ID (or derived slice id for a ticket/prompt); plan order matches spec slice order exactly.
2. **Key decision** — 2–4 plain-language sentences opening the slice: what it achieves and the approach it takes, citing Part 1 decisions by number rather than re-explaining them. This is the slice-level overview — a reader gets the shape of the slice from this alone before any implementation detail.
3. **Structure** — the classes / modules to create or touch, with method **names and signatures** (argument types, return type). **No method bodies.** Names inherit from the context file's lexicon where the slice touches existing concepts. Signatures take their collaborators as parameters rather than constructing them, so the slice's code lands testable without the implementer re-deriving the seam. Every externally-sourced fact carries its confidence marker (`[verified]` / `[inferred]`).
4. **Collaborators** — what this slice constructs and calls, and the dependency direction it must respect (honor `architecture-craft` and the project's locked style — outer depends on inner, never the reverse).
5. **Control flow** — the shape only: branches, loops, error paths, guard conditions, as an outline — never written-out logic. Prose, code, or pseudocode are all fine forms for the outline, but it must contain no comment lines (`//`, `#`, `/* */`): builders transcribe plans faithfully, so a comment in the plan becomes a comment in the source — and per `code-craft`, generated code adds no comments. Say it as a named step or a guard condition instead of an annotation.
6. **Test strategy** — decided here from `code-orienteer`'s observation of the existing suite: whether to **write a new test file or modify an existing one**, which ACs each test covers, and the test level (unit / integration) per the project's established convention. The slice-builder writes these tests after the slice's code, so name the seam each test drives through. This decision is the plan's, not re-litigated at plan-implementer time.
7. **Standards to honor** — the `code-craft` principles that bite on this slice plus the surface-specific rules from `get_relevant_standards` (cited by rule, e.g. `HIPAA 164.312(b)`, `Clean Code: SRP`).
8. **Build-agent tier** — the `slice-builder` tier for this slice (`best` / `excellent` / `fast`), resolved later via `${CLAUDE_PLUGIN_ROOT}/docs/model-tiers.md`. Tier follows the slice's implementation difficulty and consequence (harder, higher-risk slice → higher tier). One tier per slice; never a per-agent split.
9. **Quality checks** — the verifiable expectations the `slice-builder` must satisfy and the end-of-spec reviewer will verify. State these as concrete, checkable conditions drawn from the craft skills (`code-craft`, `architecture-craft`, `test-craft`, `ai-ergonomics`) and from any applicable compliance rules retrieved via `get_relevant_standards`. Two sources are mandatory whenever the spec carries them: every §10 *Established behaviour* entry that a slice's files touch yields an "X is unchanged" check, and every §9 deferred defect adjacent to a slice yields an "X is untouched" check — these are the assertions that stop a builder helpfully fixing what was deliberately left alone. Each check is a one-line assertion of a property the finished slice must have — **never a method body, algorithm, or written-out review finding**. Good examples: *"No dropped dimension (4.2): every dimension in the spec's §3.3 appears in the consolidated body"*, *"Tier discipline (4.6): no concrete model name anywhere; tier expressed as the alias \`excellent\`"*. Bad examples: *"Write a function that..."*, *"The reviewer should look for..."*. When fabric is unreachable, ground on the craft skills alone and note the degradation.
10. **Verification** — how "this slice works" is confirmed operationally at end-of-spec, in stack-agnostic shape language (never commands, file names, or routes): the full-suite step always; the migrations step when the slice changes schema; the user-facing-flow and accessibility steps when the slice has any UI surface. Author it from the skeleton's shape, keeping only the steps that apply; a spec written before Verification moved into the plan still carries a per-slice block — carry that over verbatim instead. `/anthara:plan-implementer` reads this field to decide which slices are UI slices, and `browser-verifier` drives steps 3–4 from it.

The plan references models **only by tier alias**. It is authored once and not re-derived downstream.

## Step 5: Hard floors (final step of authoring)

As the **last step**, apply the hard floors — non-negotiable marks that override whatever the plan proposed. For any slice whose affected files fall in a **designated high-risk category**, **mark the slice as floored** in the plan with a floor tier of `≥ excellent`. The floor signals to the single reviewer that security and architecture dimensions must run unconditionally over that slice's surface at `≥ excellent`, regardless of any cost consideration.

Designated high-risk categories (human-editable — edit this list to change the floor):

- **auth** — authentication, login, session, token issuance / validation.
- **authorization / guard** — access-control checks, route guards, middleware, RLS / policy files.
- **payment** — payment flows, billing, cardholder-data handling.
- **security** — crypto, secrets handling, signing.
- **migrations** — schema migrations and data migrations.

The planner **marks**; the single reviewer **enforces**. The floor tier is recorded in the plan and can **never be dropped or down-tiered** — it is a hard constraint the reviewer honors unconditionally at review time, not a suggestion the executor may override. A marked slice's floor tier may go higher than `excellent` but never lower.

## Step 6: Persist the plan

Write the plan to **`docs/plans/<stem>-plan.md`** — a dedicated, committed, PR-visible directory (create `docs/plans/` if absent; ensure it is not git-ignored). The stem is the spec's, verbatim, so spec, context file and plan correlate whether or not the repo numbers them; with no spec, follow whatever naming the repo's existing plans already use. Section order is fixed by `references/canonical-plan-skeleton.md`: metadata blockquote → `# Part 1 — Review this` → `# Part 2 — Build detail` (shared conventions → per-slice entries → `## Hard floors` — present even when none apply, stating why → **`## Escalation log`**, initially empty; `/anthara:plan-implementer` appends one-way tier bumps with reasons at execution time).

## Step 7: Hand off

Tell the user: the plan's location, a one-line summary (N slices, tier distribution, fabric-standards availability), and the next chain command. Anyone can re-run `/anthara:planner` against the same spec to revise the plan.

Hand off to **`/anthara:plan-implementer`** — the single plan executor. It spawns one `slice-builder` per slice at the slice's plan-assigned build tier, then spawns the single specialized reviewer (with the plan's hard-floor markings) plus one `pack-reviewer` per active compliance pack once at end-of-spec, then runs the full-suite gate.

Hand off via the `Skill` tool with a context note carrying `Plan: docs/plans/<NNN>-<slug>-plan.md` (plus the inherited `Process tier` / `Risk` signals), unless the user invoked planner standalone to inspect the plan first.

## What NOT to do

- Do not put method bodies, algorithms, or written-out test code in the plan — that is plan-implementer's job, and it defeats the cost model.
- Do not name a concrete model anywhere — tiers only; resolution lives in `${CLAUDE_PLUGIN_ROOT}/docs/model-tiers.md`.
- Do not invent names that conflict with the context file's lexicon — inherit existing names.
- Do not re-plan at plan-implementer time; the plan is authored once, here.
- Do not drop or down-tier a hard floor mark — the single reviewer enforces floors unconditionally and the floor tier can never be lowered.
- Do not punt a verifiable technical fact to the implementer (*"implementer picks"*, *"confirm the exact library error"*). Research it now (`WebFetch` / `WebSearch` / read the installed dependency's source) and write the resolved decision into the slice. Leave open only genuine coder latitude or environment-specific facts that require probing the machine.
- Do not put review prose or written-out findings into the Quality checks field — checks are verifiable assertions of properties the slice must have, never review commentary or method bodies.
- Do not write a bare `§N` reference — link every spec/context section citation to its heading anchor (see the §-links operating principle).
- Do not put a load-bearing detail only in Part 1 — the executor builds from Part 2; Part 1 is the reviewer's view of the same content.
- Do not state an external fact in Part 2 without its confidence marker. An `[inferred]` claim written in confident voice is how plans go wrong — the escalation log exists to record exactly that failure.
