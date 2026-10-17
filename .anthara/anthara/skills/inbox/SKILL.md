---
name: inbox
description: Role-tuned, channel-agnostic work aggregator. Pulls every connected work channel (Jira, GitHub, Slack, Linear, Notion, Gmail, Teams) via MCP, enriches each item with structured features, applies cross-source coupling, and asks the model to produce one flat ranked table of the work most worth doing now — tuned to the user's role and biased by an optional natural-language hint. First run elicits a profile (role, primary tools, time zone, usernames, manager) stored in fabric private memory; subsequent runs are instant. The rendered table is conversationally addressable — say "let's work on #3" or "ignore #5" and the model resolves rows from the in-context table. Use when the user runs /anthara:inbox, says "what should I work on next", "show me my inbox", "/inbox on-call today", "/inbox no meetings", "/inbox just got back from PTO", or wants a single prioritized view across all their work channels.
argument-hint: [optional natural-language hint, e.g. "on-call today", "no meetings", "just got back from PTO"]
allowed-tools: Read, Bash, AskUserQuestion, ToolSearch, Skill
---

# /anthara:inbox

A flat ranked table of the work most worth doing right now, drawn from every connected MCP channel and tuned to the user's role. One command. One table. No walls of text. Re-runnable any time. The table itself is conversationally addressable — `let's work on #3` resolves against the most recent render.

## Operating principles

