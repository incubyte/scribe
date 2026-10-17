<!-- Generated from the Anthara workflow authoring guide. Do not edit; regenerate from the source guide. Source sha256: 3a5e7531a77c197d -->

# Writing Anthara Workflows

Authoring guide for writing workflow YAML files. It documents the full YAML
schema the engine accepts, how a workflow executes end-to-end, and the prompt-writing
conventions every existing workflow follows.

---

## 1. What a workflow is

A workflow is a YAML file describing an automated, often AI-driven pipeline: *fetch
work items from Jira/Azure DevOps → run a Claude agent over each one → talk to humans
in Microsoft Teams → write results back (comments, work items, PRs)*.

A workflow YAML file is *definition storage*. The workflow becomes live by POSTing its
raw YAML to the api:

```
POST /workflow-engine/workflows          { "yaml": "<file contents>" }
PUT  /workflow-engine/workflows/:id      (update — same validation)
POST /workflow-engine/workflows/:id/runs { "inputs": { "meetingDate": "yesterday" } }
```

Validation happens at create/update time. Connection names are additionally
checked against existing AuthConfig records — a YAML referencing a connection that
isn't configured is rejected.

## 2. Minimal example

```yaml
name: My First Workflow
slug: my-first-workflow
trigger:
  type: manual
  config:
    inputs:
      - name: ticketKey
        type: string
llm_config:
  model: claude-sonnet-5
  max_turns: 80
  max_budget_usd: 15
  effort: high

steps:
  - name: fetch-issues
    type: read_data
    provider: jira
    connection: Jira
    config:
      query: >
        project = DEMO AND status = "To Do" ORDER BY priority DESC

  - name: analyze
    type: ai_task
    connections: [Jira, GitHub]
    config:
      interactive: true
      teams_channel_id: '19:...@thread.v2'
      teams_service_url: 'https://smba.trafficmanager.net/teams/'
      prompt: |
        You receive ONE Jira story as JSON from the upstream fan-out:
        {{steps.fetch-issues.output}}
        ...
```

## 3. Top-level schema

| Key | Required | Notes |
|-----|----------|-------|
| `name` | yes | Display name (string). |
| `slug` | yes | Machine identity (string), unique per install, at most 255 characters. Lowercase letters and digits joined by single hyphens — `^[a-z0-9]+(?:-[a-z0-9]+)*$`, e.g. `ci-fix`. It is the agent pod's git commit identity, `<slug>@<domain>` (domain from `GIT_IDENTITY_DOMAIN`, default `anthara.ai`), so every commit a workflow makes is attributable to it — see the note below. Choose it by hand and keep it stable — rewriting it splits one bot's history across two authors. Unlike `name`, it is not a display label. |
| `trigger` | yes | `{ type, config }` — see §4. |
| `steps` | yes | Non-empty ordered array — see §5. |
| `llm_config` | no | Agent defaults for `ai_task`/`coding_task` steps — see §6. |
| `repo` | conditional | **Required** for `github` and `azure_devops` triggers. GitHub shape: `owner/name` or `owner/*` (org-wide wildcard). Azure DevOps shape: `org/project/repo` or `org/project/*` (project-level wildcard). Without it the workflow silently matches zero webhook deliveries, so the validator enforces it. |

> **The slug is the pod's commit identity.** Every `ai_task` and `coding_task`
> pod the workflow dispatches commits with author and committer email
> `<slug>@<domain>` and name `Anthara <SLUG>` (the slug uppercased), e.g.
> `ci-fix@anthara.ai` / `Anthara CI-FIX`. A step resumed from a Teams reply
> commits under the same identity as its first leg. A workflow row with no
> slug — only possible for rows written straight into the database, since the
> validator requires one — falls back to `ai@<domain>`.

## 4. Triggers

Valid `trigger.type` values (`VALID_TRIGGER_TYPES` in `yaml-validator.ts`):

### `manual`
Triggered via `POST /workflow-engine/workflows/:id/runs`. Optional
`config.inputs: [{ name, type }]` declares the inputs the caller passes; each becomes
available as `{{inputs.<name>}}`.

