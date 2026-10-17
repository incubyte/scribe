#!/usr/bin/env bash
set -uo pipefail

MAX_PAYLOAD_BYTES=10485760
SCRIPT_DIR="${BASH_SOURCE[0]%/*}"
FINDINGS_HOOKS_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
FORKED_SESSION="fork"
SWITCHED_SESSION="resume"
ABANDONED_AFTER_MINUTES=1440
ACTIVE_LEDGER_MARKER='"state": "active"'

source "$SCRIPT_DIR/lib/findings-engine.sh"
source "$SCRIPT_DIR/lib/findings-state.sh"

INPUT=$(head -c "$((MAX_PAYLOAD_BYTES + 1))")
[ "${#INPUT}" -le "$MAX_PAYLOAD_BYTES" ] || exit 0

FINDINGS_ENGINE=$(detect_findings_engine) || exit 0
WORK_DIR=$(mktemp -d 2>/dev/null) || exit 0
trap 'unlock_ledger; rm -rf "$WORK_DIR"' EXIT
PAYLOAD_FILE="$WORK_DIR/payload.json"
printf '%s' "$INPUT" > "$PAYLOAD_FILE"

engine fields || exit 0
SESSION_ID="${FIELD_SESSION_ID:-}"
[ -n "$SESSION_ID" ] || exit 0
ENGINE_SESSION_ID="$SESSION_ID"
PROJECT_DIR="${FIELD_CWD:-${CLAUDE_PROJECT_DIR:-.}}"
[ -n "$PROJECT_DIR" ] || PROJECT_DIR="."
REPO_ROOT="${CLAUDE_PROJECT_DIR:-${FIELD_CWD:-.}}"
[ -n "$REPO_ROOT" ] || REPO_ROOT="."

read_own_run() {
  ENGINE_IN_LEDGER="$LEDGER_PATH"
  ENGINE_IN_COPY=$(ledger_copy_file "$SESSION_ID")
  ENGINE_IN_TASK_MAP=$(task_map_file "$SESSION_ID")
}

