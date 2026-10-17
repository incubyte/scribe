---
name: team-update
description: Backward-looking, channel-agnostic team catch-up. Pulls every connected work channel (GitHub, Jira, Slack, Linear, Notion, Gmail, Microsoft 365 / Teams) via MCP over a time window (default 18 hours, overridable by argument), auto-infers who your team is from collaboration signals (PR authors/reviewers you interact with, people in your message and email threads, shared-ticket assignees), and produces a scannable digest grouped by teammate with each person's items tagged by workstream and a "needs you" callout. Use when the user runs /anthara:team-update, says "what did my team do", "catch me up", "what happened while I was away / out / on PTO", "team update", "what did I miss", "/team-update 3d", "/team-update since monday", or wants a backward-looking summary of teammates' activity across all connected work channels.
argument-hint: [time window + optional focus, e.g. "18h" (default), "3d", "since monday", "2d backend only"]
allowed-tools: Read, Bash, AskUserQuestion, ToolSearch, Skill
---

# /anthara:team-update

A scannable catch-up on what your team did while you were away, drawn from every connected MCP channel over a time window. Auto-infers your team from who you actually collaborate with — you never maintain a roster. Grouped by teammate, each item tagged by workstream, with the things waiting on you pulled to the top. The backward-looking counterpart to `/anthara:inbox`: inbox answers "what should I do next"; team-update answers "what did everyone else just do".

## Operating principles

- **Backward-looking and team-centric.** `/inbox` is forward-looking and me-centric; `team-update` reports what *other people* did in a past window. Your own activity is incidental context, not the subject.
- **The team is inferred every run, never stored.** Derive collaborators fresh from the signals in (or near) the window — do not ask the user to maintain a roster, and do not persist one. The team adapts as collaboration shifts.
- **Channel-pluggability is the architectural commitment.** Every connected MCP (and Bash-callable CLI) that returns activity-bearing items is a channel. Adding a channel is detection, not code.
- **The window is the primary control.** Default is the last 18 hours. The argument overrides it ("3d", "since monday", "last week"). Infer the collaboration graph from a wider lookback than the report window (up to ~21 days; see Step 5), but only report activity inside the window.
- **Grouped by teammate, tagged by workstream.** One section per person. Within a section, each item carries a short `[workstream]` tag (project / feature / thread) so the reader sees both *who* and *on what*.
- **Surface what needs the reader.** Items that wait on the user — review requests on their PRs, direct @-mentions and asks, decisions where their input was requested — get an explicit "Needs you" callout above the per-teammate sections. This is the one place team-update points back at the user.
- **Cross-source coupling — bias under-couple over over-couple.** A teammate's PR linked to its ticket linked to its Slack thread is one bullet, not three. When uncertain whether two items are the same work, keep them separate. False positives erode trust faster than false negatives.
- **Read-only. No writes, no memory persistence.** team-update never files tickets, posts messages, or writes to fabric. It reuses the inbox profile read-only when present; it never creates or edits one.
- **Tool discipline.** Closed questions go through `AskUserQuestion`. At most one open question per turn — and most runs ask none.
- **Privacy lives at the MCP token, not here.** Read everything the connected MCPs allow. Token scoping is the user's responsibility.
- **No SLA.** Run channels in parallel and accept the resulting timing.

## Step 1: Load deferred tools and detect connected channels

Load `AskUserQuestion` and the fabric search tool via `ToolSearch`. The fabric tool is loaded here **only** for the Step 2 inbox-profile read (read-only) — it is not a channel and is never written to:

```
ToolSearch(query="select:AskUserQuestion,mcp__plugin_anthara_anthara-fabric__search_memories")
```

A **channel** is any source that can be queried for *activity-bearing items* over a time range — merged/opened PRs, PR reviews and comments, ticket transitions and comments, messages and threads, emails, decision-doc edits and comments. Detect channels from **two surfaces**:

- **Connected MCPs** — inspect the available tool list and select MCPs whose tools return activity-bearing items (Atlassian / Jira / Confluence, GitHub, Slack, Linear, Notion, Gmail, Microsoft 365 / Outlook / Teams, and any other MCP whose tools include search / list / read of PRs, commits, messages, threads, issues, or doc activity). For each detected channel MCP, load its query tools via `ToolSearch`.
- **Bash-callable CLIs** — probe for installed and authenticated CLIs that return activity-bearing items. Seed list (extend as needed): `gh` (GitHub — `gh pr list`, `gh search prs`, `gh api`), `glab` (GitLab), `linear` (Linear CLI), `git` (always present — local commit log). Use `command -v <name>` to detect installation; for each that exists, the CLI is a queryable channel via `Bash`.

