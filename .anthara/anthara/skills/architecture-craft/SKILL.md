---
name: architecture-craft
description: Domain expertise for architectural decision-making. Covers the recognized style families (layered / hexagonal / onion / clean / DDD / event-driven / CQRS / microservices / modular monolith / SOA / pipe-and-filter / actor / space-based / serverless / micro-frontends), their dependency directions, when each fits, the tech-stack-to-style coupling, and coupling and cohesion principles. Architecture is a deliberate choice, never emergent — a spec or codebase without a named style has not made the choice. Loaded by code-orienteer, slice-builder, and the reviewer whenever architectural decisions or verifications are involved. Loaded as context by other agents — not user-invokable.
---

# architecture-craft

Domain expertise loaded by agents whose work involves structural decisions — code-orienteer (extracting the de-facto style from a brownfield codebase), slice-builder (placing new code), reviewer (verifying adherence). Architecture is a deliberate choice; this skill carries the catalogue and the heuristics.

## Principles

- **Architecture is named.** A spec or codebase without a named architectural style has not made the choice — it has deferred it to whoever writes the first module.
- **Coupling and cohesion are the metrics.** High cohesion within a module (one responsibility, related data, related operations); low coupling between modules (few cross-module dependencies, narrow interfaces).
- **Dependency direction is the style's signature.** Each style imposes a direction (inward, outward, around a bus, through ports). Honor it.
- **Bounded contexts have anti-corruption layers.** When two contexts speak different vocabularies, translate at the seam. Do not let one bleed into the other.
- **Boundaries are inexpensive at design time, expensive after code.** Draw them early; redrawing later is rewriting.
- **The spec's `kinds-of-things` ontology shapes the architecture.** When the spec says *"there are three kinds of users"*, the architecture has three encapsulations, not one switch statement.

## Style families and when each fits

Choose by name; justify by requirements + tech stack.

- **Layered (N-tier)** — UI → application → domain → infrastructure. Familiar; easy to start; risks god-module-per-layer if cohesion is low. Fits CRUD-heavy, low-domain-complexity systems.
- **Hexagonal (Ports & Adapters)** — domain in the center; adapters at the edge for HTTP, DB, queues. Excellent testability; honest about external integrations. Fits systems with multiple I/O surfaces.
- **Onion / Clean Architecture** — concentric layers, dependencies inward. Domain knows nothing of frameworks. Strong separation; can over-formalize for small systems.
- **Domain-Driven Design (DDD)** — bounded contexts, aggregates, entities, value objects. Fits domain-heavy systems (healthcare, finance, logistics) where rules and language are intricate.
- **Event-driven** — components communicate via events on a bus; producers don't know consumers. Decouples temporally; excellent for high-throughput, multi-actor systems. Risks observability nightmares if not instrumented.
- **CQRS** — command and query paths separated; often paired with event sourcing. Fits systems with very different read and write workloads.
- **Microservices** — independent services, owned data, deployed separately. Fits multi-team, multi-domain systems with clear boundaries. Heavy ops cost; do not adopt for small systems.
- **Modular monolith** — single deployable; modules with internal boundaries; can be extracted later. Fits most products until scale demands services.
- **SOA** — coarse services, often over an enterprise bus. Mostly historical; modern equivalents are microservices or event-driven.
- **Pipe-and-filter** — data flows through a sequence of transforms. Fits ETL, data pipelines, compilers.
- **Actor model** — independent actors communicate by message; each owns its state. Fits highly concurrent, fault-tolerant systems (Erlang / Elixir / Akka).
- **Space-based** — in-memory data grid, processing units. Fits very high throughput, low latency systems.
- **Serverless / FaaS** — functions deployed and scaled independently; storage external. Fits event-driven workloads, low ops burden, sporadic traffic.
- **Micro-frontends** — UI broken into independently deployable shells; each owned by a team. Fits large multi-team frontends; over-engineering for small ones.

## Tech-stack ↔ style coupling

The tech stack constrains the style. Some pairings are natural; some are fights you don't want to pick:

- *Supabase + edge functions + Postgres + RLS* → naturally **serverless / FaaS** with RLS-as-authorization.
- *NestJS + Postgres* → naturally **modular monolith** with **layered** intra-module structure (controller / service / repository).
- *AWS Lambda + EventBridge + DynamoDB* → naturally **event-driven**.
- *Spring Boot + JPA* → **layered** by default; **DDD** with care.
- *Heavy domain logic + multiple actors* → **DDD with hexagonal ports**.
- *High-throughput data pipelines* → **pipe-and-filter** or **event-driven**.
- *Multiple teams, distinct domains* → **microservices** or **modular monolith** depending on org maturity.

When the user's stack and chosen style fight, that's a real finding. Surface it.

## Coupling and cohesion heuristics

