#!/usr/bin/env bash
set -euo pipefail

resources="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/src-tauri/resources"
redist="$resources/vc_redist.x64.exe"

mkdir -p "$resources"
if [ ! -s "$redist" ]; then
  curl --fail --location --proto '=https' --tlsv1.2 --silent --show-error \
    --output "$redist" "https://aka.ms/vs/17/release/vc_redist.x64.exe"
fi
