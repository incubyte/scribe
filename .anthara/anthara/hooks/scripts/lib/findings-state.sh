#!/usr/bin/env bash

FINDINGS_DIR_NAME=".anthara/findings"
UNTRACKED_FOLDER_IGNORE='*'
LOCK_WAIT_SECONDS=2
LOCK_POLL_SECONDS=0.02
LOCK_POLLS=$(( LOCK_WAIT_SECONDS * 50 ))
STATE_ROOT="${CLAUDE_CONFIG_DIR:-${HOME}/.claude}/anthara-findings"
LOCKS_DIR="$STATE_ROOT/locks"
HOOK_LOG="$STATE_ROOT/hook.log"
HELD_LOCK=""

pointer_file() { printf '%s' "$STATE_ROOT/$1/ledger-pointer"; }
ledger_copy_file() { printf '%s' "$STATE_ROOT/$1/ledger.json"; }
task_map_file() { printf '%s' "$STATE_ROOT/$1/task-map.json"; }
spool_dir() { printf '%s' "$STATE_ROOT/$1/spool"; }

utc_now() { date -u +%Y-%m-%dT%H:%M:%SZ; }

log_failure() {
  mkdir -p "$STATE_ROOT" 2>/dev/null &&
    printf '%s %s %s\n' "$(utc_now)" "$1" "$2" >> "$HOOK_LOG" 2>/dev/null
  return 0
}

log_engine_events() {
  local event
  [ -n "${ENGINE_LOG:-}" ] || return 0
  while IFS= read -r event; do
    [ -n "$event" ] && log_failure "${event%% *}" "${event#* }"
  done <<< "$ENGINE_LOG"
}

pointed_ledger() {
  local pointer
  pointer=$(pointer_file "$1")
  [ -f "$pointer" ] || return 1
  local path
  path=$(cat "$pointer" 2>/dev/null) || return 1
  path="${path%"${path##*[![:space:]]}"}"
  [ -n "$path" ] || return 1
  printf '%s' "$path"
}

without_trailing_slashes() {
  local path="$1"
  while [ "${#path}" -gt 1 ] && [ "${path%/}" != "$path" ]; do path="${path%/}"; done
  printf '%s' "$path"
}

findings_dir_of() {
  local root
  root=$(without_trailing_slashes "$1")
  if [ "$root" = "." ] || [ -z "$root" ]; then printf '%s' "$FINDINGS_DIR_NAME"; else printf '%s/%s' "$root" "$FINDINGS_DIR_NAME"; fi
}

session_ledger_path() {
  local directory prefix started
  directory=$(findings_dir_of "$1")
  prefix="${2:0:8}"
  started=$(LC_ALL=C; for candidate in "$directory"/*-"$prefix".json; do [ -e "$candidate" ] && { printf '%s' "$candidate"; break; }; done)
  if [ -n "$started" ]; then
    printf '%s' "$started"
  else
    printf '%s/%s-%s.json' "$directory" "$(date -u +%Y-%m-%d)" "$prefix"
  fi
}

ledger_path_for() {
  pointed_ledger "$1" || session_ledger_path "$2" "$1"
}

write_atomically() {
  local path="$1" directory temporary
  directory="${path%/*}"
  [ "$directory" = "$path" ] && directory="."
  mkdir -p "$directory" 2>/dev/null || return 1
  temporary=$(mktemp "$directory/.${path##*/}.XXXXXX" 2>/dev/null) || return 1
  if cat > "$temporary" 2>/dev/null && mv -f "$temporary" "$path" 2>/dev/null; then
    return 0
  fi
  rm -f "$temporary" 2>/dev/null
  return 1
}

keep_out_of_git() {
  [ -e "$1/.gitignore" ] || printf '%s\n' "$UNTRACKED_FOLDER_IGNORE" | write_atomically "$1/.gitignore"
}

absolute_path() {
  case "$1" in
    /*|[A-Za-z]:*) printf '%s' "$1" ;;
    *) printf '%s/%s' "$PWD" "$1" ;;
  esac
}

lock_file_for() {
  local key
  key=$(absolute_path "$1" | cksum | tr -s ' ' '-')
  printf '%s/%s.lock' "$LOCKS_DIR" "$key"
}

lock_ledger() {
  local file
  file=$(lock_file_for "$1")
  mkdir -p "$LOCKS_DIR" 2>/dev/null || return 1
  if command -v flock >/dev/null 2>&1; then
    exec 9>>"$file" || return 1
    flock -w "$LOCK_WAIT_SECONDS" 9 || { exec 9>&-; return 1; }
    HELD_LOCK="flock"
  else
    lock_directory "$file.d" || return 1
    HELD_LOCK="$file.d"
  fi
}

lock_directory() {
  local directory="$1" poll holder
  for (( poll = 0; poll < LOCK_POLLS; poll++ )); do
    if mkdir "$directory" 2>/dev/null; then
      printf '%s' "$$" > "$directory/pid"
      return 0
    fi
    holder=$(cat "$directory/pid" 2>/dev/null)
    if [ -n "$holder" ] && ! kill -0 "$holder" 2>/dev/null; then
      mv "$directory" "$directory.stale.$$" 2>/dev/null && rm -rf "$directory.stale.$$"
      continue
    fi
    sleep "$LOCK_POLL_SECONDS"
  done
  return 1
}

unlock_ledger() {
  case "$HELD_LOCK" in
    "") ;;
    flock) exec 9>&- ;;
    *) [ "$(cat "$HELD_LOCK/pid" 2>/dev/null)" = "$$" ] && rm -rf "$HELD_LOCK" ;;
  esac
  HELD_LOCK=""
}

save_run() {
  local ledger_path="$1" copy_path="$2" pretty
  pretty=$(printf '%s' "$ENGINE_LEDGER" | pretty_json) || return 1
  printf '%s\n' "$pretty" | write_atomically "$ledger_path" || return 1
  keep_out_of_git "${ledger_path%/*}" || return 1
  printf '%s\n' "$pretty" | write_atomically "$copy_path"
}

launch_send() {
  local script="${CLAUDE_PLUGIN_ROOT:-${FINDINGS_HOOKS_DIR}/..}/hooks/scripts/send-findings.sh"
  if command -v setsid >/dev/null 2>&1; then
    setsid bash "$script" "$1" </dev/null >/dev/null 2>&1 9>&- &
  else
    bash "$script" "$1" </dev/null >/dev/null 2>&1 9>&- &
  fi
}

publish_snapshot() {
  local spool="$1" snapshot superseded
  [ -n "${ENGINE_SNAPSHOT:-}" ] || return 0
  snapshot="$spool/$ENGINE_SNAPSHOT_RUN.$ENGINE_SNAPSHOT_SEQ.json"
  if ! printf '%s' "$ENGINE_SNAPSHOT" | write_atomically "$snapshot"; then
    log_failure publish write-failed
    return 0
  fi
  for superseded in "$spool/$ENGINE_SNAPSHOT_RUN".*.json; do
    [ -e "$superseded" ] && [ "$superseded" != "$snapshot" ] && rm -f "$superseded" "$superseded.dest"
  done
  launch_send "$snapshot"
}

run_of_snapshot() {
  local name="${1##*/}"
  printf '%s' "${name%%.*}"
}

