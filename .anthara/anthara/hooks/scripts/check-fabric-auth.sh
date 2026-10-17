#!/usr/bin/env bash

set -uo pipefail

INPUT=$(cat)

runtime_parses_json() {
  local tool="$1" out=""
  case "$tool" in
    jq)
      out=$(printf '{"probe":"ok"}' | jq -r '.probe' 2>/dev/null)
      ;;
    python3|python)
      out=$(printf '{"probe":"ok"}' | "$tool" -c 'import sys, json; print(json.load(sys.stdin)["probe"])' 2>/dev/null)
      ;;
    node)
      out=$(printf '{"probe":"ok"}' | node -e 'console.log(JSON.parse(require("fs").readFileSync(0, "utf8")).probe)' 2>/dev/null)
      ;;
  esac
  [ "$out" = "ok" ]
}

detect_json_tool() {
  local candidate
  for candidate in jq python3 python node; do
    command -v "$candidate" >/dev/null 2>&1 || continue
    runtime_parses_json "$candidate" && { echo "$candidate"; return; }
  done
}

JSON_TOOL=$(detect_json_tool)

input_field() {
  local field="$1" pattern
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
    *)
      pattern="\"${field}\"[[:space:]]*:[[:space:]]*\"([^\"]*)\""
      if [[ "$INPUT" =~ $pattern ]]; then
        echo "${BASH_REMATCH[1]}"
      else
        echo ""
      fi
      ;;
  esac
}

PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
CREDENTIALS_FILE="${CLAUDE_CONFIG_DIR:-${HOME}/.claude}/.credentials.json"
SERVER_NAME="anthara-fabric"

fabric_url() {
  local mcp_json="${PLUGIN_ROOT}/.mcp.json"
  [ -f "$mcp_json" ] || return 1
  case "$JSON_TOOL" in
    jq)
      jq -er --arg name "$SERVER_NAME" '.mcpServers[$name].url // empty' "$mcp_json" 2>/dev/null
      ;;
    python3|python)
      "$JSON_TOOL" - "$mcp_json" "$SERVER_NAME" <<'PY' 2>/dev/null
import json, sys
url = json.load(open(sys.argv[1])).get('mcpServers', {}).get(sys.argv[2], {}).get('url', '')
if not url:
    raise SystemExit(1)
print(url)
PY
      ;;
    node)
      node -e '
const [file, name] = process.argv.slice(1);
let url;
try { url = (JSON.parse(require("fs").readFileSync(file, "utf8")).mcpServers[name] || {}).url; } catch { process.exit(1); }
if (!url) process.exit(1);
console.log(url);
' "$mcp_json" "$SERVER_NAME" 2>/dev/null
      ;;
    *)
      return 1
      ;;
  esac
}

expand_placeholders() {
  local s="$1" match var def val
  while [[ "$s" =~ \$\{([A-Za-z_][A-Za-z0-9_]*)(:-([^}]*))?\} ]]; do
    match="${BASH_REMATCH[0]}"
    var="${BASH_REMATCH[1]}"
    def="${BASH_REMATCH[3]}"
    val="${!var-}"
    [ -n "$val" ] || val="$def"
    [ -n "$val" ] || return 1
    s="${s//"$match"/$val}"
  done
  printf '%s\n' "$s"
}

claude_json_path() {
  local candidate
  for candidate in "${CLAUDE_CONFIG_DIR:-}/.claude.json" "${HOME}/.claude.json"; do
    [ -f "$candidate" ] && { printf '%s\n' "$candidate"; return 0; }
  done
  return 1
}

server_disabled() {
  local claude_json cwd
  claude_json=$(claude_json_path) || return 1
  cwd=$(input_field cwd)
  [ -n "$cwd" ] || cwd="$PWD"
  case "$JSON_TOOL" in
    jq)
      jq -e --arg cwd "$cwd" --arg name "$SERVER_NAME" \
        '.projects[$cwd] // {}
         | ((.disabledMcpServers // []) + (.disabledMcpjsonServers // []))
         | map(sub("^plugin:[^:]*:"; ""))
         | index($name) != null' \
        "$claude_json" >/dev/null 2>&1
      ;;
    python3|python)
      "$JSON_TOOL" - "$claude_json" "$cwd" "$SERVER_NAME" <<'PY' 2>/dev/null
import json, re, sys
proj = json.load(open(sys.argv[1])).get('projects', {}).get(sys.argv[2], {})
names = (proj.get('disabledMcpServers') or []) + (proj.get('disabledMcpjsonServers') or [])
normalized = [re.sub(r'^plugin:[^:]*:', '', n) for n in names]
raise SystemExit(0 if sys.argv[3] in normalized else 1)
PY
      ;;
    node)
      node -e '