**Explicitly excluded from the channel set:** memory and infrastructure MCPs (`fabric`, `codebase-memory-mcp`) — they are team-update's own substrate, not sources of team activity. Fabric is queried once (Step 2) only to reuse the inbox profile.

Skip silently when a channel has no tools loaded and no CLI on `PATH` — that channel is "not connected" for this run; the digest is built from whatever channels did connect.

## Step 2: Resolve "you" (identity), reusing the inbox profile

To find *your* collaborators, the model must know who you are across channels. Resolve identity in this order:

1. Compute the project slug — `<owner>-<repo>` from `git remote get-url origin` (e.g., `anthara-ai-plugin`).
2. Query fabric once for the inbox profile (read-only — never write):

   ```
   mcp__plugin_anthara_anthara-fabric__search_memories(query="[INBOX-PROFILE <project-slug>", scope="private", limit=1)
   ```

   If found, reuse its `usernames` (github / slack / jira handles), `manager`, and `role` to sharpen identity and team inference. **Never create or edit the profile from here** — that is `/inbox`'s job.
3. If no profile (or fabric unreachable), derive identity from local signals without asking:
   - `git config user.email` and `git config user.name`
   - `gh api user --jq .login` (if `gh` is authenticated) for the GitHub handle
   - The Slack / Jira / Gmail MCPs' own "me" lookups when available.
4. Only if identity cannot be resolved from any of the above, ask **one** `AskUserQuestion`: the user's primary handle(s). Do not run a full inbox-style wizard — team-update needs only enough identity to detect collaborators.

If identity is partial (e.g., GitHub handle known, Slack handle unknown), proceed with what's known and note the gap as a footnote rather than aborting.

## Step 3: Parse the time window

The slash-command argument carries the window and an optional focus hint.

- **Empty argument** → default window is the **last 18 hours**.
- **Duration forms** → `18h`, `36h`, `3d`, `2 weeks`, `1w`. Convert to an absolute start timestamp using the current time.
- **Anchored forms** → `since monday`, `since friday 5pm`, `since I left`, `over the weekend`. Resolve to a concrete start timestamp; when ambiguous (`since I left` with no profile signal), default to 18h and note the assumption in the footer.
- **Focus hint** → any non-temporal remainder is a focus filter, e.g. `2d backend only`, `last week — just the payments work`, `18h ignore CI noise`. Pass it verbatim to the compose step (Step 7) as a scope modifier.

Compute and remember two timestamps for downstream steps:
- `report_start` — the start of the window the user asked about (what activity to report).
- `graph_start` — a wider lookback for inferring the collaboration graph (Step 5): `max(report window, ~21 days)`. This keeps team detection stable even for a short report window.

State the resolved window briefly only if it was non-obvious (e.g., an anchored or defaulted form); otherwise let the footer carry it.

## Step 4: Query channels in parallel over the window

For each detected channel, issue activity queries scoped to the window, in one assistant turn so MCP calls and `Bash` calls parallelize together. Query the team's activity, not just yours — pull a superset and attribute to people in Step 6. Representative queries per channel:

- **GitHub** (`gh` or MCP) — PRs merged / opened / updated in the window (`gh search prs "org:<org> merged:>=<report_start>"`, `gh pr list --search "updated:>=<report_start>"`); reviews and review comments on your PRs; commits on active branches (`git log --since=<report_start> --all`).
- **Jira / Linear** — issues transitioned, commented, or assigned in the window; status changes on issues in your projects / sprints; comments on issues you reported or are watching.
- **Slack / Teams** — messages and threads in the channels and DMs you participate in; @-mentions of you; threads you started or replied to. Bias to channels you are active in, not the whole workspace.
- **Gmail / Outlook** — emails in threads that include you, sent in the window.
- **Notion / Confluence** — pages and decision docs created or edited in the window in spaces you touch; comments requesting input.

Use the inbox profile's role to widen scope when present: an Engineering Manager's "team" weights direct reports; a Tech Lead's weights the repos/sprint they lead. Absent a role, use the personal collaboration graph.

If a channel times out or errors, skip it for this run and note it as a footnote below the digest (e.g., *"Slack: query timed out, skipping for this run"*). Never abort the whole run; build the digest from whatever channels returned.

## Step 5: Infer the team from collaboration signals

