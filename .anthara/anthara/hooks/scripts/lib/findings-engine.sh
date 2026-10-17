#!/usr/bin/env bash

FINDINGS_ENGINE_DIR="${FINDINGS_HOOKS_DIR}/findings"

engine_runs_jq() {
  command -v jq >/dev/null 2>&1 &&
    [ "$(jq -n -r --rawfile probe /dev/null '"ok"' 2>/dev/null | tr -d '\r')" = "ok" ]
}

engine_runs_node() {
  command -v node >/dev/null 2>&1 &&
    [ "$(node -e 'process.stdout.write("ok")' 2>/dev/null)" = "ok" ]
}

detect_findings_engine() {
  if engine_runs_jq; then
    printf 'jq'
  elif engine_runs_node; then
    printf 'node'
  else
    return 1
  fi
}

input_file_or_empty() {
  if [ -n "${1:-}" ] && [ -f "$1" ] && [ -r "$1" ]; then printf '%s' "$1"; else printf '/dev/null'; fi
}

run_jq_engine() {
  jq -n -r -f "$FINDINGS_ENGINE_DIR/engine.jq" \
    --arg op "$1" --arg session_id "${ENGINE_SESSION_ID:-}" --arg project "${PROJECT_DIR:-.}" \
    --arg repo_root "${REPO_ROOT:-.}" --arg ledger_path "${LEDGER_PATH:-}" \
    --rawfile payload "$PAYLOAD_FILE" --rawfile ledger "$2" --rawfile copy "$3" \
    --rawfile task_map "$4" --rawfile transcript "$5" --rawfile changed "$6"
}

run_node_engine() {
  node "$FINDINGS_ENGINE_DIR/engine.js" "$1" \
    "${ENGINE_SESSION_ID:-}" "${PROJECT_DIR:-.}" "${REPO_ROOT:-.}" "${LEDGER_PATH:-}" \
    "$PAYLOAD_FILE" "$2" "$3" "$4" "$5" "$6"
}

run_engine_operation() {
  "run_${FINDINGS_ENGINE}_engine" "$1" \
    "$(input_file_or_empty "${ENGINE_IN_LEDGER:-}")" \
    "$(input_file_or_empty "${ENGINE_IN_COPY:-}")" \
    "$(input_file_or_empty "${ENGINE_IN_TASK_MAP:-}")" \
    "$(input_file_or_empty "${ENGINE_IN_TRANSCRIPT:-}")" \
    "$(input_file_or_empty "${ENGINE_IN_CHANGED:-}")"
}

engine() {
  local operation="$1" output="$WORK_DIR/engine-$1.sh"
  unset ENGINE_UNCLAIMED ENGINE_RESPONSE ENGINE_LEDGER ENGINE_POINT ENGINE_TASK_MAP ENGINE_SNAPSHOT ENGINE_SNAPSHOT_RUN ENGINE_SNAPSHOT_SEQ ENGINE_LOG ENGINE_PARENT_ID
  run_engine_operation "$operation" 2>/dev/null | tr -d '\r' > "$output" || return 1
  . "$output"
}

pretty_json() {
  case "$FINDINGS_ENGINE" in
    jq) jq . | tr -d '\r' ;;
    node) node -e 'const fs = require("fs"); process.stdout.write(JSON.stringify(JSON.parse(fs.readFileSync(0, "utf8")), null, 2) + "\n")' ;;
  esac
}
