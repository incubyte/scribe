#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tray="$root/apps/scribe-tray"
windows_target="x86_64-pc-windows-msvc"
version="$(jq -r .version "$tray/src-tauri/tauri.conf.json")"
tag="v$version"
export PATH="/opt/homebrew/opt/llvm/bin:/opt/homebrew/opt/lld/bin:$PATH"

require_clean_main() {
  if [ -n "$(git -C "$root" status --porcelain)" ]; then
    echo "Commit or stash your changes before releasing." >&2
    exit 1
  fi
  if [ "$(git -C "$root" rev-parse --abbrev-ref HEAD)" != "main" ]; then
    echo "Release from main." >&2
    exit 1
  fi
}

build_macos() {
  bash "$tray/scripts/stage-sidecar.sh"
  (cd "$tray" && npx tauri build --bundles dmg --config src-tauri/tauri.ci.conf.json)
}

build_windows() {
  bash "$tray/scripts/stage-sidecar.sh" "$windows_target"
  (cd "$tray" && npx tauri build --runner cargo-xwin --target "$windows_target" --bundles nsis \
    --config src-tauri/tauri.ci.conf.json)
}

tag_release() {
  if ! git -C "$root" rev-parse -q --verify "refs/tags/$tag" > /dev/null; then
    git -C "$root" tag "$tag"
  fi
  git -C "$root" push origin "$tag"
}

publish_assets() {
  local dmg="$root/target/release/bundle/dmg/Scribe_${version}_aarch64.dmg"
  local installer="$root/target/$windows_target/release/bundle/nsis/Scribe_${version}_x64-setup.exe"
  if ! gh release view "$tag" > /dev/null 2>&1; then
    gh release create "$tag" --draft --title "Scribe $tag" --notes-file "$root/scripts/release-notes.md"
  fi
  gh release upload "$tag" "$dmg" "$installer" --clobber
}

require_clean_main
build_macos
build_windows
tag_release
publish_assets
echo "Uploaded Scribe $tag. Review and publish: $(gh release view "$tag" --json url --jq .url)"
