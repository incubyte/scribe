#!/usr/bin/env bash

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

access_token() {
  local url="$1" creds
  creds=$(credentials_json) || return 1
  case "$JSON_TOOL" in
    jq)
      printf '%s' "$creds" | jq -er --arg url "$url" --arg name "$SERVER_NAME" '
        .mcpOAuth // {} | to_entries
        | map(select(
            ((.key | split("|")[0] | sub("^plugin:[^:]*:"; "")) == $name)
            and .value.serverUrl == $url))
        | .[0].value.accessToken // empty
      ' 2>/dev/null
      ;;
    python3|python)
      printf '%s' "$creds" | "$JSON_TOOL" -c "
import json, re, sys
data = json.load(sys.stdin).get('mcpOAuth', {})
url, name = sys.argv[1], sys.argv[2]
for key, value in data.items():
    if re.sub(r'^plugin:[^:]*:', '', key.split('|')[0]) == name and value.get('serverUrl') == url:
        token = value.get('accessToken')
        if token:
            print(token)
            sys.exit(0)
sys.exit(1)
" "$url" "$SERVER_NAME" 2>/dev/null
      ;;
    node)
      printf '%s' "$creds" | node -e '
const [url, name] = process.argv.slice(1);
try {
  const data = JSON.parse(require("fs").readFileSync(0, "utf8")).mcpOAuth || {};
  const match = Object.entries(data).find(([key, value]) =>
    key.split("|")[0].replace(/^plugin:[^:]*:/, "") === name && value.serverUrl === url);
  const token = match && match[1].accessToken;
  if (!token) process.exit(1);
  console.log(token);
} catch { process.exit(1); }
' "$url" "$SERVER_NAME" 2>/dev/null
      ;;
  esac
}

telemetry_url() {
  local url="$1" suffix="$2"
  [[ "$url" =~ ^(.*)/mcp/([^/]+)$ ]] || return 1
  printf '%s\n' "${BASH_REMATCH[1]}/telemetry/${BASH_REMATCH[2]}/${suffix}"
}
