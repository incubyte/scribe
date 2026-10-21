---
name: meeting-notes
description: Write up a call or meeting recorded by Scribe on this computer as notes with speakers, a summary and todos, kept in one Scribe notebook artifact. Use when the user asks for meeting notes, notes from their last call, or to write up the call with someone. Accepts a row number from an earlier recap, a name or a topic.
---

# Write meeting notes

Scribe records the user's calls and meetings and stores the transcripts on this computer. The user's lines are marked `Me` and the other side is marked `Them`. A transcript, and any screenshot fetched with `get_screenshot`, is shared with Claude only when Claude reads it. An AI action on a todo sends that todo's prompt to Claude.

Use the `scribe` tools to find and read calls: `list_transcripts`, `get_transcript`, `search_transcripts` and, when present, `get_screenshot`. If the tools are not enough and you can read local files yourself, as in Claude Code or Cowork, you may open Scribe's files directly: ask `get_transcripts_folder` where they are, never guess the path. Do this only for something the tools don't give, such as a file the user pointed to or a screenshot you need at full resolution.

## When the Scribe tools are missing

If the `scribe` tools are not available, tell the user to click **Connect to Claude Desktop** in the Scribe popover, then restart Claude Desktop. Then stop.

## Pick the call

- No argument: the latest call. Call `list_transcripts` with `limit` 1.
- A row number: the row of the `/recap` table earlier in this chat.
- A name or topic: call `search_transcripts` with one to three short keywords, never a sentence. Search is a literal match, so try another keyword if nothing comes back.

If several calls match, list up to 5 with their inferred title and date and ask which one. Then call `get_transcript` for that `id`. Read one call only. Each line looks like `[HH:MM:SS] **Me:** text`, where the clock is the offset into the call, not wall time.

## Work out who spoke

Only `Me` and `Them` exist. `Them` can be several people on one lane.

- Name someone only from the dialogue, such as "Thanks, Priya" or "Sam, can you send that over?". Never guess from a topic or a role.
- A speaker's `label` is exactly `Me` or `Them`. The `name` field carries the person, or an empty string when there is no name.
- Add one entry with label `Me`, the user's name in `name` only when the dialogue gives it (else empty), and a `confidence` stated explicitly.
- Add one entry per inferred person on the `Them` lane, each with label `Them` and that person's `name`.
- For each entry give a `confidence`: `stated` when they name themselves, `likely` when others address them, `unknown` when you cannot tell. Keep the quote or line that shows it as `evidence`.
- With no evidence, use one entry with label `Them`, an empty `name` and confidence `unknown`. The notebook shows it as "Them (one or more people)".
- `Me` is always the user.

## Pass 1: read the transcript

Work from the transcript text only, with no images. Decide:

- The call's nature, as a free short `type` label such as "Product discussion".
- Who the notes are for and what shape serves them. Ask: who reads this? What decision or work happens next? What would they hate to dig for?
- Which kinds of actions fit the call.
- Which screenshots are worth seeing, when `get_transcript` ends with a `## Screenshots` section of lines like `- [HH:MM:SS] <session id>/<HH-MM-SS>.png`. Pick at most 8, each with a one-line reason tied to the discussion near its offset. Skip any screenshot adjacent to nothing substantive.

## Pass 2: look, revise, write

Call `get_screenshot` with the name of each chosen screenshot, and only those. Read each image together with the transcript about 60 seconds either side of its offset. If an image changes the picture, revise the plan from pass 1. Then write the notes.

If there is no `## Screenshots` section, or `get_screenshot` is unavailable, skip the image step silently and write the notes from the transcript.

Treat each image by what it shows:

- Slide or document: its key points, under that moment.
- Dashboard or metrics: the numbers, plus what was said about them.
- Error or broken UI: a todo with a bug-report action.
- Diagram: a description, plus the decisions made about it.
- Chat, email or ticket: the commitments in it, as todos.
- Anything sensitive (passwords, tokens, personal data): note "sensitive screen" only. Never transcribe it.

