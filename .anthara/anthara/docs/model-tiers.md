# Model tiers — the single source of truth

Anthara references models only by **capability tier**, never by concrete model name. This file is the one place that maps a tier to a model. Changing the model lineup means editing only this file; nothing else in the plugin names a model.

Skills and agents reference a tier (`best` / `excellent` / `fast`); `/anthara:start` writes tiers into the plan; `/anthara:plan-implementer` resolves a tier to the value below and passes it as the Claude Code subagent `model` field when spawning an agent.

## The tiers

| Tier | Subagent `model` value | Model | Use it for |
|---|---|---|---|
| `best` | `opus` | Claude Opus 5 | Most complex reasoning and agentic coding |
| `excellent` | `sonnet` | Claude Sonnet 5 | The default coding tier — speed and intelligence |
| `fast` | `haiku` | Claude Haiku 4.5 | High-volume, low-complexity work where speed matters |

The `model` value (`opus` / `sonnet` / `haiku`) is the Claude Code subagent alias; it resolves to the current generation of that model line, so the table tracks the lineup without pinning a dated ID.

## Tier definitions

Descriptions are quoted from the official Anthropic model overview (platform.claude.com/docs/en/docs/about-claude/models/overview).

### `best`

> Anthropic's most capable Opus-tier model for complex reasoning and agentic coding.

The orchestrator (`/anthara:start`) always runs at `best`: it triages the work and authors the plan once per ticket, so the single most consequential reasoning step gets the strongest model. Assign `best` to a slice's agents when the logic is genuinely hard — non-trivial algorithms, state machines, intricate coordination, or new abstractions the slice must design.

### `excellent`

> The best combination of speed and intelligence.

The default coding tier. Assign `excellent` to slice agents doing standard implementation work — the bulk of features and well-understood changes. Strong enough to produce good code when well-fed by the plan and context, cheaper and faster than `best`.

### `fast`

> The fastest model with near-frontier intelligence.

Assign `fast` to high-volume, low-complexity work where latency and cost dominate and the task shape is obvious — CRUD passthrough, rendering an existing field, wiring an existing utility, localized edits.

## How resolution works

1. A tier (`best` / `excellent` / `fast`) appears in a plan or a skill instruction.
2. `/anthara:plan-implementer` looks the tier up in the table above and reads its subagent `model` value.
3. That value is passed as the `model` parameter when the agent is spawned (Claude Code subagent `model` field — accepts `opus` / `sonnet` / `haiku`, a full model ID, or `inherit`).

## Changing the lineup

When the model lineup moves (a new Opus generation ships, a tier should point at a different line), edit **only the table and definitions above**. Because no other skill, agent, or plan hard-codes a model, this file is the sole edit point — the rest of the plugin keeps working unchanged.
