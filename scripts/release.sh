#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tray="$root/apps/scribe-tray"
windows_target="x86_64-pc-windows-msvc"
version="$(jq -r .version "$tray/src-tauri/tauri.conf.json")"
tag="v$version"
plugin_zip="$root/target/release/scribe-plugin.zip"
dmg="$root/target/release/bundle/dmg/Scribe_${version}_aarch64.dmg"
signing_identity="Developer ID Application: Sapankumar Parikh H (V92DDD85J3)"
notary_profile="scribe-notary"
update_key_path="$HOME/.tauri/scribe-update.key"
update_key_service="scribe-update-key"
release_config="src-tauri/tauri.release.conf.json"
update_archive="$root/target/release/bundle/macos/Scribe.app.tar.gz"
versioned_update_archive="$root/target/release/Scribe_${version}_aarch64.app.tar.gz"
installer="$root/target/$windows_target/release/bundle/nsis/Scribe_${version}_x64-setup.exe"
manifest="$root/target/release/latest.json"
download_base="https://github.com/incubyte/scribe/releases/download/$tag"
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

require_notary_credentials() {
  if ! xcrun notarytool history --keychain-profile "$notary_profile" > /dev/null 2>&1; then
    echo "Store notarization credentials first: xcrun notarytool store-credentials $notary_profile" >&2
    exit 1
  fi
}

require_update_signing_key() {
  if [ ! -f "$update_key_path" ]; then
    echo "Create the update signing key first: npx tauri signer generate -w $update_key_path" >&2
    exit 1
  fi
  if ! security find-generic-password -s "$update_key_service" > /dev/null 2>&1; then
    echo "Store the update key password first: security add-generic-password -a \"\$USER\" -s $update_key_service -w" >&2
    exit 1
  fi
  if ! command -v minisign > /dev/null; then
    echo "minisign not found; install it with: brew install minisign" >&2
    exit 1
  fi
}

tauri_build_signed() {
  TAURI_SIGNING_PRIVATE_KEY="$update_key_path" \
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(security find-generic-password -s "$update_key_service" -w)" \
    npx tauri build "$@"
}

build_macos() {
  bash "$tray/scripts/stage-sidecar.sh"
  (cd "$tray" && APPLE_SIGNING_IDENTITY="$signing_identity" \
    tauri_build_signed --bundles app,dmg --config src-tauri/tauri.ci.conf.json --config "$release_config")
}

notarize_macos() {
  xcrun notarytool submit "$dmg" --keychain-profile "$notary_profile" --wait
  xcrun stapler staple "$dmg"
  spctl --assess --type open --context context:primary-signature --verbose "$dmg"
}

verify_macos_update_archive() {
  local extracted assessment
  extracted="$(mktemp -d)"
  tar -xzf "$update_archive" -C "$extracted"
  assessment="$(spctl --assess --type execute --verbose "$extracted/Scribe.app" 2>&1 || true)"
  rm -rf "$extracted"
  if [[ "$assessment" != *"source=Notarized Developer ID"* ]]; then
    echo "The update archive's app is not a notarized Developer ID build: $assessment" >&2
    exit 1
  fi
}

verify_update_signature() {
  local file="$1" keys verification
  keys="$(mktemp -d)"
  jq -r .plugins.updater.pubkey "$tray/src-tauri/tauri.conf.json" | base64 -d > "$keys/update.pub"
  base64 -d < "$file.sig" > "$keys/update.minisig"
  verification="$(minisign -V -p "$keys/update.pub" -m "$file" -x "$keys/update.minisig" 2>&1 || true)"
  rm -rf "$keys"
  if [[ "$verification" != *"Signature and comment signature verified"* ]] || ! names_release_version "$verification"; then
    echo "Update signature check failed for $(basename "$file") (expected version:$version)." >&2
    exit 1
  fi
}

names_release_version() {
  local tab=$'\t' newline=$'\n'
  local pattern="version:${version//./\\.}([$tab$newline]|\$)"
  [[ "$1" =~ $pattern ]]
}

build_windows() {
  bash "$tray/scripts/fetch-vc-redist.sh"
  bash "$tray/scripts/stage-sidecar.sh" "$windows_target"
  (cd "$tray" && tauri_build_signed --runner cargo-xwin --target "$windows_target" --bundles nsis \
    --config src-tauri/tauri.ci.conf.json --config "$release_config")
}

validate_plugin() {
  if ! command -v claude > /dev/null; then
    echo "claude CLI not found; skipping plugin validation." >&2
    return
  fi
  claude plugin validate --strict "$root/plugins/scribe"
  claude plugin validate --strict "$root"
}

build_plugin_zip() {
  if ! command -v zip > /dev/null; then
    echo "zip not found; install it to build the plugin zip." >&2
    exit 1
  fi
  validate_plugin
  mkdir -p "$(dirname "$plugin_zip")"
  rm -f "$plugin_zip"
  (cd "$root/plugins/scribe" && zip -qr "$plugin_zip" . -x '*.DS_Store')
}

write_update_manifest() {
  jq -n \
    --arg version "$version" \
    --rawfile notes "$root/scripts/release-notes.md" \
    --arg pub_date "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg darwin_url "$download_base/$(basename "$versioned_update_archive")" \
    --arg darwin_signature "$(cat "$update_archive.sig")" \
    --arg windows_url "$download_base/$(basename "$installer")" \
    --arg windows_signature "$(cat "$installer.sig")" \
    '{version: $version, notes: $notes, pub_date: $pub_date, platforms: {
      "darwin-aarch64": {url: $darwin_url, signature: $darwin_signature},
      "windows-x86_64": {url: $windows_url, signature: $windows_signature}}}' > "$manifest"
}

tag_release() {
  if ! git -C "$root" rev-parse -q --verify "refs/tags/$tag" > /dev/null; then
    git -C "$root" tag "$tag"
  fi
  git -C "$root" push origin "$tag"
}

publish_assets() {
  if ! gh release view "$tag" > /dev/null 2>&1; then
    gh release create "$tag" --draft --title "Scribe $tag" --notes-file "$root/scripts/release-notes.md"
  fi
  gh release upload "$tag" "$dmg" "$installer" "$plugin_zip" "$versioned_update_archive" "$manifest" --clobber
}

require_clean_main
require_notary_credentials
require_update_signing_key
build_macos
notarize_macos
verify_macos_update_archive
verify_update_signature "$update_archive"
cp "$update_archive" "$versioned_update_archive"
build_windows
verify_update_signature "$installer"
build_plugin_zip
write_update_manifest
tag_release
publish_assets
echo "Uploaded Scribe $tag. Review and publish: $(gh release view "$tag" --json url --jq .url)"
