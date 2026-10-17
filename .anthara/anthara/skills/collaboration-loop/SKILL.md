---
name: collaboration-loop
description: Resolves inline @anthara annotations in an Anthara spec or discovery brief. The user writes `> @anthara <question or instruction>` directly in the doc (Google-Docs-style threaded comment); this skill scans the doc, makes the implied changes consistently across all impacted sections, and replies inline with a resolution note. Use when the user runs /anthara:collaboration-loop, says "check my comments", "review my anthara annotations", "resolve the pending @anthara comments", or asks Claude to address inline feedback in a spec or discovery brief.
argument-hint: [optional spec or discovery doc URL or file path]
allowed-tools: Read, Edit, Bash, Glob, Grep
---

# /anthara:collaboration-loop

Resolve inline `@anthara` annotations in a spec or discovery brief. The doc is the conversation surface — the user writes annotations directly in it, this skill resolves them by editing impacted sections and replying inline. Google Docs threaded-comment experience, in markdown.

## Operating principles

- **Annotations are first-class.** Every `> @anthara <text>` in a spec or discovery brief is a directive or question that must be addressed before the loop ends.
- **Resolve consistently across the doc.** A change in one place often cascades — ontology to invariants to slices to NFRs to diagrams. Apply the change everywhere it propagates; leave unrelated content untouched.
- **Reply inline, do not delete.** When an annotation is resolved, append a reply directly below it (also a block quote) so the thread shows what was asked and what was done. The original stays as part of the change history.
- **Batch the clear, one-at-a-time the ambiguous.** Resolve unambiguous instructions in a single pass. Surface ambiguous, conflicting, or cascade-decision annotations via `AskUserQuestion`, one open question per turn.
- **Tool discipline.** Closed questions go through `AskUserQuestion`. Per-turn budget: at most one open question per turn; never bundle two. _Never bundle_ means: don't pack two annotations into one question, don't pack two outcomes into one option label, don't use semicolons or "and" to join distinct decisions in an option. If `AskUserQuestion`'s 4-question cap is reached, make a second sequential call — do not compress.
- **Identifier and citation style match the host doc.** Hierarchical decimal numbering, integer source IDs, no prefix schemes, no Claude tool names. Do not introduce new conventions.
- **Source attribution carries forward.** If resolving needs a new source (e.g., an annotation says "use Supabase"; you `WebFetch` Supabase auth docs), add it to Sources as a numbered entry and cite normally.
- **Surgical edits only.** Modify only the content the annotation impacts. Do not "improve while you're at it" — that's `/anthara:spec-writer` territory.

## Annotation format

```
> @anthara <question or instruction>
```

Block quote, prefix `@anthara`. Single line or multi-line. Lives anywhere in the doc body — within a slice, alongside an invariant, beside a finding. Examples:

```
> @anthara should this be many-to-many or one-to-many?
> @anthara change auth to Supabase
> @anthara this slice needs an error-path AC for invalid file format
> @anthara drop §7.4 — the data flow is obvious from the slices
```

## Resolution format

After resolving, append a reply block quote directly below the annotation:

```
> @anthara should this be many-to-many or one-to-many?
>
> ✓ Resolved 2026-05-09: Changed to one-to-many. A clinician has many patients; a patient has exactly one primary clinician. Updated §3.1 (Kinds of users), §4.2 (invariant), §5.1.1 (AC). Sources: [1, 5].
```

The original annotation is preserved. The user can manually delete resolved threads later; this skill does not remove them.

## Step 1: Locate the doc

- *Argument provided* — read the spec or discovery brief at the URL or path.
- *No argument* — call `mcp__plugin_anthara_anthara-fabric__search_memories("recent spec | recent discovery brief", scope="shared")` on fabric MCP. If a single recent doc surfaces, confirm with the user via `AskUserQuestion` before processing. If multiple candidates, ask which.
- *Detect doc type* from the header (`# Spec:` or `# Discovery:`). Cascading changes follow the host doc's section structure.

## Step 2: Scan for unresolved annotations

Find every `> @anthara` line that is not already followed by a `✓ Resolved` reply. Index them with location (section number, surrounding context).

If none are found, tell the user and end.

## Step 3: Classify each annotation

- *Clear instruction* — *"change auth to Supabase"*, *"this slice needs an error-path AC"*, *"drop §7.4"*. Action is unambiguous.
- *Question* — *"should this be many-to-many or one-to-many?"*. Requires a decision; some are answerable from corpus / web / `mcp__plugin_anthara_anthara-fabric__search_memories`, others need the user.
- *Suggestion* — *"consider adding a slice for password reset"*. The user invited a recommendation; you decide whether to apply or push back.
- *Conflict* — annotation A contradicts annotation B, or the change requires information you cannot resolve. Surface both and ask.

## Step 4: Resolve clear instructions in batch

For each clear instruction:

1. Identify the cascade — which sections, invariants, slices, NFRs, diagrams are impacted by this change.
2. Apply changes consistently. Leave unrelated content alone.
3. Verify externals if relevant. `WebSearch` / `WebFetch` product docs the annotation references. Add fetched URLs as numbered sources in §2.
4. Replace the annotation with the resolution thread (annotation + reply). Reply lists the cascade explicitly so the user can audit.

If a cascade touches a regulatory commitment, surface it before applying — *"Changing auth from Auth0 to Supabase replaces the SOC 2 audit-trail vendor. Confirm or revert?"* — via `AskUserQuestion`. Do not apply silently.

## Step 5: Resolve ambiguous, question, or conflicting annotations one at a time

For each:

- Anchor the question in the annotation's exact content and the surrounding section. *"§3.2 currently has clinician-patient as many-to-many; the annotation asks if it should be one-to-many. Two reads: (a) one primary clinician with specialists allowed; (b) strict one-to-one. Which fits your model?"* — `AskUserQuestion`.
- Verify externals first when the annotation references a product or standard. `WebFetch` the relevant doc; cite.
- Surface conflicts explicitly. *"§1 annotation says Supabase; §6.2 annotation says Auth0 for SOC 2 reasons. These conflict. Which goes in the spec?"* — `AskUserQuestion`.
- One open question per turn — and each option resolves the question to one concrete outcome. No semicolon-joined or "and"-joined option labels; no options that bundle the verdict on a *different* annotation. Resolve, apply cascade, reply, move on.

## Step 6: Update the Changelog

After resolving (or when the user halts), append one Changelog entry summarizing the session: date, contributor, the annotations resolved with their location. Cascading changes are listed inside each individual reply, not the Changelog — keep the Changelog readable.

## Step 7: Hand off

Tell the user:

- How many annotations were resolved, how many deferred (with reason — usually awaiting external input or user decision).
- The doc has been updated in place at `<location>`.
- If cascading changes touched downstream artifacts (test catalog, ticket, design review), suggest which Anthara command to re-run to propagate.

Do not pronounce the doc final — the user owns that.

## What NOT to do

- Do not resolve an annotation by just replying — without changing the doc. If nothing changed, the annotation was rhetorical and you should ask the user how to interpret it.
- Do not modify content the annotation does not impact. Surgical edits only.
- Do not collapse two annotations into one resolution unless they are genuinely the same question. Conflicts become explicit decisions, not silent merges.
- Do not skip the Changelog.
- Do not silently apply a cascading change that affects a regulatory commitment, an existing decision in Org Memory, or a downstream artifact without surfacing the impact via `AskUserQuestion`.
- Do not remove the original annotation. Resolution is a reply; the thread stays.
- Do not invent new conventions. Match the host doc's identifier style, citation style, section structure.
- Do not bundle two open questions in one message. One per turn.
