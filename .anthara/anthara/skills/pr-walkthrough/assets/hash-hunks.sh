#!/usr/bin/env bash
# hash-hunks.sh — the versioned hunk hasher for /anthara:pr-walkthrough (spec 003, slice 5.1).
#
# Reads a unified diff and emits one line per hunk:
#   <file path> TAB <hunk index within file> TAB <content hash, 12 hex chars>
#
# The hash covers the hunk BODY (context + changed lines), not the @@ header,
# so a hunk whose position shifts but whose content is unchanged keeps its
# hash across pushes — that is what lets verdicts carry forward on delta
# re-review (invariant 4.11).
#
# Usage:
#   gh pr diff <n> > pr.diff
#   bash hash-hunks.sh pr.diff
#   bash hash-hunks.sh -          # read the diff from stdin
set -euo pipefail

diff_input="${1:--}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

awk -v out="$tmp" '
  /^diff --git a\// {
    file = substr($0, 14)
    sub(/ b\/.*/, "", file)
    idx = 0
    body = ""
    next
  }
  /^@@/ && file != "" {
    idx++
    n++
    printf "%d\t%s\t%d\n", n, file, idx >> (out "/index")
    body = out "/" n ".hunk"
    printf "" > body
    next
  }
  body != "" && /^[-+ \\]/ { print >> body }
' "$diff_input"

[ -f "$tmp/index" ] || exit 0

while IFS=$'\t' read -r n file idx; do
  hash="$(shasum -a 256 "$tmp/$n.hunk" | cut -c1-12)"
  printf '%s\t%s\t%s\n' "$file" "$idx" "$hash"
done < "$tmp/index"
