# Canonical discovery doc skeleton

Use verbatim. Section order is fixed. Read at write time (Step 7 of `SKILL.md`).

````markdown
# Discovery: <one-line problem framing>

> Owned by: <contributor name>  ·  Started: <YYYY-MM-DD>  ·  Last synthesized: <YYYY-MM-DD>
> Anthara discovery brief — multi-stakeholder living document. Run `/anthara:discovery` against this doc to add inputs.

---

## 1. Problem framing

What we're exploring and why. 2-4 sentences. Includes the actor types affected and the urgency tier as the user described them.

## 2. Sources

Indexed list of every input. Each source has an integer ID used in finding attributions.

| ID | Type | Contributor | Date | Description |
|---|---|---|---|---|
| 1 | Granola Discovery Call Transcript | <name> | <YYYY-MM-DD> | <short description> |
| 2 | Typeform Beta Survey | <name> | <YYYY-MM-DD> | <short description> |
| 3 | Teams Channel Conversation | <name> | <YYYY-MM-DD> | <short description> |

## 3. Type ontology

Drawn from corpus language. Categorical, non-overlapping, load-bearing.

### 3.1 Kinds of users
- **<Type>** — <description>; <where in corpus> [1].

### 3.2 Kinds of data
- **<Type>** — <description, regulated or not> [1].

### 3.3 Kinds of events
- **<Event>** — <when it fires, who triggers> [1].

### 3.4 Kinds of states
- **<State>** — <what it means> [1].

## 4. Journeys

### 4.1 <Journey name>

1. <Step> [1].
2. <Step> [1, 2].
3. **Workaround:** <out-of-system step described in corpus> [1].
4. **Error path missing in corpus** — <gap>.

### 4.2 <Another journey>

1. ...

## 5. Findings

### 5.1 Theme: <theme name>

**5.1.1 <one-sentence summary>.**
- Signal strength: **Strong | Moderate | Weak** (<source count>, <type diversity>, <consistency>).
- Sources: [1, 2].
- Quote: "<verbatim>" — <speaker, role>, [1].
- Tension: see 6.1.

**5.1.2 <another finding in this theme>.**
- ...

### 5.2 Theme: <another theme>

**5.2.1 <summary>.**
- ...

### 5.N Theme: Design context

(Always include this theme. Greenfield: outputs of the design probe — references, density, tone, mode, motion, accessibility, environment. Brownfield: extracted conventions and intent. Brownfield-with-system: `design.md` reference plus any drift notes.)

**5.N.1 <Design philosophy / direction>** — <one-sentence summary>.
- Mode: *greenfield* / *brownfield* / *brownfield-with-system*.
- Sources: [<probe responses, codebase scan, existing design.md, screenshots>].
- References named (greenfield): <products the user admires>.
- Density / tone / mode / motion / accessibility (greenfield): <captured probe answers>.
- Extracted conventions (brownfield): <framework, library, palette, type pairing, dominant patterns>.

**5.N.2 <Anti-patterns and don'ts>** — <what is banned and why>.
- ...

## 6. Tensions and contradictions

### 6.1 <tension name>

- <Source view A> [1].
- <Source view B contradicting A> [2].
- Possible reconciliations:
  - (a) <reconciliation>.
  - (b) <reconciliation>.
  - (c) <reconciliation>.
- Decision needed in spec.

### 6.2 <another tension>

- ...

## 7. Regulatory signals

Flagged for `/anthara:spec-writer` to analyze.

**7.1 <Signal kind — PHI flow / clinical decision / payment / accessibility / audit / consent / SOC2 / etc.>** — <quoted or paraphrased trigger> [1].

**7.2 <Another signal kind>** — <trigger> [1].

## 8. Gaps

**8.1** <What the corpus does not cover>.

**8.2** <Another gap>.

Suggested follow-up:
- <Specific action: who to talk to, what data to pull>.

## 9. Open questions

Decisions needed before spec-writing.

**9.1** <Open question>.

**9.2** <Open question>.

**Spec-readiness:** <Spec-ready | Spec-ready with caveats — see 9.N | Not spec-ready — significant unknowns in <area>>.

---

## Changelog

- <YYYY-MM-DD> — <contributor> — <what changed>.
````