### `cron`
`config.schedule` (required, standard cron expression — validated with `cron-parser`)
and `config.tz` (optional IANA timezone, e.g. `Asia/Kolkata`; defaults to system TZ).
Cron runs execute as **system actor** (no user). At most one run is *actively
executing* per workflow; extra ticks queue as pending runs and drain as soon as the
current run finishes **or parks in the Teams collaboration loop**
(`in_collaboration_loop`). A run hibernating on an unanswered card does not block
the schedule — the next tick starts a fresh run alongside it. Fan-out follows the
same rule: a parent in `awaiting_children` blocks only while at least one child is
still executing (or its children are all done and it is about to resume); once
every remaining child is parked on a card, the parent counts as parked too and
the schedule keeps firing.

### `github`
Requires top-level `repo` (`owner/name` or `owner/*`) plus `config.events` (non-empty)
and `config.branches` (non-empty glob array). Valid events (`VALID_GITHUB_EVENTS`):
- `pull_request.opened`
- `pull_request.synchronize` — supersedes an in-flight run for the same PR
- `workflow_run.completed` — dispatched only when the CI conclusion is a failure
- `issue_comment.created` — a NEW top-level PR conversation comment (comments on
  plain issues are filtered out at the edge)
- `pull_request_review_comment.created` — a NEW inline review comment

**Comment events (the @anthara mention triggers)** dispatch only when the comment
body contains the mention tag — `@anthara`, a fixed constant in the ingress code,
matched as a case-insensitive substring. The check runs at the ingress edge BEFORE any run or
pod starts, so a comment without the tag costs nothing (it is ledgered `ignored`
with `comment_no_mention`, like non-failed builds). Comment semantics differ from
PR events in three ways:
- **Per-mention runs, no supersede** — every mentioning comment starts its own
  run; a second mention never cancels an in-flight one, and a
  `pull_request.synchronize` never cancels a comment run.
- **`config.branches` is ignored** for comment events (`issue_comment` payloads
  carry no base branch) — write `branches: ["*"]`.
- **Different inputs**: `{{inputs.repo}}`, `{{inputs.prNumber}}`, `{{inputs.prUrl}}`,
  `{{inputs.sender}}` (the comment AUTHOR), `{{inputs.commentId}}`,
  `{{inputs.commentText}}`, `{{inputs.installationId}}` — and NO
  `headSha`/`baseBranch`; the prompt fetches PR metadata itself.
  `{{inputs.commentText}}` is untrusted human text (truncated to 4000 chars at the
  edge): quote it as data in the prompt, never as instructions.

Comments authored by the App's own bot identity never dispatch (self-loop guard),
so the resolver's replies cannot re-trigger it.

GitHub PR-triggered runs receive PR context inputs: `{{inputs.repo}}`,
`{{inputs.prNumber}}`, `{{inputs.prUrl}}`, `{{inputs.headSha}}`, `{{inputs.baseBranch}}`,
`{{inputs.sender}}`, `{{inputs.installationId}}`. CI (`workflow_run.completed`) runs
receive `{{inputs.runId}}`, `{{inputs.workflowName}}`, `{{inputs.headBranch}}`,
`{{inputs.headSha}}`, `{{inputs.conclusion}}`, `{{inputs.runUrl}}`, `{{inputs.ciEvent}}`.
Credentials for system runs resolve to a minted GitHub App installation token.
The GitHub App must be subscribed to the **Issue comment** and **Pull request
review comment** webhook events (App dashboard) before comment triggers deliver.

### `azure_devops`
Requires top-level `repo` (`org/project/repo or org/project/*`) plus `config.events` and
`config.branches`. Valid events (`VALID_AZURE_DEVOPS_EVENTS`):
- `git.pullrequest.created`
- `git.pullrequest.updated`
- `build.complete` — dispatched only when the build result is `failed` (the CI
  Fixer trigger; `succeeded` / `canceled` / `partiallySucceeded` deliveries are
  filtered out at the ingress edge and ledgered as `build_not_failed`)
