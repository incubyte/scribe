#!/usr/bin/env bash
set -euo pipefail

workspace_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
host_tuple="$(rustc --print host-tuple)"
target_tuple="${1:-$host_tuple}"

extension=""
case "$target_tuple" in
  *windows*) extension=".exe" ;;
esac

if [ "$target_tuple" = "$host_tuple" ]; then
  cargo build --release -p scribe-mcp --manifest-path "$workspace_root/Cargo.toml"
  built="$workspace_root/target/release/scribe-mcp$extension"
else
  cargo xwin build --release -p scribe-mcp --target "$target_tuple" --manifest-path "$workspace_root/Cargo.toml"
  built="$workspace_root/target/$target_tuple/release/scribe-mcp$extension"
fi

destination="$workspace_root/apps/scribe-tray/src-tauri/binaries"
mkdir -p "$destination"
cp "$built" "$destination/scribe-mcp-$target_tuple$extension"