From the queried data (and a wider `graph_start` lookback where a channel makes that cheap), derive the set of teammates — the people you actually collaborate with. Signals, strongest first:

- **Code review reciprocity** — people who reviewed your PRs, or whose PRs you reviewed/commented on.
- **Co-authorship & shared files** — co-authors on commits; people who recently touched the same files/areas you own.
- **Conversation reciprocity** — people in Slack/Teams threads you participate in; @-mention exchanges (either direction); recurring DM partners.
- **Email reciprocity** — people on email threads with you in the window.
- **Shared work items** — assignees/commenters on tickets linked to yours, in your sprint, or that you reported/watch.
- **Org structure** — if the inbox profile names a manager or implies reports, weight those relationships up.

Rank candidates by signal strength and recency; keep the strongly-connected core (typically 3–8 people). Drop one-off contacts (a single bot comment, a no-reply notification address). Bots, CI accounts, and no-reply senders are excluded.

If **no** collaborators can be inferred (solo repo, quiet window), say so plainly in Step 8 rather than inventing a team.

## Step 6: Normalize, attribute, and tag by workstream

Convert each raw channel response into the canonical activity-item shape and attribute it to a teammate:

```
{
  actor: "<teammate handle/name>",
  kind: "pr-merged" | "pr-opened" | "pr-review" | "commit" | "ticket-moved" | "ticket-comment" | "thread" | "direct-ask" | "decision" | "doc-edit" | "email" | ...,
  source: "github" | "jira" | "slack" | "linear" | "notion" | "gmail" | "teams",
  id: "<channel-native-id>",
  title: "<short title>",
  workstream: "<inferred project / feature / thread label>",
  when: "<timestamp within window>",
  linked_items: ["<id-of-coupled-item>", ...],
  needs_you: <bool>,            // waits on the reader: review request, @-mention/ask, decision input requested
  link: "<URL>",
  signals: { ... any free-form features ... }
}
```

**Attribution** — map each item to the teammate who performed it (PR author, commenter, message sender, ticket transitioner). Items by the user themselves are dropped from the per-teammate sections (this is a team update), except where they establish a coupling.

**Workstream tagging** — infer a short workstream/topic label per item from repo + path, ticket epic/project, PR title prefix (`feat(dispatch):` → `dispatch`), channel name, or doc title. Reuse consistent labels across people so the same initiative reads the same in every section.

**Cross-source coupling** — bias **under-couple over over-couple**:
- **ID match** — a PR/branch/commit referencing a ticket ID (`ANTHARA-76`) couples to it.
- **URL extraction** — a Slack/Notion body containing a PR or ticket URL couples to that item.
- **Fuzzy linking** — the same actor opening a PR and a thread within ~60 minutes is a coupling *candidate*; when unsure, keep separate and note "possibly same as …".

**needs_you** — set true for: review requested on a PR you authored; a direct @-mention or question addressed to you; a decision/RFC where your input was explicitly requested. These drive the "Needs you" callout.

Every item carries its canonical URL in `link` whenever the channel exposes one; for coupled items, retain each source's URL alongside its identifier.

## Step 7: Compose the digest

Hand the model: the resolved identity, the window, the optional focus hint, the inferred team, and the enriched + attributed item set. Ask for a scannable digest:

