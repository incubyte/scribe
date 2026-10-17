#!/usr/bin/env bash
# UserPromptSubmit hook for Fabric — coach Claude on when to fetch memory
# via mcp__plugin_anthara_anthara-fabric__search_memories, instead of pulling an
# unfiltered dump of the graph at session start.
#
# Pattern adapted from mem0's mem0-plugin/scripts/on_user_prompt.sh: the
# agent has more context than this script does, so we inject a decision
# rubric and let the agent decide whether/how to search. Short prompts
# (acknowledgements / continuations) skip silently so the rubric doesn't
# drown trivial messages.
#
# Input:  JSON on stdin (prompt, session_id, cwd, transcript_path)
# Output: Coaching rubric injected into Claude's context (exit 0)

set -uo pipefail

INPUT=$(cat)

# Try jq, then python3. If neither is available the prompt stays empty
# and we fall through to printing the rubric — over-coaching is
# preferable to silently skipping.
PROMPT=""
if command -v jq >/dev/null 2>&1; then
  PROMPT=$(echo "$INPUT" | jq -r '.prompt // ""' 2>/dev/null || echo "")
elif command -v python3 >/dev/null 2>&1; then
  PROMPT=$(echo "$INPUT" | python3 -c "import sys, json; d=json.load(sys.stdin); print(d.get('prompt', ''))" 2>/dev/null || echo "")
fi

# Skip the rubric on short replies (acknowledgements / continuations).
# If parsing failed PROMPT is empty and we fall through to emit.
if [ -n "$PROMPT" ] && [ ${#PROMPT} -lt 20 ]; then
  exit 0
fi

cat <<'EOF'
## Fabric memory — search deliberately, do not flood

CALL `mcp__plugin_anthara_anthara-fabric__search_memories`  WHEN the user:
  - references past work, decisions, or anything "we" built
  - asks "how should we...", "best way to...", or any decision-style question
  - hits an error, bug, or asks for debugging help
  - requests work touching their stack, tools, conventions, or preferences
  - mentions a person, project, or term you do not recognize this session

SKIP search WHEN:
  - the prompt is an acknowledgement or short continuation
  - the user is *stating* new info — that's a write trigger (`mcp__plugin_anthara_anthara-fabric__add_private_memory` / `mcp__plugin_anthara_anthara-fabric__add_shared_memory`), not a search
  - pure syntax / general-knowledge question answerable from training
  - you already searched this scope earlier in this turn

NARROW WITH `scope` — it is the precision lever. Pass `scope="private"` when the answer is about THIS USER (their preferences, habits, background, corrections they gave you). Pass `scope="shared"` when it is about the ORG (policies, architecture decisions, tooling, team conventions, URLs). Leave the default `scope="all"` only when the answer could genuinely be either.

WRITE THE QUERY LIKE A QUERY — it is matched against the text of the stored facts, so use the words a stored fact would itself contain: the person, the project, the service, the technology, the concept. Strip conversational filler. `"ArgoCD production deploy"` beats `"how do we deploy things again"`. Narrow the `limit` (default 10) rather than widening the query.

READ DOWN THE LIST — results come back as facts in the graph engine's own order, each carrying a `status` (`active` / `invalid` / `expired`). There is NO relevance score, so there is nothing to threshold on and nothing to discard: a useful fact is often near the bottom. By default only currently-true facts are returned; pass `as_of=<datetime>` (query searches only) when you need how things stood at a past moment, including facts since superseded.

Empty results are normal — proceed without context.

EOF

exit 0
