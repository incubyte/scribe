---
name: browser-verification
description: Tech-agnostic guidance for verifying a running app in a real browser during a plan-implementer run. Drives each UI slice's Verification block (user-facing flow + accessibility) — from the plan's Part 2 entry, or the spec's slice block for older plans — against the running app via a browser MCP — chrome-devtools-mcp primary, Claude-in-Chrome fallback. Assumes the dev server is already running; never boots it. Console errors are failures. Degrades to a manual checklist when no browser MCP is connected — never a silent pass. Loaded by the browser-verifier agent and available to debug-fix. Loaded as context by other agents — not user-invokable.
---

# Browser Verification

Browser MCP tools give end-of-spec verification that unit tests cannot: whether the app's
user-observable behavior actually happens in a running browser, whether the UI is accessible, and
whether the console is clean. This is the operational half of each UI slice's **Verification
block** (steps 3 user-facing flow, 4 accessibility) — the half unit tests never cover. The block
lives in the slice's plan Part 2 entry; plans authored before it moved there keep it in the spec.

This skill applies to any agent driving browser verification: the `browser-verifier` agent (both
first-pass and re-verify), and `debug-fix` when a UI fix needs eyes-on confirmation.

## Provider detection (do this ONCE, at session start)

Decide the provider once; do not re-check per operation. Use the chosen provider's tools
consistently.

### Step 1 — chrome-devtools-mcp (primary)
Anthara ships this in `.mcp.json`. Use `ToolSearch` with query `"+chrome-devtools"` to load the
tools, then call `list_pages`.
- Succeeds → `provider = chrome-devtools-mcp`. Use the tool table below.
- Tools present but `list_pages` fails → the plugin is installed but no browser is connected; note
  that in the report and continue to Step 2.

### Step 2 — Claude-in-Chrome (fallback)
Call `tabs_context_mcp` (`mcp__claude-in-chrome__tabs_context_mcp`).
- Succeeds → `provider = claude-in-chrome`.
- Fails → no provider. Degrade (see below).

### Step 3 — no provider
Degrade: skip automated verification, note it explicitly, and emit the in-scope Verification blocks
(steps 3–4) as a manual checklist for the user. **Never return a silent PASS.**

## Tool tables

**chrome-devtools-mcp** (prefix `mcp__plugin_chrome-devtools-mcp_chrome-devtools__`):

| Job | Tool |
|---|---|
| availability | `list_pages` |
| open / load | `new_page`, `navigate_page`, `wait_for` |
| observe | `take_snapshot` (element uids), `evaluate_script`, `take_screenshot` |
| interact | `click`, `fill`, `fill_form`, `press_key`, `hover` |
| console | `list_console_messages` |
| network | `list_network_requests` |

**Claude-in-Chrome** (prefix `mcp__claude-in-chrome__`):

| Job | Tool |
|---|---|
| availability | `tabs_context_mcp` |
| open / load | `tabs_create_mcp`, `navigate` |
| observe | `read_page`, `get_page_text`, `computer` (screenshot) |
| interact | `find`, `computer` (click/type/scroll), `javascript_tool` |
| console | `read_console_messages` |

## Dev server — assume it is already running

Do **not** boot the dev server. A developer runs it in their own terminal; a headless run wires it
upstream. Detect the base URL, then drive it:
1. `CLAUDE.md` in the target project (dev command / URL).
2. Prior chat context (a mentioned URL / port).
3. `package.json` scripts / framework defaults (Vite `5173`, Next `3000`, etc.) — as a *guess of
   the URL to try*, not a command to run.

Confirm the app is reachable at the URL (load it, screenshot). If nothing responds, report
*"app not running at &lt;url&gt; — start the dev server and re-run verification"* and stop; do not
attempt to start it.

## What to verify

For each UI slice's Verification block:
- **User-facing flow (step 3).** Drive the slice's primary user-observable behavior end-to-end —
  navigate, interact, assert the AC's promised outcome actually appears. Translate the AC into
  browser actions by judgment; no explicit test scripts.
- **Accessibility (step 4).** Tab-order traversal, focus-visibility on every interactive control,
  aria-labels per the context file's **§8a Design system** and the project's WCAG level.
- **Console = failure.** Read the console after each flow. An error or unhandled rejection is a
  **failure**, not just evidence — even when the visual state looks correct.
- **Design drift (when §8a present).** Flag hand-tuned colors/spacing or a re-created component
  where the system provides one — medium severity.
- **Screenshot** each flow (pass and fail) as evidence.

## Severity (align with the reviewer's shape)
- **HIGH** — a broken user flow, an accessibility barrier (no keyboard path, missing focus trap on
  a modal), or a console error/exception.
- **MEDIUM** — design drift vs §8a, minor a11y gaps.
- **LOW** — cosmetic notes.

## Re-verify mode (for the fix loop)
When invoked to re-check a specific prior finding after a `slice-builder` fix, drive **only that
finding's flow** again and report `closed` or `still-failing` — not a full re-run. A UI fix may
need the frontend to rebuild / HMR to settle before re-checking; wait for the page to reflect the
change. A finding is **closed only when the browser confirms it passes**.

If re-verification cannot execute (app down, provider disconnected, rebuild didn't happen), report
the finding as **`unverified` (fix applied, not confirmed)** with the manual step — never mark it
closed. Unconfirmed is not passed.

## Discipline
- Decide provider once; consistent tools thereafter.
- Read-only w.r.t. app code — verify and report; the orchestrator routes fixes to `slice-builder`.
- One fresh page/tab per run; do not reuse a stale session.
- Specific failures only: AC text, expected vs observed, the console error if any. Vague failures
  are useless to the fix loop.
- Never a silent pass — skipped or unverified is stated explicitly.