incident_path() {
  local root
  case "$1" in
    /*) [ -e "$1" ] && printf '%s' "$1" ;;
    *)
      for root in "$REPO_ROOT" "$PROJECT_DIR"; do
        [ -e "$root/$1" ] && { printf '%s' "$root/$1"; return 0; }
      done
      return 1 ;;
  esac
}

modified_since() { [ -n "$(find "$1" -maxdepth 0 -newermt "$2" 2>/dev/null)" ]; }

note_changed_files() {
  local id seen file path
  ENGINE_IN_CHANGED="$WORK_DIR/changed.txt"
  : > "$ENGINE_IN_CHANGED"
  engine unclaimed-files || return 0
  while IFS=$'\t' read -r id seen file; do
    [ -n "$id" ] || continue
    path=$(incident_path "$file") || continue
    modified_since "$path" "$seen" && printf '%s\n' "$id" >> "$ENGINE_IN_CHANGED"
  done <<< "${ENGINE_UNCLAIMED:-}"
}

reads_review_pass() { [ "$1" = review-stop ] || [ "$1" = hand-back ]; }

locked_operation() {
  local handler="$1" operation="$2"
  lock_ledger "$LEDGER_PATH" || { log_failure "$handler" lock-timeout; return 0; }
  read_own_run
  reads_review_pass "$operation" && note_changed_files
  if engine "$operation"; then
    apply_engine_outputs "$handler" "$LEDGER_PATH" "$SESSION_ID"
  else
    log_failure "$handler" engine-failed
  fi
  unlock_ledger
}

brief_review_agent() {
  [ -n "${FIELD_IS_REVIEW_SUBAGENT:-}" ] || return 0
  LEDGER_PATH=$(pointed_ledger "$SESSION_ID") || return 0
  read_own_run
  if engine brief; then
    apply_engine_outputs brief "$LEDGER_PATH" "$SESSION_ID"
  else
    log_failure brief engine-failed
  fi
}

handle_review_stop() {
  [ -n "${FIELD_IS_REVIEW_AGENT:-}" ] || return 0
  LEDGER_PATH=$(ledger_path_for "$SESSION_ID" "$REPO_ROOT")
  ENGINE_IN_TRANSCRIPT="${FIELD_AGENT_TRANSCRIPT:-}"
  locked_operation review-stop review-stop
}

hand_back_review_todos() {
  [ -n "${FIELD_IS_FINISHED_REVIEW:-}" ] || return 0
  LEDGER_PATH=$(ledger_path_for "$SESSION_ID" "$REPO_ROOT")
  locked_operation hand-back hand-back
}

track_task_event() {
  case "${FIELD_TOOL_NAME:-}" in TaskCreate|TaskUpdate) ;; *) return 0 ;; esac
  LEDGER_PATH=$(pointed_ledger "$SESSION_ID") || return 0
  locked_operation track track
}

close_on_stop() {
  [ -z "${FIELD_BACKGROUND_RUNNING:-}" ] || return 0
  LEDGER_PATH=$(pointed_ledger "$SESSION_ID") || return 0
  locked_operation close close
}

close_on_session_end() {
  [ "${FIELD_SESSION_EVENT:-}" != "$SWITCHED_SESSION" ] || return 0
  LEDGER_PATH=$(pointed_ledger "$SESSION_ID") || return 0
  locked_operation end end
}

carry_on_parent_run() {
  local parent
  ENGINE_IN_TRANSCRIPT="${FIELD_SESSION_TRANSCRIPT:-}"
  engine fork-parent || return 0
  parent="${ENGINE_PARENT_ID:-}"
  [ -n "$parent" ] || return 0
  LEDGER_PATH=$(pointed_ledger "$parent") || return 0
  lock_ledger "$LEDGER_PATH" || { log_failure start lock-timeout; return 0; }
  ENGINE_IN_LEDGER="$LEDGER_PATH"
  ENGINE_IN_COPY=$(ledger_copy_file "$parent")
  ENGINE_IN_TASK_MAP=$(task_map_file "$parent")
  if engine adopt; then
    apply_engine_outputs start "$LEDGER_PATH" "$SESSION_ID"
  else
    log_failure start engine-failed
  fi
  unlock_ledger
}

recently_active() {
  [ -n "$(find "$@" -maxdepth 0 -mmin "-$ABANDONED_AFTER_MINUTES" 2>/dev/null)" ]
}

possibly_active_sessions() {
  local directory owner copy
  for directory in "$STATE_ROOT"/*/; do
    owner="${directory%/}"
    owner="${owner##*/}"
    [ "$owner" = "$SESSION_ID" ] || [ "$owner" = "locks" ] && continue
    [ -f "$(pointer_file "$owner")" ] || continue
    copy=$(ledger_copy_file "$owner")
    if [ ! -f "$copy" ] || grep -qF "$ACTIVE_LEDGER_MARKER" "$copy" 2>/dev/null; then
      printf '%s\n' "$owner"
    fi
  done
}

close_run_of() {
  local owner="$1" ledger_path existing=()
  ledger_path=$(pointed_ledger "$owner") || return 0
  [ -e "$ledger_path" ] && existing+=("$ledger_path")
  [ -e "$(ledger_copy_file "$owner")" ] && existing+=("$(ledger_copy_file "$owner")")
  [ "${#existing[@]}" -gt 0 ] && recently_active "${existing[@]}" && return 0
  lock_ledger "$ledger_path" || { log_failure start lock-timeout; return 0; }
  ENGINE_IN_LEDGER="$ledger_path"
  ENGINE_IN_COPY=$(ledger_copy_file "$owner")
  ENGINE_SESSION_ID="$owner"
  if engine sweep; then
    apply_engine_outputs start "$ledger_path" "$owner"
  else
    log_failure start engine-failed
  fi
  ENGINE_SESSION_ID="$SESSION_ID"
  unlock_ledger
}

close_abandoned_runs() {
  local owner
  [ -d "$STATE_ROOT" ] || return 0
  shopt -s nullglob
  for owner in $(possibly_active_sessions); do
    close_run_of "$owner"
  done
}

on_session_start() {
  [ "${FIELD_SESSION_EVENT:-}" = "$FORKED_SESSION" ] && carry_on_parent_run
  close_abandoned_runs
  resend_outbox "$(spool_dir "$SESSION_ID")"
}

case "${1:-}" in
  brief) brief_review_agent ;;
  review-stop) handle_review_stop ;;
  hand-back) hand_back_review_todos ;;
  track) track_task_event ;;
  close) close_on_stop ;;
  end) close_on_session_end ;;
  start) on_session_start ;;
esac
exit 0