- `git.pullrequest.commented` — the @anthara mention trigger. This is the YAML
  name for the native service-hook event `ms.vss-code.git-pullrequest-comment-event`
  ("Pull request commented on" — a second service-hook subscription pointed at
  the same `/azure-devops/webhooks` endpoint + secret must be created in the ADO
  project). Same edge rules as the GitHub comment events: dispatched only when
  the comment body contains the mention tag (`@anthara`, case-insensitive),
  per-mention runs with no supersede in either
  direction, `config.branches` ignored (write `"*"`), and only human `text`
  comments qualify — ADO system comments (votes, policy messages) are ledgered
  as `comment_not_text`. Comment runs receive `{{inputs.repo}}`,
  `{{inputs.prNumber}}`, `{{inputs.prUrl}}`, `{{inputs.sender}}` (the comment
  author), `{{inputs.commentId}}`, `{{inputs.commentText}}` (untrusted, truncated
  to 4000 chars), `{{inputs.threadId}}` (parsed from the comment's links — may be
  an empty string, in which case enumerate the PR's threads and locate the one
  containing `commentId`), and `{{inputs.provider}}`. No `headSha`/`baseBranch`.
  ADO has NO bot-identity self-loop guard (system runs borrow the creator's
  connection, so the resolver's replies look like a normal user) — the mention
  filter is the only loop-break: a reply prompt must NEVER write the mention tag
  into a comment it posts.

PR-triggered runs receive `{{inputs.repo}}`, `{{inputs.prNumber}}`,
`{{inputs.prUrl}}`, `{{inputs.headSha}}`, `{{inputs.baseBranch}}`,
`{{inputs.sender}}`. Build (`build.complete`) runs receive the SAME input keys
as GitHub CI runs — `{{inputs.runId}}` (the failed build id),
`{{inputs.workflowName}}` (the pipeline definition name), `{{inputs.headBranch}}`
(the branch the build ran on; a PR-validation build carries the synthetic
`refs/pull/<id>/merge`, from which the prompt derives the PR),
`{{inputs.headSha}}`, `{{inputs.conclusion}}` (always `failed`),
`{{inputs.runUrl}}`, `{{inputs.ciEvent}}` (the ADO build reason, e.g.
`pullRequest` / `individualCI` / `manual`) — so a fixer prompt ports across
providers with only the API calls changing.

System runs **borrow the workflow creator's** Azure DevOps connection (the creator must
have connected the provider, else the run fails with a lender-not-connected error).

The `build.complete` service-hook subscription must be created in the ADO
project (Project settings → Service hooks → Web Hooks → "Build completed"),
pointed at the api's `/azure-devops/webhooks` endpoint with the basic-auth
secret, ideally with the subscription's own "Build status = Failed" filter —
the ingress re-checks the result either way.

### `webhook`
Accepted by the validator but currently a stub (future).

## 5. Steps

Common step fields:

| Field | Notes |
|-------|-------|
| `name` | Required. Referenced by `{{steps.<name>.output}}`. |
| `type` | Required. One of the step types below. |
| `provider` | Optional string — data/messaging provider (`jira`, `azure_devops`, ...). Inferred from the connection's AuthConfig type when omitted. |
| `connection` | Single AuthConfig name (e.g. `Jira`). |
| `connections` | Array of AuthConfig names. **Plural wins** when both are present (the singular is dropped with a debug log). Entries must be unique and each must sanitize to a valid env-var stem (see §7). |
| `config` | Type-specific object — templating is resolved recursively through every string in it. |
| `on_error` | **Not implemented — do not rely on it.** The key parses, but the engine has no consumer: every failure ends the run regardless, and `continue` behaves as `fail`. |
| `retry_count` / `retry_delay` | **Not implemented — do not rely on them.** Nothing retries a failed step. Across 1,402 step executions in the last 90 days, `attempt` was `1` every time. A failed step is a fully-paid pod with no recovery, so design the prompt to be safely re-runnable and re-trigger the workflow by hand. |
| `fan_in` | `wait_all_collect` — join point after a fan-out (see §9). |

Valid step types (`VALID_STEP_TYPES` in `yaml-validator.ts`):

### `read_data`
Fetches items from a provider. Requires `connection`. Output is a JSON array — which
usually triggers fan-out (§9).
- **`jira`** provider — `config.query` is JQL; optional `config.fields`.
- **`azure_devops`** provider — `config.organization` and `config.project` required;
  `config.query` is WIQL; optional `config.fields`.

### `ai_task`
Runs a Claude agent in a pod with **no repo checkout** (the prompt does everything via
Bash/curl or shallow clones into `/tmp`). `config.prompt` is required.

> **An `ai_task` that sets no `system_prompt` is told not to use its tools.** With
> `system_prompt` absent, the pod falls back to the `ai` entrypoint's default
> prompt: *"You are a reasoning assistant… Do not
> create files, explore directories, run commands, or attempt to build anything.
> Your entire response should be the requested output — nothing else."*
>
> If your step prompt tells the agent to clone, write files, commit, or raise a PR,
> that default contradicts it outright, and the model resolves the conflict
> differently from run to run — sometimes doing the work over ~90 turns, sometimes
> returning a one-turn analysis explaining why it could not act. The failure is
> silent: the step is marked `completed` and the output is a plausible-looking
> report for work that never happened.
>
> A step that does real work must either set `system_prompt` (§6) or be a
> `coding_task`. The tell for a run that hit this is `numTurns: 1` in
> `workflow_run_metrics` — one model response, zero tool calls.