resend_outbox() {
  local spool="$1" index file run sent_runs=" " LC_COLLATE=C
  local snapshots=("$spool"/*.json)
  [ "${#snapshots[@]}" -gt 0 ] && [ -e "${snapshots[0]}" ] || return 0
  for (( index = ${#snapshots[@]} - 1; index >= 0; index-- )); do
    file="${snapshots[index]}"
    run=$(run_of_snapshot "$file")
    if [[ "$sent_runs" == *" $run "* ]]; then
      rm -f "$file" "$file.dest"
    else
      sent_runs+="$run "
      launch_send "$file"
    fi
  done
}

apply_engine_outputs() {
  local handler="$1" ledger_path="$2" owner="$3"
  log_engine_events
  if [ -n "${ENGINE_LEDGER:-}" ]; then
    save_run "$ledger_path" "$(ledger_copy_file "$owner")" || { log_failure "$handler" write-failed; return 1; }
  fi
  if [ -n "${ENGINE_POINT:-}" ]; then
    printf '%s' "$ledger_path" | write_atomically "$(pointer_file "$owner")" || { log_failure "$handler" write-failed; return 1; }
  fi
  if [ -n "${ENGINE_TASK_MAP:-}" ]; then
    printf '%s' "$ENGINE_TASK_MAP" | pretty_json | write_atomically "$(task_map_file "$owner")" || { log_failure "$handler" write-failed; return 1; }
  fi
  [ -n "${ENGINE_LEDGER:-}" ] && publish_snapshot "$(spool_dir "$owner")"
  [ -n "${ENGINE_RESPONSE:-}" ] && printf '%s\n' "$ENGINE_RESPONSE"
  return 0
}