1. Apply the focus hint as a scope filter (`backend only`, `ignore CI noise`) — drop or downweight out-of-scope items.
2. Compute a **TL;DR** — one line summarizing the window: counts that matter (PRs merged, tickets closed, decisions made) and the single most important "needs you" if any. Example: *"Last 18h: 4 PRs merged, 2 tickets closed, 1 decision waiting on you."*
3. Build the **Needs you** block — the `needs_you` items, terse, most-waiting first. Omit the block entirely if empty (don't print "Needs you: none").
4. Build **per-teammate sections** — ordered by volume/significance of activity (most active first). Within each person, list items as short bullets, each prefixed with a `[workstream]` tag and a status glyph where it adds signal. The full sanctioned glyph set is `✅` (merged / closed / done), `⚠` (waiting / blocked / needs you), `→` (in progress / opened), `💬` (message / thread / email). Group a person's bullets by workstream when they have several. Keep each bullet to one line; link the source identifier.
5. Cap the digest at the ~8 most-connected teammates and the most significant items per person (soft cap ~6 bullets/person). Note dropped volume in the footer (e.g., *"+3 quieter contributors omitted"*) rather than padding.

## Step 8: Render the digest

Output clean markdown — no ANSI color, no Unicode box-drawing, minimal emoji (only the sanctioned status glyphs `✅ ⚠ → 💬`). Source identifiers are markdown links to canonical URLs whenever a URL exists; coupled items link each identifier independently. No "Here's your team update:" preamble.

```
**TL;DR — last 18h:** 4 PRs merged, 2 tickets closed, 1 decision waiting on you.

**Needs you**
- ⚠ [web#150](https://github.com/anthara-ai/web/pull/150) — @priya requested your review (2h ago) `[dispatch]`
- ⚠ [RFC-018](https://notion.so/...) — @dinesh asked for your input on the vector store `[search]`

### @priya — 3 items
- ✅ `[dispatch]` Merged [web#142](https://github.com/anthara-ai/web/pull/142) — retry fix
- → `[dispatch]` Opened [web#150](https://github.com/anthara-ai/web/pull/150) — needs your review
- 💬 `[#platform]` Asked about the dispatch backoff window

### @dinesh — 2 items
- ✅ `[search]` Closed [ANTHARA-76](https://anthara.atlassian.net/browse/ANTHARA-76) ↔ [web#98](https://github.com/anthara-ai/web/pull/98)
- → `[search]` Opened [RFC-018](https://notion.so/...) — vector store direction

### @sarah — 1 item
- 💬 `[launch]` Emailed re: launch checklist timing
```

Below the digest, on a single muted line, summarize the window, channels queried, and any skipped/failed ones plus omitted volume:

```
Window: last 18h (since 2026-06-15 23:00) · Channels: github · jira · slack · notion · skipped: gmail (timeout) · +3 quieter contributors omitted
```

That is the entire output. No trailer, no "Anything else?".

## Step 9: Handle follow-ups

After rendering, the user may say things like:

- *"open priya's PR #150"* / *"let's review #150"*
- *"what's the full thread on the dispatch backoff question?"*
- *"widen to 3 days"* / *"just the search workstream"*
- *"who else worked on dispatch?"*

These resolve against the most recent rendered digest (which lives in the session's conversation context). "Widen to 3 days" or a new focus re-runs from Step 3 with the adjusted window/hint. "Let's review #150" hands off to the actual work (e.g., `/anthara:code-review` or `/anthara:plan-implementer`) — team-update produces the catch-up; the host conversation drives execution. team-update writes nothing to memory on any follow-up.

## Step 10: Re-invocation within the same session

If `/anthara:team-update` is re-invoked in the same session, repeat from Step 3 (parse window) — identity and channels are already resolved from this session's earlier turn. Re-query channels fresh for the new window; do not reuse the prior digest's items. The new digest replaces the prior one as the addressable state.

## When the channels return nothing

If all channels return zero team activity for the window (quiet period, solo repo, everyone off), render a one-line note instead of an empty digest:

```
No team activity in the last 18h across github, jira, slack. Quiet window — try a wider range like /team-update 3d.
```

Do not invent activity. Do not pad with your own items.

## Use this when / not when

Use this skill when the user runs `/anthara:team-update` or says any of: *"what did my team do"*, *"catch me up"*, *"what did I miss"*, *"what happened while I was out / away / on PTO"*, *"team update"*, *"/team-update 3d"*, *"/team-update since monday"*.

- **Not for "what should I work on next."** That's forward-looking — use `/anthara:inbox`.
- **Not for creating work.** Use `/anthara:create-ticket`.
- **Not for executing on an item.** When the user says "let's review #150", hand off to the relevant skill (`/anthara:code-review`, `/anthara:plan-implementer`, `/anthara:debug-fix`).
- **Not for first-time exploration of an idea.** That's `/anthara:brainstorm`.

## What NOT to do

- Do not maintain or persist a team roster. Infer the team every run; write nothing to fabric.
- Do not create or edit the inbox profile — reuse it read-only when present.
- Do not output prose before the TL;DR. The digest is the output.
- Do not report the user's own activity in the per-teammate sections (it's a *team* update).
- Do not couple two items unless evidence is strong. Under-couple when uncertain.
- Do not invent teammates or activity to fill the digest.
- Do not retry timed-out channels in the same run. Skip, footnote, move on.
- Do not show ANSI color or Unicode box-drawing; keep emoji to the sanctioned status glyphs (`✅ ⚠ → 💬`) only.
- Do not require a specific MCP. Whatever the user has connected drives the channel set; it works with a single channel as well as with seven.
- Do not commit to a latency SLA. Run channels in parallel and accept the timing.