- Dispatched to coding-service with entrypoint `ai`.
- Available tools in the pod: `Edit`, `Write`, `Bash`, `Read`, `Glob`, `Grep` — plus, in
  interactive runs, `AskUserQuestion`, `mcp__anthara__ask_with_card`,
  `mcp__anthara__notify_card`.
- Every name in `connections` is injected as an env var (§7).
- Output post-processing: code fences are stripped; if the result isn't valid JSON it
  is wrapped as `{"output": "<text>"}` — which is why every prompt ends with a
  single-line JSON OUTPUT RULES contract (§11).
- `ai_task` **rejects a `repos` key** — use `coding_task` for repo-based work.
- Interactive mode: see §8.

### `coding_task`
Like `ai_task` but with repo checkout: requires `config.repos` (array of repo URLs) and
a GitHub connection; the pod clones the repos, codes, commits, and raises PRs
(entrypoint `coding`).

### `send_message`
Sends a templated `config.message` through a messaging provider connection (Teams,
console).

### `send_teams_card`
Sends a Teams Adaptive Card from the engine itself (no pod).

### `transform`
Pure data step: `config.input` (template expression), optional `config.path` (JSON
field path), optional `config.template` (output format).

### `echo`
Debug utility — echoes `config.message`.

## 6. `llm_config`

Defaults for agent steps. Exactly these keys are forwarded to the Claude Agent SDK:

| YAML key | Agent SDK option | Typical values |
|----------|------------------|----------------|
| `model` | `model` | `claude-sonnet-5` |
| `max_turns` | `maxTurns` | 50–240 (higher for interactive/coding flows) |
| `max_budget_usd` | `maxBudgetUsd` | 10–50 |
| `effort` | `effort` | `low` \| `medium` \| `high` \| `max` |
| `system_prompt` | `systemPrompt` | omit only for genuinely read-only reasoning — an `ai_task` that uses tools **must** set it (§5) |

Anything else is **rejected at save time** — the workflow will not validate. This is
deliberate: an unrecognised key used to be dropped without comment, so a misspelled
`max_budget_usd` (or a camelCase `maxTurns`) silently removed the run's only cost
ceiling. Types are checked too — `max_turns` must be a positive integer and
`max_budget_usd` a positive number, so a quoted `"40"` is an error rather than a
discarded cap. Budget and turn caps are enforced by the SDK inside the pod.

### Per-step overrides

A step may carry its own `config.llm_config`, which merges over the workflow-level
block key by key — the step's value wins, everything it does not name is inherited.
Use it when one agent step needs a different system prompt, model, or turn cap from
the rest of the file:

```yaml
llm_config:
  model: claude-sonnet-5
  max_turns: 240

steps:
  - name: author-feature-spec-and-raise-draft-pr
    type: ai_task
    config:
      llm_config:
        system_prompt: |
          You are an autonomous QA automation engineer. Use your tools — clone the
          repo, write the feature files, run git, and raise the PR. Do not answer
          with an analysis instead of doing the work.
      prompt: |
        ...
```

## 7. Connections & credentials

- A connection name refers to an **AuthConfig** record in the api (created via the
  integrations UI/endpoints). Names are validated to exist when the workflow is saved.
- Each connection is resolved to an OAuth access token at dispatch time and injected
  into the pod env as `<SANITIZED_NAME>_ACCESS_TOKEN` — the name is uppercased and
  non-alphanumerics become underscores: `Jira` → `JIRA_ACCESS_TOKEN`,
  `Microsoft Teams` → `MICROSOFT_TEAMS_ACCESS_TOKEN`, `Azure DevOps` →
  `AZURE_DEVOPS_ACCESS_TOKEN`, `GitHub` → `GITHUB_ACCESS_TOKEN`.
- The validator rejects: duplicate entries, two names that collide onto the same env
  key, and names whose stem doesn't match `^[A-Z][A-Z0-9_]*$` after sanitization.
- Only `*_ACCESS_TOKEN` env vars pass the pod's env filter — nothing else from the
  caller reaches the agent.
