# Scribe

Architecture: see `docs/ARCHITECTURE.md` (hexagonal; ports in `scribe-core`, adapters in `crates/*`, apps in `apps/*`).

## Commands

- Format: `cargo fmt --all`
- Lint (Rust): `cargo clippy --workspace --all-targets -- -D warnings`
- Test (Rust): `cargo test --workspace`
- Lint (frontend): `npm run lint` in `apps/scribe-tray`
- Test (frontend): `npm test` in `apps/scribe-tray`
- Mutation: `cargo mutants --in-diff <diff>` and `npm run mutate` in `apps/scribe-tray`
