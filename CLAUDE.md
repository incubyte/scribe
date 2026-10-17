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