- Tokens are short-lived and are **re-resolved on every resume** after a Teams card
  hibernation, so long human response delays never stale credentials.
- Credential resolution by actor: user-triggered runs use the triggering user's
  connection; GitHub system runs mint an App installation token (via
  `installationId` input); Azure DevOps system runs borrow the workflow creator's
  connection.

## 8. Interactive `ai_task` (Teams collaboration loop)

Set inside `config` (the handler reads `config.interactive`):

```yaml
config:
  interactive: true
  teams_channel_id: '19:...@thread.v2'   # required with a connection
  teams_service_url: 'https://smba.trafficmanager.net/teams/'
  prompt: |
    ...
```

Optional config overrides: `card_title`, `card_description` (card metadata shown in
the run UI), and per-step Teams bot overrides `teams_bot_app_id`,
`teams_bot_app_password`, `teams_conversation_ref`, `teams_bot_tenant_id`.

Lifecycle:
1. The engine creates an `interactive_conversations` row and dispatches the pod with
   Teams bot credentials.
2. When the agent calls `mcp__anthara__ask_with_card`, the card is posted to Teams,
   the SDK session files are persisted to a shared volume, and the pod **exits**
   (hibernation). The run parks as `in_collaboration_loop`. A parked run does not
   hold the workflow's cron slot: queued ticks drain and future ticks fire even
   while a card sits unanswered (§4 `cron`) — design cron + interactive prompts
   for idempotency (freshness windows) so a stale parked run and a fresh run
   never double-process the same work.
3. When the human submits the Adaptive Card, the api resumes: tokens are re-resolved,
   a new pod starts with `resume: <sessionId>`, and the answers arrive as the next
   prompt turn.
4. A run that finishes a resume turn without sending a follow-up card triggers a
   synthetic **recovery card** ("Retry / Stop") instead of silently completing.

### `mcp__anthara__ask_with_card` (ask and WAIT)
Schema:

```yaml
card:                       # prelude rendered above the questions — REQUIRED
  title: "Readiness Clarification"      # keep stable across runs
  description: "markdown: links [x](url), **bold**, *italic*, - lists, \n breaks"
questions:                  # 1–5 entries
  - question: "..."         # required
    header: "chip label"    # optional
    options:                # optional → renders a choice set; omit for free text
      - { label: "...", description: "..." }
    multiSelect: true       # optional → multi-choice
```

Call it **exactly once per turn** — a second card in the same turn is refused.
Everything run-specific (story key, score, bullets) goes in `card.description`, not
the title; the prelude renders above the questions so don't repeat it in question text.

### `mcp__anthara__notify_card` (inform, do NOT wait)
One-way notification: `{ title, description }` only, no questions, no hibernation.
Send it once at the very end of a run; never ask a question after it.

## 9. Templating, fan-out, fan-in

**Templating** — resolved
recursively through every string in a step's `config`:
- `{{inputs.<name>}}` — trigger input (manual inputs or webhook context keys, §4).
- `{{steps.<step-name>.output}}` — the full upstream output (string).
- `{{steps.<step-name>.output.<field>}}` / `...output.items[0].id}}` — JSON property
  paths with array indexing.
- Unknown references, missing properties, or out-of-bounds indexes **throw** and fail
  the step — templates are resolved at execution time, not validation time.

**Fan-out** is automatic: when a step's output parses as a JSON array with **length >
0** and more steps remain, the engine spawns **one child run per item** and the parent
parks as `awaiting_children`. Inside a child, `{{steps.<fan-out-step>.output}}`
resolves to that child's **single item** — hence the "You receive ONE work item from
the upstream fan-out" framing in every prompt. An **empty array** completes the run
immediately (nothing to process). Each child is independent: own status, own step
executions, own Teams cards.

**Fan-in**: a later step with `fan_in: wait_all_collect` waits for all children and
receives their collected outputs as a JSON array; child failures are logged but do not
fail the parent.

## 10. Run lifecycle & operations

Run states: `running` → (`awaiting_children` | `in_collaboration_loop`) →
`completed` | `failed` | `cancelled`.

Useful endpoints: list runs (`GET /workflow-engine/runs`), run detail with step
executions (`GET /workflow-engine/runs/:id`), children
(`GET /workflow-engine/runs/:id/children`), audit trail
(`GET /workflow-engine/runs/:id/audit`), step logs
(`GET /workflow-engine/runs/:runId/steps/:stepId/logs`), cancel
(`POST /workflow-engine/runs/:id/cancel`), toggle enabled
(`PATCH /workflow-engine/workflows/:id`).

