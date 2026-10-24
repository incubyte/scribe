# Scribe

Architecture: see `docs/ARCHITECTURE.md` (hexagonal; ports in `scribe-core`, adapters in `crates/*`, apps in `apps/*`).

## Commands

- Stage the bundled MCP server (run once before clippy, test or build, and after changing `apps/scribe-mcp`): `apps/scribe-tray/scripts/stage-sidecar.sh`
- Format: `cargo fmt --all`
- Lint (Rust): `cargo clippy --workspace --all-targets -- -D warnings`
- Test (Rust): `cargo test --workspace`
- Lint (frontend): `npm run lint` in `apps/scribe-tray`
- Test (frontend): `npm test` in `apps/scribe-tray`
- Mutation: `cargo mutants --in-diff <diff>` and `npm run mutate` in `apps/scribe-tray`

## CI

CI layout and its reasons are ADR 2 and ADR 3 in `docs/ARCHITECTURE.md`.

- `portable` (ubuntu) checks the platform-neutral crates and the frontend, `macos` checks the Mac-only crates, and `windows` only runs clippy on the Windows-only crates. A new crate goes into `PORTABLE_CRATES` in `.github/workflows/ci.yml` unless it needs macOS or Windows APIs.
- CI pins Rust 1.93.1. Raise it only together with the local toolchain.
- Check the exit code of `npm test`, not only the passed count. An unhandled promise rejection fails the run even when every test passes, so test backends built with `mockIPC` must answer every command the popover invokes.

## Releasing

Releases are built on this Mac (no CI minutes). One-time prerequisites besides the Windows tooling below:

- The update signing key: `npx tauri signer generate -w ~/.tauri/scribe-update.key` in `apps/scribe-tray`, then `security add-generic-password -a "$USER" -s scribe-update-key -w` with the same password. Back both up in the password manager; losing them ends updates for every installed copy (ADR 9).
- `brew install minisign`, which `scripts/release.sh` uses to verify update signatures before it tags.

1. Bump `version` in `apps/scribe-tray/src-tauri/tauri.conf.json` and commit it on `main`.
2. Run `scripts/release.sh`. It builds the macOS `.dmg` and cross-compiles the Windows installer (needs `cargo install cargo-xwin` and `brew install llvm lld nsis`), tags `v<version>`, and uploads both to a draft GitHub Release.
3. Review the draft on GitHub and publish it.

`scripts/release.sh` also validates the Claude plugin and uploads `scribe-plugin.zip`; the manual `release.yml` fallback does neither.

`.github/workflows/release.yml` is a manual fallback that builds the Windows installer on GitHub for an existing tag. macOS builds are signed with a Developer ID and notarized, which needs the certificate in the keychain and a one-time `xcrun notarytool store-credentials scribe-notary`; Windows builds are unsigned. Keep `src-tauri/Entitlements.plist` with the audio-input entitlement whenever signing changes, or the microphone stops working. Decisions behind this are ADR 4 to ADR 6 and ADR 8 in `docs/ARCHITECTURE.md`.
