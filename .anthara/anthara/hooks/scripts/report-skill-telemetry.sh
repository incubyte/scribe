#!/usr/bin/env bash

set -uo pipefail

MAX_PAYLOAD_BYTES=10485760
UUID_PATTERN='^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
SERVER_NAME="anthara-fabric"

SCRIPT_DIR="${BASH_SOURCE[0]%/*}"
source "${SCRIPT_DIR}/lib/fabric-client.sh"

INPUT=$(head -c "$((MAX_PAYLOAD_BYTES + 1))")
[ "${#INPUT}" -le "$MAX_PAYLOAD_BYTES" ] || exit 0

JSON_TOOL=$(detect_json_tool)
[ -n "$JSON_TOOL" ] || exit 0

input_field() {
  local field="$1"
  case "$JSON_TOOL" in
    jq)
      echo "$INPUT" | jq -r --arg f "$field" '.[$f] // ""' 2>/dev/null || echo ""
      ;;
    python3|python)
      echo "$INPUT" | "$JSON_TOOL" -c \
        "import sys, json; print(json.load(sys.stdin).get(sys.argv[1], ''))" \
        "$field" 2>/dev/null || echo ""
      ;;
    node)
      echo "$INPUT" | node -e \
        'try { console.log(JSON.parse(require("fs").readFileSync(0, "utf8"))[process.argv[1]] ?? "") } catch { console.log("") }' \
        "$field" 2>/dev/null || echo ""
      ;;
  esac
}

skill_name() {
  case "$JSON_TOOL" in
    jq)
      echo "$INPUT" | jq -r '
        (.tool_input.skill // "")
        | if (type == "string" and length > 0 and length <= 256) then . else "" end
      ' 2>/dev/null || echo ""
      ;;
    python3|python)
      echo "$INPUT" | "$JSON_TOOL" -c '
import sys, json
tool_input = json.load(sys.stdin).get("tool_input")
skill = tool_input.get("skill") if isinstance(tool_input, dict) else None
print(skill if isinstance(skill, str) and 0 < len(skill) <= 256 else "")
' 2>/dev/null || echo ""
      ;;
    node)
      echo "$INPUT" | node -e '
try {
  const toolInput = JSON.parse(require("fs").readFileSync(0, "utf8")).tool_input;
  const skill = toolInput && typeof toolInput === "object" ? toolInput.skill : undefined;
  console.log(typeof skill === "string" && skill.length > 0 && skill.length <= 256 ? skill : "");
} catch { console.log(""); }
' 2>/dev/null || echo ""
      ;;
  esac
}

typed_command_name() {
  case "$JSON_TOOL" in
    jq)
      echo "$INPUT" | jq -r '
        (.prompt // .user_prompt // "")
        | if type == "string" then . else "" end
        | (capture("^/(?<c>[A-Za-z0-9_-]+(?::[A-Za-z0-9_-]+)?)(?:\\s|$)").c) // ""
        | if length > 0 and length <= 256 then . else "" end
      ' 2>/dev/null || echo ""
      ;;
    python3|python)
      echo "$INPUT" | "$JSON_TOOL" -c '
import json, re, sys
payload = json.load(sys.stdin)
prompt = payload.get("prompt") or payload.get("user_prompt") or ""
match = re.match(r"^/([A-Za-z0-9_-]+(?::[A-Za-z0-9_-]+)?)(?:\s|$)", prompt) if isinstance(prompt, str) else None
name = match.group(1) if match else ""
print(name if 0 < len(name) <= 256 else "")
' 2>/dev/null || echo ""
      ;;
    node)
      echo "$INPUT" | node -e '
try {
  const payload = JSON.parse(require("fs").readFileSync(0, "utf8"));
  const prompt = payload.prompt ?? payload.user_prompt ?? "";
  const match = typeof prompt === "string" ? prompt.match(/^\/([A-Za-z0-9_-]+(?::[A-Za-z0-9_-]+)?)(?:\s|$)/) : null;
  const name = match ? match[1] : "";
  console.log(name.length > 0 && name.length <= 256 ? name : "");
} catch { console.log(""); }
' 2>/dev/null || echo ""
      ;;
  esac
}

PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
CREDENTIALS_FILE="${CLAUDE_CONFIG_DIR:-${HOME}/.claude}/.credentials.json"

telemetry_body() {
  local skill="$1" session_id="$2" source="$3"
  case "$JSON_TOOL" in
    jq)
      jq -cn --arg skill "$skill" --arg session_id "$session_id" --arg source "$source" \
        '{skill: $skill, anthara_session_id: $session_id, invocation_source: $source}' 2>/dev/null
      ;;
    python3|python)
      "$JSON_TOOL" -c \
        "import json, sys; print(json.dumps({'skill': sys.argv[1], 'anthara_session_id': sys.argv[2], 'invocation_source': sys.argv[3]}))" \
        "$skill" "$session_id" "$source" 2>/dev/null
      ;;
    node)
      node -e 'console.log(JSON.stringify({skill: process.argv[1], anthara_session_id: process.argv[2], invocation_source: process.argv[3]}))' \
        "$skill" "$session_id" "$source" 2>/dev/null
      ;;
  esac
}

case "$(input_field hook_event_name)" in
  PostToolUse)
    SKILL=$(skill_name)
    SOURCE="skill_tool"
    ;;
  UserPromptSubmit)
    SKILL=$(typed_command_name)
    SOURCE="typed_command"
    ;;
  *)
    exit 0
    ;;
esac
[ -n "$SKILL" ] || exit 0

SESSION_ID=$(input_field session_id)
[[ "$SESSION_ID" =~ $UUID_PATTERN ]] || exit 0

FABRIC_URL=$(fabric_url) || exit 0
FABRIC_URL=$(expand_placeholders "$FABRIC_URL") || exit 0

TOKEN=$(access_token "$FABRIC_URL") || exit 0
[ -n "$TOKEN" ] || exit 0

TELEMETRY_URL=$(telemetry_url "$FABRIC_URL" "skill-invocations") || exit 0

BODY=$(telemetry_body "$SKILL" "$SESSION_ID" "$SOURCE") || exit 0
[ -n "$BODY" ] || exit 0

printf 'header = "Authorization: Bearer %s"\n' "$TOKEN" | curl --config - --max-time 5 -fsS -X POST \
  -H 'Content-Type: application/json' \
  --data "$BODY" \
  "$TELEMETRY_URL" >/dev/null 2>&1 &

exit 0
