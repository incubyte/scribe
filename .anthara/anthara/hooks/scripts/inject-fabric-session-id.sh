#!/usr/bin/env bash
set -uo pipefail

NODE_PROGRAM='
const MAX_PAYLOAD_BYTES = 10485760;
const UUID = /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;
let raw = "";
process.stdin.on("data", (chunk) => {
  raw += chunk;
  if (raw.length > MAX_PAYLOAD_BYTES) process.exit(0);
});
process.stdin.on("end", () => {
  let payload;
  try {
    payload = JSON.parse(raw);
  } catch {
    process.exit(0);
  }
  const sessionId = typeof payload.session_id === "string" ? payload.session_id.trim() : "";
  const toolInput = payload.tool_input;
  const isPlainObject =
    typeof toolInput === "object" && toolInput !== null && !Array.isArray(toolInput);
  if (!UUID.test(sessionId) || !isPlainObject) process.exit(0);
  console.log(
    JSON.stringify({
      hookSpecificOutput: {
        hookEventName: "PreToolUse",
        updatedInput: { ...toolInput, anthara_session_id: sessionId },
      },
    })
  );
});
'

PYTHON_PROGRAM='
import json, re, sys

MAX_PAYLOAD_BYTES = 10485760
UUID = r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}"

raw = sys.stdin.read(MAX_PAYLOAD_BYTES + 1)
if len(raw) > MAX_PAYLOAD_BYTES:
    sys.exit(0)
try:
    payload = json.loads(raw)
except ValueError:
    sys.exit(0)
if not isinstance(payload, dict):
    sys.exit(0)
session_id = payload.get("session_id")
tool_input = payload.get("tool_input")
if not isinstance(session_id, str) or not isinstance(tool_input, dict):
    sys.exit(0)
session_id = session_id.strip()
if not re.fullmatch(UUID, session_id):
    sys.exit(0)
tool_input["anthara_session_id"] = session_id
print(json.dumps({
    "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "updatedInput": tool_input,
    },
}))
'

PERL_PROGRAM='
use strict;
use JSON::PP;
my $max_payload_bytes = 10485760;
my $raw = do { local $/; <STDIN> };
exit 0 if !defined $raw || length($raw) > $max_payload_bytes;
my $payload = eval { decode_json($raw) };
exit 0 unless ref $payload eq "HASH";
my $session_id = $payload->{session_id};
exit 0 if !defined $session_id || ref $session_id;
$session_id =~ s/^\s+|\s+$//g;
exit 0 unless $session_id =~ /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;
my $tool_input = $payload->{tool_input};
exit 0 unless ref $tool_input eq "HASH";
$tool_input->{anthara_session_id} = $session_id;
print encode_json({
  hookSpecificOutput => {
    hookEventName => "PreToolUse",
    updatedInput  => $tool_input,
  },
});
'

AWK_PROGRAM='
function is_uuid(s,  hex) {
  if (length(s) != 36) return 0
  if (substr(s, 9, 1) != "-") return 0
  if (substr(s, 14, 1) != "-") return 0
  if (substr(s, 19, 1) != "-") return 0
  if (substr(s, 24, 1) != "-") return 0
  hex = s
  gsub(/-/, "", hex)
  return length(hex) == 32 && hex ~ /^[0-9a-fA-F]+$/
}

{ doc = doc $0 "\n"; if (length(doc) > 10485760) { oversized = 1; exit } }

END {
  if (oversized) exit 0
  depth = 0; in_string = 0; escaped = 0
  awaiting_colon = 0; in_value = 0; key = ""; candidate = ""
  sid = ""; sid_found = 0; ti_start = 0; ti_end = 0
  n = length(doc)
  for (i = 1; i <= n; i++) {
    c = substr(doc, i, 1)
    if (in_string) {
      if (escaped) escaped = 0
      else if (c == "\\") escaped = 1
      else if (c == "\"") {
        in_string = 0
        content = substr(doc, string_start + 1, i - string_start - 1)
        if (in_value && depth == 1) {
          if (key == "session_id" && !sid_found) { sid = content; sid_found = 1 }
          in_value = 0
        } else if (depth == 1) {
          candidate = content
          awaiting_colon = 1
        }
      }
      continue
    }
    if (c == "\"") { in_string = 1; string_start = i; continue }
    if (c == ":" && awaiting_colon && depth == 1) {
      key = candidate; awaiting_colon = 0; in_value = 1; continue
    }
    if (c == "{") {
      if (in_value && depth == 1 && key == "tool_input" && !ti_start) ti_start = i
      depth++
      continue
    }
    if (c == "}") {
      depth--
      if (ti_start && !ti_end && depth == 1) { ti_end = i; in_value = 0 }
      continue
    }
    if (c == "," && depth == 1) { in_value = 0; awaiting_colon = 0 }
  }
  if (!sid_found || !is_uuid(sid) || !ti_start || !ti_end || depth != 0 || in_string) exit 0
  body = substr(doc, ti_start, ti_end - ti_start)
  inner = substr(doc, ti_start + 1, ti_end - ti_start - 1)
  separator = (inner ~ /^[ \t\r\n]*$/) ? "" : ","
  injected = body separator "\"anthara_session_id\":\"" sid "\"}"
  printf "%s%s%s\n", "{\"hookSpecificOutput\":{\"hookEventName\":\"PreToolUse\",\"updatedInput\":", injected, "}}"
}
'

runtime_works() {
  command -v "$1" >/dev/null 2>&1 && "$@" </dev/null >/dev/null 2>&1
}

if runtime_works node -e 1; then
  exec node -e "$NODE_PROGRAM" 2>/dev/null
fi
if runtime_works python3 -c pass; then
  exec python3 -c "$PYTHON_PROGRAM" 2>/dev/null
fi
if runtime_works perl -MJSON::PP -e 1; then
  exec perl -e "$PERL_PROGRAM" 2>/dev/null
fi
if runtime_works awk 'BEGIN { exit 0 }'; then
  exec awk "$AWK_PROGRAM" 2>/dev/null
fi
exit 0
