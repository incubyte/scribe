#!/usr/bin/env bash
set -euo pipefail

workspace_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
host_tuple="$(rustc --print host-tuple)"
extension=""
case "$host_tuple" in
  *windows*) extension=".exe" ;;
esac

cargo build --release -p scribe-mcp --manifest-path "$workspace_root/Cargo.toml"

destination="$workspace_root/apps/scribe-tray/src-tauri/binaries"
mkdir -p "$destination"
cp "$workspace_root/target/release/scribe-mcp$extension" "$destination/scribe-mcp-$host_tuple$extension"
