---
name: create-ticket
description: Creates a tracker ticket in two modes. **Spec mode** — pass a spec; the ticket carries title, description, hierarchical ACs (functional + compliance), pack labels, and Anthara metadata. **Ad-hoc mode** — no spec; elicits title, description, ACs, and an optional one-line "where does this live?" hint refined by a single grep. Pushes via the connected tracker MCP (Atlassian / Linear / GitHub Issues / Azure DevOps) after preview and confirmation. Records the artifact link to Org Memory. Use when the user runs /anthara:create-ticket, says "create a ticket", "push this to Jira / Linear / GitHub", "file a bug", "open an issue", or wants tracker work from a spec OR from an ad-hoc description.
argument-hint: [optional spec URL or file path]
allowed-tools: Read, Write, Edit, Bash
---

# /anthara:create-ticket

Turn a spec into a tracker ticket faithfully. One ticket per spec by default. All acceptance criteria preserved (functional + compliance) with the spec's hierarchical numbering. Pushed via the connected tracker MCP only after the user previews and confirms.

## Operating principles

- **One ticket per spec by default.** Decomposition into epic / story / task only when the spec contains genuinely independent value streams. Mechanical decomposition by section, slice, or NFR pack is not a value stream.
- **Two modes, one skill.** Spec-mode: rich ticket inheriting slices, ACs, packs, architectural style, source links. Ad-hoc mode: light ticket with title + description + ACs + optional affected-area hint + regulated-context labels. Both modes preview and push the same way.
- **Compliance ACs are non-negotiable.** Every compliance AC from every slice carries to the ticket. Dropping one is a defect.
- **Metadata fidelity.** Spec UUID, regulatory tags, NFR pack list, architectural style — these round-trip exactly. Where the tracker has custom fields, use them; otherwise put metadata in a fenced block at the bottom of the description.
- **No silent push.** The user sees a full preview and confirms before any tracker write. Push failures surface the error verbatim.
- **Preserve hierarchical numbering.** Spec ACs are `5.1.1`, `5.1.2`, etc. Carry the same numbering in the ticket as a markdown checklist (`- [ ] **5.1.1** ...`).
- **Tool discipline.** Closed questions go through `AskUserQuestion`. Per-turn budget: at most one open question per turn.
- **Tracker-format conformance.** Detect which tracker is connected and adapt to its schema. The skeleton below works on any tracker; per-tracker mapping is delegated to the LLM's judgement.

## Step 1: Detect mode (spec or ad-hoc)

- *Argument provided* — read it. If it is an Anthara spec (header `# Spec:`, hierarchical AC checkboxes in §5), enter **spec mode** (Step 2 onward). If the argument is a description or pointer to a non-spec file, enter **ad-hoc mode** (Step 1b).
- *No argument and conversation context has a recent spec* — call `mcp__plugin_anthara_anthara-fabric__search_memories("recent spec", scope="shared")` on fabric MCP. If a single recent spec surfaces, enter spec mode against it. If multiple, ask which via `AskUserQuestion`. If none, ad-hoc mode.
- *No argument and no spec in context* — ad-hoc mode.

## Step 1b: Ad-hoc elicitation (ad-hoc mode only — skip if spec mode)

The user has work to file but no spec. Gather the minimum to create a useful ticket — never more.

