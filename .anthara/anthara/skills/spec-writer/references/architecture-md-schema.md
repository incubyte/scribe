# Project-wide `ARCHITECTURE.md` schema

Used when extracting §7 of the spec into a standalone repo-root reference. Read at extraction time (Step 11 of `SKILL.md`).

Sections, in order:

1. **Style and rationale** — from §7.2 (named style + 2-4 sentence justification).
2. **Tech stack** — from §7.1 (table of layer / choice / rationale).
3. **Dependency rules** — explicit form of the style's inward/outward direction.
4. **Module map** — the §7.3 mermaid graph, plus a responsibility table written fresh from the codebase (the spec draws boundaries by kind and carries no paths).
5. **Cross-cutting concerns** — auth, logging, error handling, validation, observability, feature flags, migrations, audit.
6. **Integration points** — external systems table (system / owning module / contract / failure mode).
7. **Forbidden anti-patterns** — what the style disallows.
8. **How to evolve** — adding a module, splitting, deprecating.
9. **Decision records (ADRs)** — append-only; this spec's architecture decision is ADR 1.
10. **References** — back to the spec, the discovery brief, Org Memory queries that informed the decision.

After writing, cite the new `ARCHITECTURE.md` from §2 of the spec as a numbered source. Future spec runs find it via Step 3's brownfield-with-architecture detection and treat it as authoritative.