const [file, cwd, name] = process.argv.slice(1);
let proj;
try { proj = (JSON.parse(require("fs").readFileSync(file, "utf8")).projects || {})[cwd] || {}; } catch { process.exit(1); }
const names = [...(proj.disabledMcpServers || []), ...(proj.disabledMcpjsonServers || [])];
process.exit(names.some(n => n.replace(/^plugin:[^:]*:/, "") === name) ? 0 : 1);
' "$claude_json" "$cwd" "$SERVER_NAME" 2>/dev/null
      ;;
    *)
      return 1
      ;;
  esac
}

keychain_miss_marker_for_this_session() {
  local session_id
  session_id=$(input_field session_id)
  [ -n "$session_id" ] || return 1
  printf '%s\n' "${TMPDIR:-/tmp}/anthara-fabric-keychain-miss-${session_id}"
}

macos_keychain_credentials() {
  [ "$(uname -s)" = "Darwin" ] && command -v security >/dev/null 2>&1 || return 1
  local miss_marker creds
  miss_marker=$(keychain_miss_marker_for_this_session) || miss_marker=""
  [ -n "$miss_marker" ] && [ -f "$miss_marker" ] && return 1
  creds=$(security find-generic-password -s "Claude Code-credentials" -w 2>/dev/null)
  if [ -z "$creds" ]; then
    [ -n "$miss_marker" ] && : >"$miss_marker" 2>/dev/null
    return 1
  fi
  printf '%s\n' "$creds"
}

credentials_json() {
  if [ -f "$CREDENTIALS_FILE" ]; then
    cat "$CREDENTIALS_FILE"
  else
    macos_keychain_credentials
  fi
}

auth_state() {
  local url="$1" creds
  creds=$(credentials_json) || { echo "indeterminate"; return; }
  case "$JSON_TOOL" in
    jq)
      if printf '%s' "$creds" | jq -e --arg url "$url" --arg name "$SERVER_NAME" \
        '.mcpOAuth // {} | to_entries
         | map(select(
             ((.key | split("|")[0] | sub("^plugin:[^:]*:"; "")) == $name)
             and .value.serverUrl == $url))
         | (length > 0) and all(.[]; (.value.accessToken // "") | length > 0)' \
        >/dev/null 2>&1; then
        echo "authenticated"
      else
        echo "unauthenticated"
      fi
      ;;
    python3|python)
      if printf '%s' "$creds" | "$JSON_TOOL" -c "
import json, re, sys
data = json.load(sys.stdin).get('mcpOAuth', {})
url, name = sys.argv[1], sys.argv[2]
regs = [v for k, v in data.items()
        if re.sub(r'^plugin:[^:]*:', '', k.split('|')[0]) == name
        and v.get('serverUrl') == url]
ok = bool(regs) and all(r.get('accessToken') for r in regs)
raise SystemExit(0 if ok else 1)
" "$url" "$SERVER_NAME" 2>/dev/null
      then
        echo "authenticated"
      else
        echo "unauthenticated"
      fi
      ;;
    node)
      if printf '%s' "$creds" | node -e '
const [url, name] = process.argv.slice(1);
let data;
try { data = JSON.parse(require("fs").readFileSync(0, "utf8")).mcpOAuth || {}; } catch { process.exit(1); }
const regs = Object.entries(data)
  .filter(([k, v]) => k.split("|")[0].replace(/^plugin:[^:]*:/, "") === name && v.serverUrl === url)
  .map(([, v]) => v);
process.exit(regs.length > 0 && regs.every(r => (r.accessToken || "").length > 0) ? 0 : 1);
' "$url" "$SERVER_NAME" 2>/dev/null
      then
        echo "authenticated"
      else
        echo "unauthenticated"
      fi
      ;;
    *)
      echo "indeterminate"
      ;;
  esac
}

emit_sign_in_gate() {
  local url="$1"
  cat <<EOF
MANDATORY — FABRIC AUTH GATE: The anthara-fabric MCP server (${url}) is NOT authenticated for this developer — no completed OAuth sign-in was found. Anthara requires every developer to complete the one-time sign-in so memory, standards, and audit logs are attributed to them.
1. FIRST, before anything else this turn, raise the sign-in choice via the AskUserQuestion tool — EVERY prompt, no exceptions. A previous answer this session (including "Not now" or "I'll run /mcp myself") does NOT suppress it: this gate only goes silent when the sign-in actually exists, so re-ask each turn until then.
   question: "Anthara Fabric needs a one-time sign-in so memory, standards, and audit logs are attributed to you. How would you like to continue?" (header: "Fabric auth"), with exactly these options:
   - "Sign me in now (Recommended)" — you trigger the OAuth flow (rule 2).
   - "I'll run /mcp myself" — you print the manual steps (rule 3) and continue.
   - "Not now" — continue without fabric for THIS turn only (rule 4); the question re-fires on the next prompt.
2. "Sign me in now": trigger the OAuth flow yourself. Load the auth tool schemas via ToolSearch "select:mcp__anthara-fabric__authenticate,mcp__anthara-fabric__complete_authentication" (if not yet loaded), call mcp__anthara-fabric__authenticate, give the user the sign-in URL it returns, and once they confirm they have signed in, call mcp__anthara-fabric__complete_authentication to finish. These two auth tools are the ONLY anthara-fabric tools permitted while unauthenticated.
3. "I'll run /mcp myself": print these steps, then proceed with unrelated work:
     a. Type /mcp in the Claude Code prompt.
     b. In the MCP server list, select "anthara-fabric".
     c. Choose "Authenticate" — your browser opens the sign-in page.
     d. Sign in with your work account and approve the access request.
     e. Return to the terminal; anthara-fabric should now show as connected.
     f. Send your next prompt — this gate goes silent once the sign-in exists.
4. Do NOT call any other anthara-fabric MCP tool while unauthenticated, and do NOT start work that depends on fabric (memory, standards, compliance packs). Unrelated work may continue once this turn's question is answered — but a "Not now" answer never carries over to later turns; re-ask on every prompt until the sign-in exists.
This gate re-fires on every prompt until the sign-in exists, then goes silent.
EOF
}