- **The output is a flat ranked table of up to twenty rows. Never prose.** Columns: `#`, `Kind`, `Item`, `Why now`. The "Why now" column is the rationale audit trail; the most important signal goes in the first few words.
- **Channel-pluggability is the architectural commitment.** Adding a new channel = adding 2-3 lines of manifest, not editing code. Every connected MCP that returns work-bearing items is a channel.
- **Role tunes scope and ranking.** Junior IC sees personal feed only; staff/principal sees cross-org signal; EM sees people-shaped signals weighted up. Role is set once during the first-run wizard and reused.
- **The hint primes the ranker.** `/inbox on-call today` biases toward incidents and urgent reviews; `/inbox no meetings` biases toward focused work. The model interprets the hint as a ranking-policy modifier and surfaces its influence in the "Why now" column.
- **Cross-source coupling — bias under-couple over over-couple.** A Jira ticket linked to its PR linked to its Slack thread is one row, not three. When uncertain whether two items are the same work, show as separate rows. False positives kill trust faster than false negatives.
- **Memory writes happen only on explicit teaching.** Behavioral signals (user picked #3, ignored #5) influence the current run but never persist. Only when the user teaches — via answering a curiosity-ask or saying "remember that..." — does the model write `[INBOX-LEARNED ...]` to fabric.
- **No SLA, no per-kind staleness windows.** Performance is an orchestration concern (parallel tool calls, in-memory caching). Staleness is one feature among many the model reasons over; it is never the primary ranking dimension.
- **No dismiss / snooze / pin commands.** The Claude Code session's conversation context is the addressable state. `let's work on #3` and `ignore #5` resolve against the most recent rendered table.
- **Tool discipline.** Closed questions go through `AskUserQuestion`. Per-turn budget: at most one open question per turn.
- **Privacy lives at the MCP token, not in /inbox.** Read everything the connected MCPs allow. Token scoping is the user's responsibility; `/inbox` does not enforce a second-layer policy.

## Step 1: Load deferred tools and detect connected channels

Load fabric MCP tools and `AskUserQuestion` via `ToolSearch`:

```
ToolSearch(query="select:AskUserQuestion,mcp__plugin_anthara_anthara-fabric__add_shared_memory,mcp__plugin_anthara_anthara-fabric__search_memories")
```

A **channel** is any source that can be queried for *work-bearing items* — tickets, PRs, messages, threads, emails, calendar events, decisions awaiting input. Detect channels from **two surfaces**:

- **Connected MCPs** — inspect the available tool list and select MCPs whose tools return work-bearing items (Atlassian / Jira / Confluence, GitHub, Slack, Linear, Notion, Gmail, Microsoft 365 / Outlook / Teams, and any other MCP whose tools include search / list / read of assignments, PRs, messages, threads, or issues). For each detected channel MCP, load its query tools via `ToolSearch`.
- **Bash-callable CLIs** — probe for installed and authenticated CLIs that return work-bearing items. Seed list (extend as needed): `gh` (GitHub — `gh pr list`, `gh search issues`), `glab` (GitLab), `linear` (Linear CLI). Use `command -v <name>` to detect installation; for each that exists, the CLI is a queryable channel via `Bash`.

**Explicitly excluded from the channel set:** memory and infrastructure MCPs (`fabric`, `codebase-memory-mcp`) — they are `/inbox`'s own substrate, not sources of work. Fabric stores the profile and learned facts; it is queried on every invocation regardless of channel selection. Do not offer it in the wizard's channel list, even though `mcp__plugin_anthara_anthara-fabric__search_memories` is technically a search tool.

The detection criterion is "returns *work-bearing items*", not "has a search tool." When in doubt about a borderline MCP (a generic "search" MCP, a calendar MCP without assignments, a wiki MCP without comments-needing-input), surface it in the wizard with a one-line description and let the user decide.

Skip silently when a channel has no tools loaded and no CLI on `PATH` — that channel is "not connected" for this invocation; the table is built from whatever channels did connect.

## Step 2: Resolve the profile

Compute the project slug — `<owner>-<repo>` from `git remote get-url origin` (e.g., `anthara-ai-plugin`).

Query fabric for the profile:

```
mcp__plugin_anthara_anthara-fabric__search_memories(query="[INBOX-PROFILE <project-slug>", scope="private", limit=1)
```

- If a profile entry exists with `status=active`, parse the role / tools / time zone / usernames / manager from it and proceed to Step 4.
- If no profile entry exists, proceed to Step 3 (first-run wizard).
- If fabric is unreachable, run with a session-only in-memory profile assembled from a minimal one-pass elicitation — single `AskUserQuestion` for role only, no persisted write. Surface this honestly to the user: *"Fabric is unreachable; running with a session-only profile. The full wizard will run when fabric reconnects."*

## Step 3: First-run wizard (profile-absent)

Elicit the profile via `AskUserQuestion` calls. Recommended question set, in order:

1. **Role** — single-select, from the fixed ontology: Junior IC / Senior IC / Staff or Principal / Tech Lead / Engineering Manager / Product Manager / Founder or solo / Other (free-text role description).
2. **Primary tools / channels** — multi-select from detected channels. The question text must always read: *"Which work channels should /inbox pull from? (loaded MCPs and installed CLIs shown; type more tool names in the 'Other' option)"* — both surfaces named explicitly so the user understands what's being offered. Options list every detected MCP and every detected CLI side-by-side with a one-line description, distinguishing the access path (e.g., *"GitHub (via `gh` CLI) — PRs awaiting your review, your open PRs"* vs. *"Jira (Atlassian MCP) — tickets assigned to you, mentions, sprint state"*). The "Other" / "Type something" option must invite the user to type additional tool names not auto-detected (e.g., *"linear, height, asana, plus any other tool name — /inbox will probe for an MCP or CLI"*). Only the channels the user picks will be queried.
3. **Time zone** — single-select from common zones plus "Other" (free-text). Used by the model for "age" interpretation in the "Why now" column.
4. **Usernames** — one open text question: *"What usernames / handles do you go by across your work tools? (e.g., GitHub: sapan, Slack: @sapan, Jira: sapan.parikh)"*. Drives `@-mention` detection.
5. **Manager's name** — open text question: *"Who's your manager (if any)? Used for 'who should I be unblocking' reasoning."*. Optional; can skip with "none".

Persist to fabric:

```
mcp__plugin_anthara_anthara-fabric__add_private_memory(data="[INBOX-PROFILE <project-slug> role=<X> tools=[<Y>,<Z>] tz=<TZ> usernames={github=<X>,slack=<Y>,jira=<Z>} manager=<name>]")
```

The wizard is not strictly three questions — the brief explicitly opted for a richer onboarding to build a useful profile for the model. Skip any field the user declines without re-asking.

After the wizard writes, fall through to Step 4 and produce the table immediately using the just-saved profile.

## Step 4: Parse the optional hint

If the slash-command argument is non-empty, it is the user's natural-language hint. Examples:

- `/inbox on-call today`
- `/inbox no meetings today`
- `/inbox just got back from PTO`
- `/inbox 30 min before standup`
- `/inbox interview at 2pm — only 15-minute tasks`
- `/inbox blocked, need something to chew on`

The hint is ephemeral — never persisted. Pass it verbatim to the ranking step (Step 7) as a ranking-policy modifier.

## Step 5: Query channels in parallel

For each channel in the user's profile, issue the channel's role-scoped queries in parallel via one assistant turn. Use the channel's native access path: **MCP tool calls** for channels detected as MCPs, **`Bash` invocations** for channels detected as CLIs (e.g., `gh pr list --search "review-requested:@me"`, `gh search issues "assignee:@me state:open"`). Both paths fire in the same assistant turn — Claude Code's parallel-tool-call primitive parallelizes MCP calls and Bash calls together. Role drives the scope:

- **Junior IC** — personal feed only. Per channel: assigned-to-me, my-open-PRs, @-mentions in last 24h.
- **Senior IC** — personal + team. Adds: PRs awaiting my review, team's blocked tickets.
- **Staff / Principal** — personal + team + cross-org. Adds: RFCs in domains I own, stale PRs anywhere in my repos, mentions of areas I own.
- **Tech Lead** — personal + team blockers + sprint anomalies. Adds: stale PRs across the team, tickets in review > 24h.
- **Engineering Manager** — + reports' blockers + 1:1 prep candidates + escalations.
- **Product Manager** — + stakeholder threads + decisions awaiting input + customer messages.
- **Founder / solo** — wider, impact-weighted mixed bag. Adds: customer-facing items, top-priority tickets across all projects.
- **Other** — base personal scope + the free-text role-description hint applied as a query bias.

If a channel returns a transient rate-limit / throttle (e.g. HTTP 429, or a `Retry-After` response), retry it once after the indicated (or a brief) delay before giving up — throttling is recoverable, and dropping the channel silently loses real work. If a channel times out or returns a hard error (or is still throttled after that one retry), skip it for this run and note it as a footnote below the table, making clear its items are absent (e.g., *"Slack: rate-limited, skipping for this run — its items are not in this table"*). Do not abort the whole invocation; render the table from whatever channels returned.

## Step 6: Normalize and enrich

Convert each raw channel response into the canonical `Work item` shape:

```
{
  kind: "pr-review" | "ticket" | "direct-ask" | "thread" | "decision" | "blocker" | "incident" | "email" | "stale-own-pr" | "fyi" | ...,
  source: "github" | "jira" | "slack" | "linear" | "notion" | "gmail" | "teams",
  id: "<channel-native-id>",
  title: "<short title>",
  opener: "<username>",
  opener_role: "<inferred from profile, if applicable>",
  age_hours: <int>,
  last_activity: "<human-readable>",
  linked_items: ["<id-of-coupled-item>", ...],
  signals: { ... any free-form features that may matter ... },
  link: "<URL>",
  your_relationship: "assigned" | "reviewer" | "mentioned" | "blocking-others" | "watcher" | ...,
  hint_relevant: <bool>
}
```

Apply cross-source coupling:

- **String-match on IDs** — if a PR title or description contains a Jira ticket ID (e.g., `INC-198`), couple them.
- **URL extraction** — if a Slack message body or Notion comment contains a PR or ticket URL, couple to that item.
- **Fuzzy linking** — same author opening a PR and a Slack thread within 60 minutes is a coupling candidate.

Bias is **under-couple over over-couple**. When uncertain, leave them as separate rows. The model can disambiguate later in the "Why now" column with phrasing like *"possibly same as #5"*.

Every item must carry its canonical URL in the `link` field whenever the channel exposes one. For coupled items, also retain each source's URL alongside its source identifier (e.g., `{source: "github", id: "anthara-ai/web#98", link: "https://github.com/..."}` and `{source: "jira", id: "ANTHARA-76", link: "https://...atlassian.net/..."}` — both preserved for the render step). Items without a URL (rare — typically only synthetic FYI rows) are rendered as plain text in Step 8.

## Step 7: Rank via the model

Retrieve any `[INBOX-LEARNED ...]` entries for this project from fabric:

```
mcp__plugin_anthara_anthara-fabric__search_memories(query="[INBOX-LEARNED <project-slug>", scope="private", limit=50)
```

Hand the model: the profile, the optional hint, the learned facts, and the enriched work-item set. Ask for a ranked table of up to twenty rows.

The model's job:

1. Reason over all available features (role, hint, learned facts, age, blocking-count, deadline, opener, last-activity, linked-items, your-relationship).
2. Decide what makes the top of the table given the role and the hint. Heuristics are allowed but not required; the model is given good data and chooses how to use it.
3. Write a terse "Why now" for each row — most important signal in the first few words. Free-form (not template). Examples:
   - `"blocks Fri demo — Priya waiting 31h"`
   - `"@sarah waiting on your design-review reply — 1h ago"`
   - `"on-call: only incident open; investigate"`
   - `"stale 4d — you're tagged for input"`
4. Render the source identifier in the Item column as a markdown link to the item's URL. For coupled items, link each source identifier independently — `[anthara-ai/web #98](pr-url) ↔ [ANTHARA-76](jira-url)`. The descriptive tail after the identifier (`— Dinesh — fix(...)`) stays plain text. Cross-references to *other rows* of the same table (`pairs with #4`) stay plain text (the host conversation resolves row numbers). Cross-references to *external items by ID* (`blocks ANTHARA-77`) may be linked if that item's URL is known from the enriched set, but never invent URLs.
5. Cap output at 20 rows. Aim lower (10-15) when the corpus is small or the role/hint narrows scope; never exceed 20. Items that didn't make the table are dropped — not paginated, not stashed. Re-running with a different hint will surface them if relevant then.

## Step 8: Render the table

Output one markdown table — plain markdown, no ANSI colors, no Unicode box-drawing beyond standard markdown syntax. Every source identifier in the Item column is a markdown link to the canonical URL whenever a URL exists. Coupled rows link each source identifier independently. The descriptive tail (text after the identifier) stays plain. Channel names in the footer line are plain text (they're labels, not items):

```
| # | Kind        | Item                                                                              | Why now                                       |
|---|-------------|-----------------------------------------------------------------------------------|-----------------------------------------------|
| 1 | PR review   | [anthara-ai/web #142](https://github.com/anthara-ai/web/pull/142) — @priya — fix dispatch retry | blocks Fri demo — 31h waiting                  |
| 2 | PR + ticket | [anthara-ai/web #98](https://github.com/anthara-ai/web/pull/98) ↔ [ANTHARA-76](https://anthara.atlassian.net/browse/ANTHARA-76) — hide Assess button while scan | PR open 4d, you reported the bug; close-out near |
| 3 | Direct ask  | [@sarah in #plat](https://anthara.slack.com/archives/CXYZ/p123) — "design review?"  | 1h waiting on you                              |
| 4 | Decision    | [RFC-018 (vector store)](https://notion.so/...rfc-018) — your input asked          | stale 4d                                       |
| 5 | Your work   | [INC-198 slice 5.3](https://anthara.atlassian.net/browse/INC-198) — untouched      | stale 2d, your active spec                     |
...
```

If a row has no URL (e.g., a synthetic FYI roll-up, or a CLI query that didn't expose a link), render the identifier as plain text — never invent a URL.

Below the table, on a single muted line, summarize channels and any skipped/failed ones:

```
Channels: github · jira · slack · linear  ·  skipped: gmail (timeout)
```

That is the entire output. No "Here's your inbox:" preamble. No "Anything else?" trailer. The table speaks for itself.

## Step 9: Handle row-action follow-ups

After rendering, the user may say things like:

- *"let's work on #3"*
- *"ignore #5"*
- *"what's the context on #7?"*
- *"#2 is actually waiting on me, not Priya"*
- *"remember that ACME is our biggest client — bump #4"*

These resolve against the most recent rendered table (which lives in the Claude Code session's conversation context). No dedicated commands. Apply the curiosity-ask + learning loop:

1. When the user picks a row (*"let's work on #3"*), decide whether the choice is **obvious** (e.g., #3 is the only P0; the on-call hint is active and #3 is the incident) or **non-obvious**.
2. If non-obvious, ask one short open question: *"I'm curious, why this one?"* No `AskUserQuestion` — this is a follow-up open question.
3. If the user's answer contains a fact worth retaining — explicit teaching like *"because ACME is our biggest client"* or *"remember that Sarah is the staff eng on the platform team"* — write to fabric:

   ```
   mcp__plugin_anthara_anthara-fabric__add_private_memory(data="[INBOX-LEARNED <project-slug> fact='<verbatim or paraphrased fact>' learned-from='row #3 selection']")
   ```

4. If the user's answer is a one-off rationale that doesn't generalize ("I'm just in the mood for that one"), do **not** write to memory. Behavioral signals alone do not produce learned facts.

The bar for writing is "the model judges the user's framing implies persistence." Literal phrases like *remember*, *note that*, *for future* are strong signals. A clearly transient phrase like *"just for today"* or *"in this moment"* is a strong negative signal.

## Step 10: Subsequent invocations within the same session

If `/inbox` is re-invoked in the same session, repeat from Step 4 (parse hint) — the profile and learned facts are already loaded from this session's earlier turn. Re-query the channels fresh; do not re-use the prior table's items. The rendered table replaces the prior one as the addressable state.

## When the channels return nothing

If all channels return zero items for the role's scope (rare but possible — fresh repo, no work assigned yet), render a one-line note instead of an empty table:

```
No open work in scope. Channels queried: github, jira, slack. Try a wider role scope or check back later.
```

Do not invent items. Do not pad with FYI noise.

## Use this when / not when

Use this skill when the user runs `/anthara:inbox` or says any of: *"what should I work on next"*, *"show me my inbox"*, *"what's on my plate"*, *"prioritize my work"*, *"/inbox on-call"*, *"/inbox no meetings today"*.

- **Not for backward-looking standup posts.** That's a different command (e.g., `/standup`); `/inbox` is forward-looking.
- **Not for creating new work.** Use `/anthara:create-ticket` to file new tickets.
- **Not for executing on a chosen item.** When the user says "let's work on #3", the model proceeds with the actual work using whatever skills apply (`/anthara:debug-fix` for an incident, `/anthara:plan-implementer` for a spec slice, etc.). `/inbox` only produces the table and the curiosity-ask follow-up; the host conversation drives execution.
- **Not for first-time exploration of an idea.** That's `/anthara:brainstorm`.

## What NOT to do

- Do not output prose before the table. The table is the output.
- Do not write to fabric memory on behavioral signals alone. Only explicit teaching persists.
- Do not couple two items unless evidence is strong. Under-couple when uncertain.
- Do not invent items to fill the table. Ten is a soft cap, not a target.
- Do not retry timed-out channels in the same invocation. Skip them, footnote them, move on.
- Do not show ANSI color, emojis, or Unicode box-drawing in the rendered table.
- Do not maintain row-state between invocations (no "dismissed" / "snoozed" lists). Conversation context is the only state.
- Do not paginate the table. Items past the 20-row cap are dropped; re-run with a different hint if they matter.
- Do not commit to a latency SLA. Run channels in parallel and accept the resulting timing.
- Do not encode per-kind staleness windows. Staleness is one feature the model reasons over, never the primary dimension.
- Do not require a specific MCP. Whatever the user has connected drives the channel set; the skill works with a single channel as well as with seven.
