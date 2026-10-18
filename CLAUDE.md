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

1. Bump `version` in `apps/scribe-tray/src-tauri/tauri.conf.json` and commit it.
2. Tag the commit with the same version (`git tag v0.2.0 && git push origin v0.2.0`).
3. `.github/workflows/release.yml` builds the macOS `.dmg` and Windows installers and attaches them to a draft GitHub Release; review and publish it.

Builds are ad-hoc signed (macOS) and unsigned (Windows) until signing certificates exist; adding the Apple and Windows signing secrets to the workflow enables real signing.