- **Afferent coupling (Ca)** — how many other modules depend on this one. High Ca = stable; should change rarely.
- **Efferent coupling (Ce)** — how many modules this one depends on. High Ce = unstable; expected to change.
- **Instability (I = Ce / (Ca + Ce))** — should approach 0 for stable modules (domain), 1 for unstable modules (UI, glue code). Stable modules should be abstract; unstable modules should be concrete.
- **Cycles are forbidden.** If module A depends on B and B depends on A, the boundary is wrong. Refactor.
- **Cohesion (LCOM) — high.** Members of a module relate to each other; if you can split the module along a clean line, do.

## Anti-patterns

- **Layered chaos** — controller calling another controller; service calling another service across domains. Indicates missing structure.
- **God service** — one service with many responsibilities. Split.
- **Anemic domain** — domain entities are bags of getters; logic lives in services. The domain should carry the rules.
- **Cross-context imports** — bounded context A reaches into B's internals. Use an anti-corruption layer.
- **Hidden coupling via shared mutable state** — singletons, ambient context, request-scoped globals. Inject explicitly.
- **Architecture by accident** — features added wherever feels convenient. The spec's named style is the prevention.
- **Over-engineered architecture for the problem size** — DDD + CQRS for a CRUD app; microservices for a 3-person team. Match style to system size and team size.
- **Style mismatch with stack** — fighting the framework. Spring's ergonomics push layered; insisting on pure clean architecture in Spring is a multi-month tax.

## Six-dimension evaluation framework

When recommending or verifying an architecture, evaluate the choice across six dimensions. The recommendation is the *simplest architecture that passes all six well enough for the current requirements*, with explicit evolution triggers for when complexity grows.

| Dimension | The question |
|---|---|
| **Simplicity** | Is this the simplest shape that meets the requirements? Or are we paying complexity tax for needs that may never materialize? |
| **Cohesion** | Does each module have one clear responsibility? Can you state each module's purpose in a single sentence? |
| **Decoupling** | Are modules independent enough to change one without rippling? Are dependencies narrow (interfaces, not implementations) and directional (no cycles)? |
| **Evolvability** | What's the path forward when this grows? Can we extract a service later, swap an adapter, split a module — without rewriting? |
| **Testability** | Can each module be tested in isolation? Are external dependencies injected? Is the domain logic pure? |
| **Readability** | Can an engineer new to the codebase understand the shape in 30 minutes? Are module names and structure self-explanatory? |

## Simplest that works — and when to evolve

The default choice for any spec is the *simplest* architecture that scores well on all six dimensions today. Add complexity only when a current requirement demands it; document the *evolution triggers* — concrete signals that say "now it's time to add complexity."

Common starting points and their evolution triggers:

| Starting style | When it fits | Evolve to → | Trigger |
|---|---|---|---|
| **Feature folders** (flat per-domain layout) | Small product, single team, low domain complexity | **Modular monolith + layered intra-module** | Domain folder grows past 10-15 files; cross-domain imports proliferate |
| **Modular monolith + layered** | Most products, single team, moderate domain | **Hexagonal / Ports & Adapters** | Multiple input surfaces (web + mobile + MCP + queue consumers); want to swap I/O without touching domain |
| **Hexagonal** | Multiple I/O surfaces, testability is critical | **DDD with bounded contexts** | Domain logic becomes intricate; team is sub-dividing along domain lines; aggregate boundaries emerge |
| **Modular monolith** (any internal style) | Single deployable, single team | **Microservices** | Independent scaling needs; multiple teams owning disjoint domains; ops maturity to handle distributed systems |
| **Synchronous request-response** | Most CRUD + transactional flows | **Event-driven** | Throughput / latency demands async; multiple consumers per producer; temporal decoupling matters |

**The rule:** name today's choice and tomorrow's trigger. *"Modular monolith with layered intra-module today; evolve to hexagonal when we add a second input surface beyond HTTP."* This way the architecture's evolution path is built into the spec.

## How to use this skill in your work

When loaded by an agent, this skill should drive the following:

1. **Read the spec's §7 Architecture or `ARCHITECTURE.md` FIRST.** That is the chosen style for this project. Verify against it; do not invent a different one.
2. **Read the task context file's Patterns section.** Established patterns are the de-facto style; honor them unless explicitly evolving.
3. **Surface deviations from the chosen style as findings** (when in reviewer role) or **align new code with the style** (when in slice-builder role).
4. **When choosing placement** for new code (slice-builder role), follow the style's dependency direction. New domain logic stays in the domain; new I/O lives at the edge.
5. **When verifying a design plan** (reviewer role), check that the spec's slices respect the style's boundaries — UI slices don't reach into the domain directly; integrations go through adapters.
6. **When extracting de-facto style** (code-orienteer role), name it from this skill's catalogue. Tell the user explicitly which style the codebase reads as.
