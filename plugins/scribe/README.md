# Scribe plugin

The Scribe plugin gives Claude two commands for your calls: **recap** lists your recent calls with a title, summary and participants for each, and **meeting-notes** writes one call up as notes with todos. It reads your calls through Scribe's `scribe` tools. Scribe records and stores your calls on your computer. A transcript is shared with Claude only when you ask Claude to read it, and an AI action on a todo sends that todo's prompt to Claude.

## Requirements

- Scribe installed on this computer, with **Connect to Claude Desktop** clicked in the Scribe popover. See [Connect an AI app](../../README.md#connect-an-ai-app).
- Claude Desktop restarted after connecting.
- A paid Claude plan (web or Desktop) for the AI actions on todos.
- Code execution turned on in Claude, so the notebook template can load.
- A Claude that can publish artifacts (claude.ai/artifact links) for the notebook.

## Install

**Claude Code**

1. Run `/plugin marketplace add incubyte/scribe`.
2. Run `/plugin install scribe@scribe`.

**Claude Desktop**

1. Open **Customize**, then **Plugins**.
2. Click **Add marketplace** and enter `incubyte/scribe`.
3. Install **Scribe**.

If the marketplace install does not work, download `scribe-plugin.zip` from the latest GitHub Release and upload it under **Customize**, then **Plugins**.

## Commands

- `/recap` lists today's and yesterday's calls. Try `/recap last 3` or "recap my calls since Monday". In Claude Code it reads the calls in parallel with a small `call-summarizer` agent; in Claude Desktop it reads them one at a time.
- `/meeting-notes` writes up one call. Try "meeting notes for my call with Priya". Use `/meeting-notes` with a row number from a recap in the same chat. Each call is added to your one Scribe notebook, from any chat.
- Ask Claude to import your old Scribe notebooks, or to update your notebook page after a plugin update.

You can also just ask in plain words, such as "what calls did I have this week?"

## Versions

The plugin has its own version in `plugins/scribe/.claude-plugin/plugin.json`. It changes only when the plugin changes, and it is separate from the Scribe app version. Bump it in the same commit as any plugin change.

## Next candidates

- "Ask across calls", for questions that span several calls.
- "Follow-up email", for a draft email from one call.