Screen-derived notes and todos carry the offset they came from, for example "(screen at 00:12:34)".

## Shape the notes

Write a 2 to 3 sentence `summary`, then `blocks` composed for the reader. This is guidance, not a template: use few blocks, drop anything empty, and skip play-by-play unless the order matters. `guide.md` beside this file has a worked example for each shape.

- Executive call: decisions, risks and asks.
- Product discussion: the problem, options weighed, the decision, open questions.
- Sales or client call: pain, budget and timeline, objections, the next step.
- Engineering or incident: what broke, the cause, the fix, follow-ups.

For any other call, ask the three questions and shape the notes to the answers.

## Todos and AI actions

Write 0 to 6 todos, and none when the call has no follow-ups. Each has an `id` (`t1`, `t2`, and so on), `text`, `owner` (`Me`, an inferred name, or `Unassigned`), `due` (a date or phrase from the call, else an empty string) and 1 to 2 `actions`.

An action is a `label` and a `prompt`. Choose actions that fit the call: "Create ticket" for engineering work, "Draft follow-up email" for clients, "Add lead to CRM" for sales, "Prepare agenda" for the next meeting. An action only drafts text. A CRM action drafts the record and says where it goes, because the action cannot write to a CRM itself.

Write the prompt so it works alone: it states the task, names the owner and includes only the facts from the meeting that the task needs. Name the form of the output, for example "a 5-line email to Priya".

## Build or update the notebook

Read `notebook.html` from this skill's folder. If you cannot read it, say the notebook needs code execution turned on, give the notes as plain markdown, and stop.

Each meeting in the data island has these fields:

- `id`: the transcript `id`.
- `title`: a 3 to 6 word title you write from the content. Never use the stored `title`, because it is always the default `Call YYYY-MM-DD HH:MM` stamp.
- `started_at` and `duration_seconds`: copied from `list_transcripts`.
- `type`: the free short label.
- `summary`: as above.
- `speakers`: a list of `label`, `name`, `confidence`, `evidence`.
- `blocks`: a list of objects, each with a `kind` and the fields below. The notebook skips any other kind.
  - `heading {text}`, `paragraph {text}`, `list {items[]}`
  - `decision {text, by?}`, `quote {text, speaker}`
  - `metric {label, value, note?}`
  - `table {columns[], rows[][]}`, for real tabular data only
  - `screen {offset, caption}`: a caption and offset only, because images cannot be embedded
  - `callout {tone, text}`, with `tone` either `info` or `risk`
- `todos`: a list of `id`, `text`, `owner`, `due`, `actions`, where each action is `label` and `prompt`.

Make the artifact like this:

1. If no Scribe notebook artifact exists in this conversation, create one titled `Scribe notebook` from the template, with the data island `<script type="application/json" id="scribe-meetings">` set to a one-meeting array holding the new meeting.
2. If one exists, take the meetings array from the data island of the latest version of that artifact, which you wrote earlier in this conversation. Keep every meeting exactly as it is. Replace a meeting only when it has the same `id` as the new one, otherwise put the new meeting first. Then update that same artifact with the full template and the complete array.

When you publish with an Artifact tool that takes `capabilities` (claude.ai/artifact links), pass `capabilities: {sample: {}}` when you create the notebook, or the AI action buttons stay disabled. Leave `capabilities` out when you update it, so the declaration carries forward.

Start from the unmodified template file each time. Change only the data island. Never edit the template's markup, styles or script.

Reply with one line that names the meeting you added and says how many meetings the notebook now holds.

## Rules

- Never present a guessed name as fact. Every name other than `Me` carries a `confidence`.
- Never create a second notebook in the same conversation.
- Never drop or change an earlier meeting when you update the notebook.
- Write the JSON for the data island with `<` escaped as `<`, so transcript text can never close the script.
- Give every todo 1 to 2 actions, each with a non-empty `label` and `prompt`.
- Fetch at most 8 screenshots, and never transcribe a sensitive screen.
