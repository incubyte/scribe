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

## Releasing

Releases are built on this Mac (no CI minutes):

1. Bump `version` in `apps/scribe-tray/src-tauri/tauri.conf.json` and commit it on `main`.
2. Run `scripts/release.sh`. It builds the macOS `.dmg` and cross-compiles the Windows installer (needs `cargo install cargo-xwin` and `brew install llvm lld nsis`), tags `v<version>`, and uploads both to a draft GitHub Release.
3. Review the draft on GitHub and publish it.

`.github/workflows/release.yml` is a manual fallback that builds the Windows installer on GitHub for an existing tag. Builds are ad-hoc signed (macOS) and unsigned (Windows) until signing certificates exist.