## 11. Prompt-writing house style

Every workflow follows these conventions — keep them when writing new
ones:

0. **No YAML comments, plain-language prompts, smallest workflow that works.**
   - Workflow files carry ZERO `#` comments — anything worth explaining belongs in
     the prompt text itself or in this guide. (Lines starting with `#` *inside* a
     block-scalar prompt are string content, not comments — e.g. a PR template — and
     are fine.)
   - Write the prompt so a non-developer can follow it: short plainly-named steps,
     the intent of each step stated before any mechanics, words over jargon, and
     only the API details the agent genuinely needs.
   - Prefer the fewest steps that work; one concern per workflow.
1. **Structure the prompt as short, numbered, plainly-named steps** (find → check →
   understand → act → report).
2. **Credential rules block**: name the exact `*_ACCESS_TOKEN` env vars, state which
   host each token is for, and repeat: never log tokens, never echo them into
   comments/messages, never put them in URLs (exception: the documented
   `x-access-token:<token>@github.com` clone form; for ADO use
   `git -c http.extraHeader="Authorization: Bearer $TOKEN"`).
3. **curl call shape**: always via the Bash tool, always capture status —
   ```bash
   curl -sS -w '\nHTTP_STATUS:%{http_code}\n' \
     -H "Authorization: Bearer $TOKEN" -H "Accept: application/json" "<url>"
   ```
   Build JSON POST bodies in a temp file (`BODY=$(mktemp)`) to avoid escaping pain.
   Pin API versions (ADO `api-version=7.1`, Jira REST v2, Graph `v1.0`).
4. **Single-line JSON OUTPUT RULES**: the step's final output is exactly ONE
   single-line JSON object from an enumerated set of shapes (e.g.
   `{"issueKey": "...", "status": "clarified", ...}`); on unrecoverable failure the
   step fails loudly — never emit a success-shaped object.
5. **Idempotency marker**: every workflow needs one — a tag removed after processing
   (ADO), a literal comment footer (`_Clarification collected via Anthara._`,
   `[anthara-ci-fix]`), a branch-name prefix check, or a PR artifact-link check.
   Check the marker BEFORE writing; exit with an `already_*` status when found.
6. **Fail loudly vs soft-fail**: hard-fail (with HTTP status + body) on credential
   resolution, clone/push failures, and writes that gate idempotency; soft-fail
   (record and continue) on best-effort steps like final notifications and
   PR-to-ticket links.
7. **Teams formatting**: no `#`/`##` markdown headings (they render as giant banners)
   — bold lines for section titles, `- ` bullets, blank lines between topics, no
   tables or fenced code blocks in Teams messages.
8. **Fan-out framing**: prompts downstream of a `read_data` step open with "You
   receive ONE <item> as JSON from the upstream fan-out:
   `{{steps.<fetch-step>.output}}`".
9. **Read-only grounding clones**: into `/tmp` only, never write/commit/push,
   optionally delete after reading. Default to shallow (`--depth 1`); a workflow
   whose task genuinely needs history — e.g. a PR review running `git log`,
   `git blame`, or commit-by-commit passes — may clone full history instead,
   still read-only.
10. **Interactive etiquette**: ask blocking questions via `ask_with_card` (with full
    context in the prelude), defer non-blocking questions into the artifact's "Open
    Questions", cap clarification rounds (existing workflows cap at 3), and finish
    with a single `notify_card`.
11. **Name the card tool for EVERY Teams-facing action**: never write "post a
    message to Teams" or "send a notification" without the exact tool name
    (`mcp__anthara__ask_with_card` / `mcp__anthara__notify_card`) — an unnamed
    Teams instruction plus the curl-primed prompt style leads the agent to
    improvise a Graph `POST /chats/.../messages` with a delegated token, which
    posts AS the connection owner (this happened: a delegated Teams token meant
    for transcript reads was used to post a brainstorm into a meeting chat under
    the owner's identity). If a delegated Graph token is in the pod env, also
    state in CREDENTIAL RULES that it is for reads only and must never send,
    post, or edit messages. `notify_card` is one-way and FINAL — at most once,
    as the last Teams action of the run (terminal early-exit notices are fine);
    never follow it with a question card. Mid-run analysis the team should see
    (e.g. a brainstorm) rides in the next `ask_with_card` prelude `description`
    as a condensed digest, not in a free-form message.