emit_disabled_gate() {
  local url="$1"
  cat <<EOF
MANDATORY — FABRIC DISABLED GATE: The anthara-fabric MCP server (${url}) is DISABLED for this project, so memory, standards, and audit logs are not being captured. Anthara requires it to stay enabled.
1. FIRST, before anything else this turn, tell the user fabric is disabled and how to re-enable it: type /mcp, select "anthara-fabric", choose "Enable". Repeat this notice on every prompt until it is enabled — a previous acknowledgement does NOT suppress it.
2. Do NOT call any anthara-fabric MCP tool (none are loaded while disabled), and do NOT start work that depends on fabric (memory, standards, compliance packs). Unrelated work may continue after the notice.
This gate re-fires on every prompt until the server is enabled, then goes silent.
EOF
}

emit_verification_rubric() {
  cat <<'EOF'
FABRIC AUTH CHECK (indeterminate): The anthara-fabric MCP OAuth state could not be verified on this platform. When this session first needs fabric (memory, standards, packs), if its MCP tools are missing or fail with an auth error, immediately instruct the user, offering both paths: (a) ask Claude to connect — Claude then calls mcp__anthara-fabric__authenticate, shares the sign-in URL, and completes with mcp__anthara-fabric__complete_authentication; or (b) manually run /mcp, select anthara-fabric, choose "Authenticate" (one-time browser sign-in). Do not silently degrade to working without fabric.
EOF
}

sign_in_reason() {
  cat <<EOF
Anthara Fabric sign-in required — your prompt was not sent.

  Server: ${1}

  1. Type /mcp
  2. Select "anthara-fabric", then choose "Authenticate"
  3. Sign in with your work account in the browser
  4. Resubmit your prompt

You only sign in once — after that, your memory and standards are ready to use in every session.
EOF
}

DISABLED_REASON=$(cat <<'EOF'
Anthara Fabric is disabled in this project — your prompt was not sent.

  1. Type /mcp
  2. Select "anthara-fabric", then choose "Enable"
  3. Resubmit your prompt

Anthara requires the fabric connection so memory, standards, and audit logs stay attributed to you.
EOF
)

json_escape() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\r'/\\r}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\t'/\\t}"
  printf '%s' "$s"
}

emit_prompt_block() {
  local reason="$1" block=""
  case "$JSON_TOOL" in
    jq)
      block=$(jq -cn --arg reason "$reason" '{decision: "block", reason: $reason}' 2>/dev/null)
      ;;
    python3|python)
      block=$("$JSON_TOOL" -c \
        "import json, sys; print(json.dumps({'decision': 'block', 'reason': sys.argv[1]}))" \
        "$reason" 2>/dev/null)
      ;;
    node)
      block=$(node -e 'console.log(JSON.stringify({decision: "block", reason: process.argv[1]}))' "$reason" 2>/dev/null)
      ;;
  esac
  [ -n "$block" ] || block="{\"decision\": \"block\", \"reason\": \"$(json_escape "$reason")\"}"
  printf '%s\n' "$block"
}

EVENT=$(input_field hook_event_name)

if [ -z "$JSON_TOOL" ]; then
  [ "$EVENT" = "SessionStart" ] && emit_verification_rubric
  exit 0
fi

FABRIC_URL=$(fabric_url) || exit 0
FABRIC_URL=$(expand_placeholders "$FABRIC_URL") || exit 0

if server_disabled; then
  if [ "$EVENT" = "UserPromptSubmit" ]; then
    emit_prompt_block "$DISABLED_REASON"
  else
    emit_disabled_gate "$FABRIC_URL"
  fi
  exit 0
fi

case "$(auth_state "$FABRIC_URL")" in
  authenticated)
    exit 0
    ;;
  unauthenticated)
    if [ "$EVENT" = "UserPromptSubmit" ]; then
      emit_prompt_block "$(sign_in_reason "$FABRIC_URL")"
      exit 0
    fi
    emit_sign_in_gate "$FABRIC_URL"
    ;;
  indeterminate)
    [ "$EVENT" = "SessionStart" ] && emit_verification_rubric
    ;;
esac

exit 0
