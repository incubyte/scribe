---
name: recap
description: Recap the user's recent calls and meetings recorded by Scribe on this computer, as one table with a title, summary and participants for each call. Use when the user asks to recap their calls, asks what calls or meetings they had, or asks to summarise this week's meetings. By default it covers today's and yesterday's calls. Accepts a date such as "since Monday" or "this week", and "last N".
---

# Recap recent calls

Scribe records the user's calls and meetings and stores the transcripts on this computer. The user's lines are marked `Me` and the other side is marked `Them`. A transcript is shared with Claude only when the user asks Claude to read it.

Use the `scribe` tools to find and read calls: `list_transcripts`, `get_transcript` and `search_transcripts`. If the tools are not enough and you can read local files yourself, as in Claude Code or Cowork, you may open Scribe's files directly: ask `get_transcripts_folder` where they are, never guess the path. Do this only for something the tools don't give, such as a file the user pointed to or a screenshot you need at full resolution.

## When the Scribe tools are missing

If the `scribe` tools are not available, tell the user to click **Connect to Claude Desktop** in the Scribe popover, then restart Claude Desktop. Then stop.

## Choose the calls

Work out the scope from the request before you call anything.

- No number and no date: today's and yesterday's calls. Pass yesterday's date as `since`.
- A date phrase ("since Monday", "this week", "since 2026-10-13"): convert it to `YYYY-MM-DD` using today's date, and pass it as `since`.
- "last N" or "N calls": the newest N calls, with no `since`.

Never recap more than 15 calls. Call `list_transcripts` with `limit` set explicitly: 15 for a date scope, or N (at most 15) for "last N". Never leave `limit` out, because the default is 20. If a date scope returns 15 calls, there may be more, so say the recap shows the newest 15 and suggest a shorter range. Say so too if you capped a "last N" request.

If the list is empty, reply "No calls today or yesterday" for the default scope, or "No calls recorded since <date>" for a date scope, or "No calls recorded yet" for "last N", then stop.

Keep the list's order, which is newest first.

## Read each call

If you can start sub-agents, start one `scribe:call-summarizer` agent per call, all in the same turn so they run in parallel. Give each one the call's `id`, `started_at` and `status`. Each returns a block with `title`, `summary` and `participants`, or `error`. Do not call `get_transcript` yourself in this case. If a block comes back malformed, read that one call yourself as described below.

If you cannot start sub-agents, call `get_transcript` once for each `id` in the list. Read one call at a time.

From each transcript, take only what the summary and participants need:

- A 3 to 6 word title that describes what the call was about. Write it from the content. Never use the stored `title`, because it is always the default `Call YYYY-MM-DD HH:MM` stamp.
- A 2 to 3 sentence summary of the main points and any outcome.
- Names of the other people, but only when the dialogue says them, such as "Thanks, Priya" or "Sam, can you send that over?". Do not guess a name from a topic or a role.

If `get_transcript` or a sub-agent returns an error for one call, show that row with "Couldn't read this call" in the summary and carry on with the rest.

## Write the table

Show one markdown table with these columns, in this order:

`| # | Meeting | Summary | Participants |`

- **#**: the row number, starting at 1.
- **Meeting**: the inferred title on the first line, then the date and time from `started_at` in the form `Oct 14, 10:30`. Add `(incomplete)` after the date when the call's `status` is `incomplete`. Do not hide incomplete calls.
- **Summary**: the 2 to 3 sentence summary on one line, with no line breaks inside the cell.
- **Participants**: `Me`, then each name heard in the dialogue, each followed by "(inferred)". If no name was heard, write `Me, Them`.

Put nothing else above the table. After it, add one line: "Run `/meeting-notes` with any row number to write up that call."

## Rules

- Never show the stored `title` as the meeting title.
- Never present a name as fact. Every name other than `Me` carries "(inferred)".
- Never hide an incomplete call.
- Never read more calls than the scope allows, whether yourself or through sub-agents.
- Keep the table to the four columns. Put extra detail in the summary, not in new columns.
