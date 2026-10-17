#!/usr/bin/env bash

set -uo pipefail

FILE="${1:-}"
[ -n "$FILE" ] && [ -f "$FILE" ] || exit 0

SCRIPT_DIR="${BASH_SOURCE[0]%/*}"
source "${SCRIPT_DIR}/lib/fabric-client.sh"

PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-${SCRIPT_DIR}/../..}"
CREDENTIALS_FILE="${CLAUDE_CONFIG_DIR:-${HOME}/.claude}/.credentials.json"
SERVER_NAME="anthara-fabric"

SPOOL_DIR="${FILE%/*}"
SESSION_DIR="${SPOOL_DIR%/*}"
SESSION_ID="${SESSION_DIR##*/}"

input_field() {
  [ "$1" = "session_id" ] && printf '%s' "$SESSION_ID"
}

JSON_TOOL=$(detect_json_tool)
[ -n "$JSON_TOOL" ] || exit 0

DEST_FILE="${FILE}.dest"
if [ -s "$DEST_FILE" ]; then
  FABRIC_URL=$(cat "$DEST_FILE")
else
  FABRIC_URL=$(fabric_url) || exit 0
  FABRIC_URL=$(expand_placeholders "$FABRIC_URL") || exit 0
  printf '%s' "$FABRIC_URL" > "$DEST_FILE.$$" && mv -f "$DEST_FILE.$$" "$DEST_FILE"
fi

TOKEN=$(access_token "$FABRIC_URL") || exit 0
[ -n "$TOKEN" ] || exit 0

TELEMETRY_URL=$(telemetry_url "$FABRIC_URL" "review-findings") || exit 0

response_settles_snapshot() {
  case "$1" in
    401|403|408|429) return 1 ;;
    2??|4??) return 0 ;;
    *) return 1 ;;
  esac
}

HTTP_STATUS=$(printf 'header = "Authorization: Bearer %s"\n' "$TOKEN" | curl --config - --max-time 10 -sS \
  -o /dev/null -w '%{http_code}' -X POST \
  -H 'Content-Type: application/json' \
  --data-binary "@$FILE" \
  "$TELEMETRY_URL" 2>/dev/null)

response_settles_snapshot "$HTTP_STATUS" && rm -f "$FILE" "$DEST_FILE"

exit 0
