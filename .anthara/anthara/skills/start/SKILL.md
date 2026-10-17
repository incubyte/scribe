---
name: start
description: Triage router and the first command to run when the user doesn't know which Anthara command applies. Takes a description, classifies the work (bug / small feature / large feature / exploratory / existing artifact pointer), detects regulated context, and recommends the right chain entry — brainstorm, discovery, spec-writer, debug-fix, plan-implementer, create-ticket, or skip Anthara entirely. Always confirms before invoking. Use when the user runs /anthara:start, says "kick off a feature", "I want to build X", "we need to fix Y", "where do I start", "which Anthara command", or describes work without picking a command themselves.
argument-hint: <description of what to build, fix, explore, or a URL/path to an existing discovery or spec>
allowed-tools: Read, Bash, Skill
---

# /anthara:start

Triage router. Takes a description, classifies the work, recommends the right chain entry point, then invokes the chosen command. The user's first interaction with Anthara when they don't yet know which command to run.

## Operating principles

- **Closed questions ALWAYS go through `AskUserQuestion`.** No exceptions. Plain-text numbered questions are a defect. Before sending any message containing a question, classify each: closed (yes/no, choose-one, binary) → `AskUserQuestion`; open (the value is in the user's words) → text. Mixed messages send the open part as text and the closed part as a tool call in the same turn. If you find yourself writing `1. ... 2. ... 3. ...` as questions, stop — that is the failure mode.
- **At most 2 closed questions before routing.** If you need more elicitation, that's a discovery signal — invoke `/anthara:discovery` directly with what you have (Step 5). Even when the user's prompt explicitly asks for "clarifying questions," route to discovery — discovery is where elicitation belongs, not start.
- **Silent routing within Anthara; confirm only when bailing out.** brainstorm / discovery / spec-writer / plan-implementer / debug-fix / create-ticket are reversible — invoke directly via `Skill` with the structured context note from Step 6. Confirm only the "skip Anthara, fix manually" path, which exits the chain.
- **Detect regulated context once.** Pack detection drives whether trivial fixes get a spec or skip Anthara entirely.
- **Stateless.** start records nothing; the routed-to command does its own memory writes.

## Step 1: Read the description

Parse the user's argument for signal words and structure:

- **Debug/diagnose signals** — *"debug"*, *"why is this failing"*, *"diagnose"*, *"tests are broken"*, *"returns 500"*, *"getting an error"*, *"stack trace"*, *"flaky"*, *"intermittent"*, specific error messages or stack traces pasted directly, CI failure URLs.
- **Bug-fix signals** — *"fix"*, *"broken"*, *"not working"*, *"bug in"*, *"regression"*, specific symptom phrases (*"page 2 missing first item"*, *"login fails when..."*).
- **Small-feature signals** — *"add"*, *"build"*, with concrete scope (*"a button to..."*, *"an endpoint for..."*).
- **Larger-feature signals** — *"build a system"*, *"new product area"*, multiple actors mentioned, multi-stakeholder hints.
- **Brainstorm signals** — *"brainstorm"*, *"what are our options"*, *"how might we"*, *"what if we"*, *"help me think through"*, *"explore approaches"*, *"let's brainstorm"*, open-ended problem with no clear direction yet.
- **Exploratory signals** — *"explore"*, *"investigate"*, *"we're thinking about"*, *"figure out"*, *"research"*, no clear scope but has identifiable stakeholders or inputs to synthesize.
- **Pointer signals** — `https://...`, `docs/specs/...`, *"see the discovery doc at"*, *"per the spec at"*. The user has a starting artifact.

If a URL or file path is provided, that takes precedence — route to the next chain step from that artifact (Step 3 mapping).

## Step 2: Quick context scan

If running inside a repo (cap at ~5 reads — start is triage, not exploration):

- Compute `raw_remote_url = git remote get-url origin`.
- Parse `<owner/repo>` from `raw_remote_url` (e.g., `github.com/anthara-ai/plugin` → `anthara-ai/plugin`). Never `ORG_WIDE`.
- List `docs/specs/` for existing specs (informational — useful when recommending).
- Read `README.md`, `CLAUDE.md`, and one top-level build or dependency manifest if present (the project's primary manifest file — whatever format the stack uses). Note primary stack, frameworks, and major modules in 2-4 bullets. These bullets feed the **Codebase observations** field of the structured context note in Step 6 — so future skills don't redo this scan.

**Detect design state.** Classify as one of four states using stack-agnostic signals — look for the *role* each file plays, not the specific framework or filename:

- **Greenfield** — no repo at all, OR repo exists but contains no UI source files, no component library config, no design system files, no visual design docs.
- **Brownfield-no-system** — UI source files are present (any of: `components/`, `views/`, `screens/`, `pages/`, `templates/`, `assets/styles/`, `public/css/`, or similar), but no formal design doc and no authoritative token/theme file.
- **Brownfield-with-system** — `design.md` exists (at repo root or `/docs`), OR an authoritative theme/token/variable file is present and acts as the visual source of truth (any stack: CSS custom-properties file, `tokens.json`, `theme.*`, `styles/variables.*`, etc.).
- **Ambiguous** — mixed signals (UI code present, partial docs). Default to brownfield-no-system; surface the ambiguity in the context note.

**Detect UI-touching** from the description. Mark `yes` if the description contains words such as: screen, page, view, component, button, form, modal, dialog, dashboard, layout, navigation, UI, UX, frontend, app, interface, visual, design, style. Mark `no` if the description is exclusively about: API, backend, migration, job, worker, queue, CLI, script, infrastructure, data pipeline, or similar. When ambiguous, default to `yes`.

Call `mcp__plugin_anthara_anthara-fabric__get_standards(<owner/repo>)` on fabric MCP. Inspect active packs.

- **Regulated context** — any of `hipaa`, `pci`, `soc-2`, `fda-samd`, `gdpr` (or close variants by pack-name) are active.
- **Non-regulated context** — only general packs (`clean-code`, `owasp`, `wcag`, `test-integrity`, etc.) are active.

This drives the bug-fix routing in Step 3.

If fabric is unreachable: assume regulated by default (Anthara's value prop is regulated industries; safer assumption). Note the assumption to the user.

**Pack-canonical seeding (fabric-reachable only, idempotent).** After `mcp__plugin_anthara_anthara-fabric__get_standards()` returns the active pack list, check whether seeding has already happened for this repo by running `mcp__plugin_anthara_anthara-fabric__search_memories("[GLOSSARY <project> derived_from_pack=<any-active-pack>", scope="shared", limit=1)`. If a result comes back, seeding has already run; skip. Otherwise, for each pack, seed its canonical terms / tokens / controls into fabric's shared memory:

- Compliance terms (e.g., HIPAA's "Protected Health Information", PCI's "Cardholder Data") → `mcp__plugin_anthara_anthara-fabric__add_shared_memory("[GLOSSARY <project> term='<X>' status=canonical derived_from_pack=<pack> def='<Y>']")`
- Compliance design tokens (WCAG contrast 4.5:1, focus indicators) → `mcp__plugin_anthara_anthara-fabric__add_shared_memory("[DESIGN-TOKEN <project> token_type=<X> name='<Y>' value='<Z>' pack=<pack>]")`
- Compliance controls → `mcp__plugin_anthara_anthara-fabric__add_shared_memory("[COMPLIANCE-CONTROL <project> pack=<pack> control_id=<id> description='<X>']")`

`<project>` is the `owner-repo` slug from `raw_remote_url`. Pinned `status=canonical` so downstream skills surface them as authoritative; can't be sharpened away. Skip silently when fabric is unreachable. See `docs/fabric-adoption.md` for the full convention.

**Detect RISK from description signals.** Risk is a second axis alongside the bucket. It flows downstream to every command — spec depth, plan-implementer retry budget, ship checklist, audit weight.

| Risk | Signals |
|---|---|
| **LOW** | Internal tool, low traffic, easy to revert, no regulated data, dev / staging only |
| **MODERATE** | User-facing surface, moderate traffic, some business logic, non-regulated data |
| **HIGH** | Payment flow, authentication / authorization, data migration, regulated content (PHI / PCI / etc.), high-traffic endpoint, hard-to-revert change, third-party integration with no idempotency |

If the description has explicit risk signals (*"payment"*, *"login"*, *"migrate"*, *"PHI"*, *"production database"*), classify accordingly. If unclear, ask once via `AskUserQuestion` at Step 4.

Risk threads forward as a hint to the invoked skill in Step 6.

## Step 2.5: Triage the process tier

START is the brain of the workflow. Before routing, classify the work into exactly one **process tier** — how much process the work warrants. The tier is authored here, on the strongest model (START always runs at the `best` tier per `${CLAUDE_PLUGIN_ROOT}/docs/model-tiers.md`), so the single most consequential routing decision is made once, well, and up front. The tier flows forward into Step 6's routing and (for `standard` / `full`) into plan authorship.

The three tiers differ on two orthogonal switches — **(A)** is a human-readable spec written for intent sign-off, and **(B)** is a structured plan produced:

| Tier | Spec? | Plan? | What runs |
|---|---|---|---|
| **quick-fix** | no | no | Direct change. A **bug** is fixed test-first; a non-bug trivial change gets a test only when the logic warrants one (Step 6 quick-fix path). |
| **standard** | no | yes | The work is well-understood — no human intent sign-off needed — but the machine routing is authored and inspectable: slices, per-slice agent model tiers, dynamic agent selection. No feasibility pass, no hard floors, no escalation ratchet. |
| **full** | yes | yes | Large / high-risk / regulated-material-touching work. Spec written and feasibility-passed against the code before the plan is authored. Hard floors enforced; escalation ratchet live. |

**Classify deterministically at the floor, then by the bucket+risk signal:**

1. **Floor to `full`** when the change *touches regulated material* OR is **HIGH** risk — regardless of size. The floor signal is whether **this change's blast radius touches regulated material** (PHI / PCI / cardholder data / auth / authorization surfaces actually in the diff), established per-change. It is **never** merely "a compliance pack is active org-wide" — packs are active across the whole org by design, so that signal would force every change to `full` and defeat the purpose.
2. Otherwise, **`quick-fix`** when the bucket is *trivial-fix* or *small-feature* AND risk is **LOW** AND the change touches no regulated material.
3. Otherwise, **`standard`** — the default for ordinary small / large features at MODERATE risk that need machine routing but no human-readable spec.

```mermaid
flowchart TD
  A[prompt / ticket] --> B{this change touches regulated material<br/>OR HIGH risk?}
  B -- yes --> F[full]
  B -- no --> C{bucket + risk}
  C -- "trivial-fix/small AND LOW risk AND no regulated touch" --> Q[quick-fix]
  C -- "otherwise" --> S[standard]
```

Surface the chosen tier and a one-line reason to the developer before any downstream work — *"Process tier: standard — small feature, MODERATE risk, no regulated material touched."* The tier name is threaded into the Step 7 context note as `Process tier: <tier>`.

## Step 3: Design probe gate

For **Small feature** and **Larger feature** buckets (detected in Step 1), run the design probe below before routing. Skip for all other buckets (debug, bug-fix, brainstorm, exploratory, pointer — those routes handle design themselves or don't need it).

The gate decision depends on the design state and UI-touching flag derived in Step 2:

| Design state | UI-touching | First spec? | Action |
|---|---|---|---|
| Greenfield | any | any | Run **full design probe** |
| Brownfield-no-system | yes | any | Run **extract-and-confirm probe** |
| Brownfield-no-system | no | — | Skip |
| Brownfield-with-system | yes | yes | Run **alignment check** (one `AskUserQuestion`) |
| Brownfield-with-system | yes | no | Skip — design system answers it |
| Brownfield-with-system | no | — | Skip |

"First spec" = `docs/specs/` is empty or does not exist (already checked in Step 2).

**Full design probe (greenfield).** Respect per-turn budget — one open question per turn, `AskUserQuestion` for all closed:

1. Open: *"Name two or three products whose look and feel you admire. What should this product learn from each?"*
2. `AskUserQuestion` — Density: *dense and information-rich / balanced / spacious and minimal*.
3. `AskUserQuestion` — Tone: *precise and clinical / professional and warm / playful and consumer / utilitarian*.
4. `AskUserQuestion` — Color mode: *light / dark / both / follow system*.
5. `AskUserQuestion` — Motion: *none / subtle / expressive*.
6. Open: *"In one word, what should a user feel the first time they use this?"*
7. `AskUserQuestion` — Accessibility floor: *WCAG AA / WCAG AAA / legally mandated minimum*.

**Extract-and-confirm probe (brownfield-no-system).** Read existing UI source files first, then:

1. Summarize the visual conventions you observed: dominant layout patterns, styling approach, color palette, type scale. Keep to 3-5 bullets. Describe in neutral terms — do not name specific libraries unless the user already has.
2. `AskUserQuestion` — Direction: *keep these conventions and formalize them / evolve in some areas (ask which) / stop drift, do not evolve*.
3. Open: *"What visual anti-patterns do you want explicitly banned going forward?"*

**Alignment check (brownfield-with-system, first spec).** One `AskUserQuestion`: *"I found a design system doc at [path]. Should new features follow it as-is, or has the direction evolved since it was written?"* Options: *follow as-is / it has evolved — let me explain / I'll update design.md first*.

Capture probe answers in the context block under `Design direction` (see Step 6). If the probe was skipped, set `Design direction: none — probe skipped`.

The design probe questions count toward the 2-question elicitation cap only if elicitation is also needed for routing disambiguation (Step 4). When the bucket is already clear and only the probe runs, the cap does not apply to the probe itself.

## Step 4: Heuristic classification

Map description signals + regulated context to a bucket:

| Bucket | Description signals | Regulated? | Route |
|---|---|---|---|
| Debug/diagnose | debug/diagnose, error message, stack trace, CI failure, *"why is this failing"* | either | `/anthara:debug-fix` |
| Trivial fix (regulated) | bug-fix, localized, user already knows the cause | yes | `/anthara:spec-writer` (bug-fix mode) → `/anthara:plan-implementer` |
| Trivial fix (non-regulated) | bug-fix, localized, user already knows the cause | no | Skip Anthara — suggest manual fix; offer `/anthara:create-ticket` for tracker entry |
| Small feature | clear scope, single slice expected | either | `/anthara:spec-writer` → `/anthara:plan-implementer` |
| Larger feature | clear scope, multi-slice / multi-stakeholder | either | `/anthara:spec-writer` → `/anthara:plan-implementer` (spec-writer's grill-me will fire) |
| Brainstorm | open-ended, no plan yet, *"brainstorm"*, *"options"* | either | `/anthara:brainstorm` → `/anthara:discovery` → `/anthara:spec-writer` → `/anthara:plan-implementer` |
| Exploratory | fuzzy, multi-stakeholder, *"explore"*, has inputs to synthesize | either | `/anthara:discovery` → `/anthara:spec-writer` → `/anthara:plan-implementer` |
| Existing artifact (discovery doc) | user pointed at a discovery brief | either | `/anthara:spec-writer` against that doc |
| Existing artifact (spec) | user pointed at a spec | either | `/anthara:plan-implementer` against that spec |

If the heuristic is unambiguous, advance to Step 6. Otherwise Step 5.

## Step 5: Targeted elicitation (only if heuristic is unsure)

**Hard cap: 2 closed questions max.** If the heuristic from Step 4 is already unambiguous, skip Step 5 entirely. After 2 closed `AskUserQuestion` calls, if the route is still unclear, that's a discovery signal — proceed to Step 6 with bucket `exploratory` and silently invoke `/anthara:discovery` with the structured context note. Do not open a third question.

**Per-message check.** Before sending any message in this step, classify every question: closed → `AskUserQuestion`; open → text. Plain-text numbered question lists are a defect (the model's default tendency under user pressure — resist it).

Pick from these closed-question options for the disambiguating `AskUserQuestion` calls:

- *"Is this a bug fix or new functionality?"* — bug / feature / refactor / exploring.
- *"How big do you think it is?"* — single file / a few files / new module / new product area.
- *"Risk level?"* — low (internal, easy revert) / moderate (user-facing, some business logic) / high (payment, auth, migration, regulated data, hard to revert).
- *"Do you have an existing discovery doc or spec already?"* — yes (with pointer) / no.

Skip questions the heuristic already answered. The goal is one or two questions, never a wall.

## Step 6: Choose and invoke the route

**The process tier (Step 2.5) drives the route; the bucket refines it.** Route by tier first:

- **quick-fix** → the **quick-fix execution path** below. No spec, no plan.
- **standard** → `/anthara:planner` (it authors the plan, then hands to `/anthara:plan-implementer`). No spec.
- **full** → `/anthara:spec-writer` (it writes the spec, then hands to `/anthara:planner`, which hands to `/anthara:plan-implementer`).

START triages and routes; it does **not** author the spec or the plan itself. Those live in their own skills (`/anthara:spec-writer`, `/anthara:planner`) because a `Skill` handoff ends START's turn — START cannot run a step *after* it routes. The chain unfolds skill-to-skill: `start → spec-writer → planner → plan-implementer` (full); `start → planner → plan-implementer` (standard).

### quick-fix execution path

A `quick-fix` change is applied directly — no spec file, no plan file. Routing splits on whether the change is a bug:

- **Bug** (Debug/diagnose or bug-fix bucket, or the user describes a defect): route through `/anthara:debug-fix`. Its own three gates already enforce the test-first shape — a tight red/green loop before any theorising (gate 1) and a failing regression test before the fix is written (Step 9) — so pass the symptom straight through rather than restating the instruction. Prefer the top rung of its repro ladder, a committed failing test, over a manual trigger. The bug is never patched without a reproducing test landing first.
- **Non-bug trivial change** (a small tweak the user already knows the shape of): apply the change directly in the current context. Add a test **only when the logic is non-trivial** — i.e. the change introduces or alters behavior a reader could get wrong. A pure rename, copy tweak, or config bump needs no test; a changed conditional or computed value does. State the test decision in one line when handing off.

Both branches reuse the existing `debug-fix` skill or direct editing — no new agent or path is introduced. For a non-regulated trivial fix the user may still prefer the "skip Anthara, fix manually" exit (confirm via `AskUserQuestion` as below).

### standard / full

Map the bucket from Step 4 (or `exploratory` if Step 5 hit the elicitation cap) to a route. Invoke the route's entry skill via `Skill` directly with the structured context note from Step 7 — no intermediate confirmation. The single exception is the "skip Anthara, fix manually" path: surface that one recommendation via `AskUserQuestion` (options: *Skip and fix manually / Create a tracker entry first / Route through Anthara anyway*) because it exits the chain.

| Bucket | Route |
|---|---|
| Debug / diagnose | `/anthara:debug-fix` |
| Trivial fix (regulated) | `/anthara:spec-writer` (bug-fix mode) → `/anthara:plan-implementer` |
| Trivial fix (non-regulated) | Confirm via `AskUserQuestion` — skip Anthara default; tracker-entry and route-anyway as alternatives |
| Small feature (standard tier) | `/anthara:planner` → `/anthara:plan-implementer` |
| Small feature (full tier) | `/anthara:spec-writer` → `/anthara:planner` → `/anthara:plan-implementer` |
| Larger feature | `/anthara:spec-writer` → `/anthara:planner` → `/anthara:plan-implementer` |
| Brainstorm | `/anthara:brainstorm` → `/anthara:discovery` |
| Exploratory | `/anthara:discovery` → `/anthara:spec-writer` → `/anthara:planner` → `/anthara:plan-implementer` |
| Pointer to discovery doc | `/anthara:spec-writer` |
| Pointer to spec | `/anthara:planner` (plan it) → `/anthara:plan-implementer` |
| Elicitation overflow (>2 questions needed in Step 5) | `/anthara:discovery` |

For multi-step routes, invoke only the **first** skill. The chain unfolds naturally — discovery's hand-off mentions spec-writer, spec-writer's mentions plan-implementer, etc. The chain is not fully automatic: spec-writer **pauses after writing the spec** for the user's review and invokes planner only once the user confirms it (its Step 11 review gate).

## Step 6.5: Spec and plan are authored downstream (not in START)

START does **not** author the spec or the plan — it routes to the skills that do. This is deliberate: a `Skill` handoff ends START's turn, so any step placed *after* a handoff is unreachable. Spec authorship lives in `/anthara:spec-writer`; plan authorship lives in `/anthara:planner`. The plan is still authored **once, on the `best` tier** (planner runs at `best`) — just in its own skill, not here.

The chain by tier:

- **quick-fix** — no spec, no plan; the quick-fix execution path above handles it directly.
- **standard** — `/anthara:start` → `/anthara:planner` → `/anthara:plan-implementer`. Planner authors the plan from the triaged prompt + code context (no spec).
- **full** — `/anthara:start` → `/anthara:spec-writer` → *user reviews the spec* → `/anthara:planner` → `/anthara:plan-implementer`. spec-writer writes the spec, pauses for the user's spec review (its mandatory Step 11 gate), and hands to planner only after the user confirms; planner authors the plan and hands to plan-implementer.

Thread the triage signals forward in the Step 7 context note so planner (and spec-writer) inherit them: `Process tier`, `Risk`, `Regulated material touched`. Planner re-derives them from the spec/code when invoked manually without a note. The plan artifact, its per-slice schema, the hard floors, and persistence to `docs/plans/` all live in `/anthara:planner` — see that skill.

## Step 7: Invoke the chosen skill with the structured context note

Invoke via the `Skill` tool. The argument is a **structured prose block** — loose prose, not JSON; downstream skills read what they need without parsing a schema. Required format:

```
<user's original prompt verbatim>

---
ROUTED FROM /anthara:start

Bucket: <bucket name from Step 6 table>
Process tier: <quick-fix | standard | full, from Step 2.5>
Risk: <LOW | MODERATE | HIGH>
Regulated packs active: <comma-separated pack names, or "none">
Regulated material touched by this change: <yes | no, from Step 2.5 floor check>
Codebase observations: <2-4 short bullets from Step 2's scan, if any; "none" if not in a repo>
Pointer: <URL or path if user provided one, else "none">
Design state: <greenfield | brownfield-no-system | brownfield-with-system | ambiguous | not-in-repo>
UI-touching: <yes | no>
Design direction: <summary of probe answers from Step 3, or "none — probe skipped">

Provisional elicitation questions (use, refine, or skip):
1. <Q1>
2. <Q2>
...
```

Rules:

- The block is required for every silent invocation. No exceptions.
- **Provisional elicitation questions** are the ones start *would* have asked if it kept asking — capture them here instead of asking the user. They land in discovery (or whichever skill is invoked) as a starting elicitation set, not a wall. Closed and open both fine — discovery's Tool discipline section routes closed ones through `AskUserQuestion`.
- The risk hint scales spec-writer's grilling depth and plan-implementer's quality-check depth — propagate it accurately.
- **Design direction** carries the design probe answers forward so spec-writer and discovery do not re-ask questions start already asked. When the probe was skipped, set it to `none — probe skipped` and let the downstream skill decide whether to run its own probe.

For "skip Anthara" routes (trivial fix in non-regulated context, user picked "fix manually" in Step 6's `AskUserQuestion`): tell the user explicitly, suggest the manual-fix shape (which file, which test to add), then exit. Do not invoke a skill, do not emit the context block.

## Rules

- **Render closed questions via `AskUserQuestion`** — yes/no, choose-one, binary. Plain-text numbered lists are a defect.
- **Cap elicitation (Step 5) at 2 closed questions.** Over the cap → invoke `/anthara:discovery` silently with the structured context block; do not ask a third. The design probe (Step 3) is separate and does not count toward this cap.
- **Invoke within-Anthara routes silently** — brainstorm / discovery / spec-writer / plan-implementer / debug-fix / create-ticket all run directly via `Skill` with the structured context note. Only the "skip Anthara, fix manually" path needs `AskUserQuestion` confirmation because it exits the system.
- **Detect regulated context every run** — the one fabric call start always makes; drives bug-fix routing.
- **Routing is start's job; framing is the routed-to skill's job** — never scope, frame, or pre-elicit on behalf of the downstream skill. Capture provisional questions in the context block instead.
- **Read and triage, don't write.** start does not modify files and does not record to Org Memory; the routed-to command does its own memory writes.
- **One open question per turn** (per Tool discipline). Closed questions go through `AskUserQuestion` in the same turn.