1. Open question: *"What's the work in one paragraph? Title-friendly summary first, details after."*
2. Open question: *"What does done look like? List the acceptance criteria as `- [ ] <text>` checklist items."*
3. Open question (single attempt; never push if the user can't answer): *"Where in the codebase does this live? Module / file / domain hint."* If the user gives a hint, do a single `Glob` or `Grep` to refine to a concrete path and add to the ticket as *"Best-guess affected: `<path>`"*. If skipped, omit the field.
4. Detect regulated context via `mcp__plugin_anthara_anthara-fabric__get_standards(<repo_name>)` if running inside a repo. Active pack list (HIPAA / PCI / SOC 2 / FDA SaMD / GDPR / WCAG / OWASP / etc.) drives the `pack:<name>` labels on the ticket — the audit trail carries even when there's no spec.

No `code-orienteer` spawn, no architectural style probe, no design probe. The ticket is a lightweight scheduling artifact; deep orientation happens at `/anthara:plan-implementer` time if the ticket gets picked up there.

## Step 2: Detect target tracker

Discover which tracker MCP is connected:

- *Atlassian (Jira)* — via Atlassian MCP.
- *Linear* — via Linear MCP.
- *GitHub Issues* — via GitHub MCP or `gh` CLI in the local environment.
- *Azure DevOps* — via Azure DevOps MCP.

If multiple are connected, ask via `AskUserQuestion` which to use.

If none are connected, surface the absence — *"No tracker MCP detected (Atlassian / Linear / GitHub / Azure DevOps). Printing the composed ticket inline; copy-paste into your tracker manually."* — and render the full ticket body in the conversation.

Detect the project / repository / team / board within the chosen tracker. If obvious from context (single project, single team, or `mcp__plugin_anthara_anthara-fabric__search_memories` returns one recent target), use it silently. Only ask via `AskUserQuestion` when genuinely ambiguous.

## Step 3: Decide decomposition

Default: **one ticket per spec**.

Decompose only if the spec contains genuinely independent value streams. Test: can the two parts ship in different releases without breaking each other? Do they touch disjoint slices, disjoint modules, and feature-flag separately? If yes, propose decomposition. If no, one ticket.

When decomposition is genuinely warranted, ask via `AskUserQuestion`: *"This spec has two independent value streams: [A] (slices 5.1-5.5) and [B] (slices 5.6-5.10). One ticket or two?"* — make `one ticket` the default option.

Do not decompose by section (§5 vs §6 vs §7), by pack (HIPAA vs OWASP), or by component layer. Those are not value streams.

## Step 4: Compose the ticket

Use the canonical structure below. Carry every slice and every AC. Where the tracker has structured fields (priority, labels, custom fields), populate them; where it does not, embed in the description.

**Title** — short, scope-revealing. Format: `[Spec: <slug>] <one-line summary from spec §1>`.

**Labels** — one per pack (e.g., `pack:hipaa`, `pack:owasp`, `pack:wcag`, `pack:clean-code`), one for the spec UUID (`spec:<uuid-prefix>`), one for regulatory class if any (`regulated:phi`, `regulated:pci`).

**Assignees** — leave unset by default. The user can populate during the preview step.

**Priority** — ask via `AskUserQuestion` with the tracker's standard scale (e.g., Linear: urgent / high / medium / low / no priority; Jira: highest / high / medium / low / lowest). Default option is medium / normal.

**Description body** — see canonical ticket structure below.

## Step 5: Preview and confirm

Render the full ticket to the user as it will appear in the tracker. Allow inline edits via natural-language request: *"change the title"*, *"drop the WCAG label"*, *"add @sarah as assignee"*, *"shorten the overview"*. Apply the edit, re-show the preview.

When the user is satisfied, ask via `AskUserQuestion`: *push to <tracker> / save as markdown locally / cancel*.

## Step 6: Push via MCP

Call the connected tracker's MCP tool with the composed payload. On success, capture the tracker URL and ticket ID and surface them.

If the push fails (auth, rate limit, schema validation, missing field), surface the error verbatim. Do not retry silently. Offer via `AskUserQuestion`: *retry / save as markdown locally and resolve the error / cancel*.

## Step 7: Record link in Org Memory

Call `mcp__plugin_anthara_anthara-fabric__add_shared_memory()` on fabric MCP with a brief summary: `Spec <uuid> at <spec-url> → Ticket <tracker-id> at <ticket-url>`. This makes the link queryable from `/anthara:explain` and `/anthara:audit` later.

## Step 8: Hand off

Tell the user:

- The ticket location (URL).
- The spec → ticket link is recorded in Org Memory.
- `/anthara:plan-implementer` executes slices in order and ticks off ACs as they land — both in the ticket (via tracker MCP) and in the spec.

Do not pronounce the ticket "ready to start" — the user owns that call. Open questions in §8 of the spec carry into the ticket and may block start.

## Canonical ticket structure (description body)

````markdown
## Overview

<2-3 sentences from spec §1 Overview & business context>

## Linked artifacts

- **Spec:** <url or path>
- **Architecture:** <link to ARCHITECTURE.md or "see spec §7">
- **Design:** <link to design.md or "n/a">
- **Discovery brief:** <link or "n/a">

## Architecture

**Style:** <named style from spec §7.2>
**Rationale:** <one line from spec §7.2>

## Acceptance criteria

(Tick `[x]` as ACs are implemented and verified. Compliance ACs are interleaved with functional ones; do not separate.)

### Slice 5.1 — <slice name>

- [ ] **5.1.1** <functional AC>
- [ ] **5.1.2** <compliance AC tied to an active pack rule>
- [ ] **5.1.3** <error-path AC>

### Slice 5.2 — <slice name>

- [ ] **5.2.1** ...

<continued for every slice>

## Active compliance packs

- **<Pack name>** — <one-line summary of what it covers>.
- **<Pack name>** — <...>.

## Open questions (block completion)

- [ ] **9.1** <open question>. Awaiting: <stakeholder / data / review>.

---

```yaml
# Anthara metadata — do not edit by hand
spec_uuid: <uuid>
spec_location: <url-or-path>
discovery_brief: <url-or-path or null>
architecture_doc: <url-or-path or null>
design_doc: <url-or-path or null>
slice_count: <n>
ac_count: <n>
packs: [hipaa, owasp, wcag, clean-code]
architectural_style: <named-style>
```
````

## Ad-hoc ticket structure (when no spec exists)

Lighter — no slice / ontology / architecture inheritance. Use this for ad-hoc mode only.

````markdown
## Description

<one paragraph from the user; the title-friendly summary first, then details>

## Acceptance criteria

- [ ] <AC from the user>
- [ ] <AC from the user>

## Best-guess affected (optional)

`<path/from/grep-refinement>` — based on the user's hint. Deep orientation deferred to `/anthara:plan-implementer` if this ticket gets picked up there.

## Active compliance packs (when running inside a regulated repo)

- **<Pack name>** — <one-line summary>.

---

```yaml
# Anthara metadata — do not edit by hand
source: ad-hoc
created_via: /anthara:create-ticket
regulated_context: <true / false>
packs: [<list of active packs if regulated; empty otherwise>]
```
````

## What NOT to do

- Do not auto-decompose into epic / story / task hierarchies. One ticket unless genuinely independent value streams.
- Do not push without user confirmation. Always show the preview first.
- Do not drop compliance ACs. They are not optional.
- Do not strip the spec's hierarchical numbering. Preserve `5.1.1`, `5.1.2`, etc.
- Do not invent ACs that are not in the spec. The ticket is a faithful representation, not a re-design.
- Do not silently retry on push failure. Surface the error.
- Do not skip the Org Memory record. The spec → ticket link is load-bearing for `/anthara:explain` and `/anthara:audit`.
- Do not change the spec's open questions when carrying them to the ticket — they round-trip verbatim.
