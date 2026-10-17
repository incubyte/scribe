# Synthesis rubric

How to turn a corpus of ingested inputs into the discovery doc. Synthesis runs from scratch each time the corpus changes — no patching individual sections in isolation.

## 1. Build the type ontology

Identify the kinds of things the corpus describes. Four categories:

- **Kinds of users** — distinct user types with distinct goals and contexts. Drawn from the corpus, not from imagined personas.
- **Kinds of data** — distinct data types the work touches (records, events, measurements, documents, attachments). Note regulated kinds (PHI, PII, payment, biometric).
- **Kinds of events** — meaningful state changes the system observes or causes (admission, discharge, claim filed, alert triggered, threshold crossed).
- **Kinds of states** — meaningful resting states a thing can be in (pending review, approved, escalated, archived).

Rules:

- Categories must come from corpus language. If the support tickets call them "high-touch accounts" and the sales transcripts call them "enterprise", note both labels and pick the most evidenced one — do not invent a third.
- Each category must be **non-overlapping** with its siblings. If two categories overlap, you have not yet found the right cuts.
- Each category must be **load-bearing** — it must matter for some finding, journey, or constraint. Categories that don't show up downstream are decorative; remove them.
- Use **categorical framing** ("there are three kinds of users: clinicians, schedulers, billing"). Avoid branching language ("if the user is a clinician, then..."). The first shapes encapsulation in the eventual code; the second shapes if-else.

## 2. Map journeys

For each major workflow described in the corpus, map the actual journey:

- **Steps in order**, including the starts and ends.
- **Workarounds** — note explicitly when a step happens outside the system (emailing a PDF, copying to a spreadsheet, calling someone).
- **Hand-offs** between user types. These are usually where pain lives.
- **Decision points** — where the user picks a path. Note which path they pick by default.
- **Error paths** — what happens when this step fails. Many sources only describe the happy path; flag where error paths are missing in the corpus.

Cite at least one source per journey step.

## 3. Cluster findings by theme

Group extracted findings into themes. For each finding:

### Compute signal strength

Three dimensions:

- **Source count** — how many sources mention this.
- **Source diversity** — how varied the source types are (a finding echoed across surveys, tickets, and calls is stronger than one echoed across three calls).
- **Consistency** — do the sources agree, or do they contradict?

Map to a label:

- **Strong** — multiple sources, diverse types, consistent.
- **Moderate** — multiple sources, but limited type diversity OR some inconsistency.
- **Weak** — single source OR offhand mention OR significant contradictions.

State the strength label on every finding. Do not soften weak findings to look stronger.

### Each finding includes

- **Summary statement** — one sentence, plain language.
- **Signal strength** — strong / moderate / weak, with the count and types.
- **Source attributions** — every source that supports it.
- **Stakeholder-attributed quotes** — verbatim where the language matters.
- **Tensions** — link to any contradiction this finding is part of.

## 4. Surface contradictions explicitly

When two sources disagree, do not pick a winner. Surface the tension:

> The support team reports that customers complete the onboarding flow in under 5 minutes (12 tickets, 3 months). Sales calls indicate it takes "all afternoon" (3 of 5 transcripts). Possible reconciliations: (a) different customer segments, (b) sales calls describe initial setup including data import while support tickets describe authentication only, (c) different definitions of "onboarding".

Always propose 2-3 possible reconciliations. The user picks. Sometimes the reconciliation is "we don't know yet, this needs more research" — that is a valid outcome and goes in the Gaps section.

## 5. Flag regulatory signals

Note any signal that looks regulatory. Do not analyze depth — that is for `/anthara:spec-writer`. Flag:

- **PHI flow** — patient data being read, written, transmitted, stored. Cite the source mention.
- **Clinical decision support** — anything that recommends, triages, or auto-decides on clinical actions.
- **Payment data** — card numbers, account numbers, transactions.
- **PII subject to law** — GDPR (EU residents), CCPA (California), regional equivalents.
- **Accessibility** — explicit mentions of screen readers, keyboard navigation, contrast, low-vision users.
- **Audit and consent** — mentions of access logs, consent capture, retention.
- **Security controls** — mentions of SOC2, ISO, HITRUST, FedRAMP.

For each, cite the source and quote the passage that triggered the flag.

## 6. Identify gaps

Honest about what the corpus does not cover. Examples:

- "Rich signal from power users (8 transcripts), almost none from first-time users (0 transcripts)."
- "Workflow is well-described from the clinician perspective; the billing team's experience is absent."
- "Failure modes are not described — every source describes the happy path."
- "No quantitative data on frequency or volume."

For each gap, propose a specific follow-up: who to talk to, which channel to scrape, which dataset to pull.

## 7. Surface open questions

Decisions the user needs to make before spec-writing begins. Examples:

- "Are we solving for the clinician segment or the admin segment? Findings differ."
- "Is the regulatory scope HIPAA only, or also FDA SaMD? Some quotes hint at clinical decisions."
- "Is this a replacement of the existing workflow or a parallel path? Sources disagree."

Open questions block spec-readiness. Be honest in the doc about whether the brief is spec-ready.

## 8. Spec-readiness signal

In the Open Questions section, end with a one-line honest assessment:

- "Spec-ready — open questions are scoping decisions, not unknowns."
- "Spec-ready with caveats — see open question N."
- "Not spec-ready — significant unknowns in [area]; suggest [specific follow-up]."

Do not declare spec-readiness without honest evidence.

## What synthesis must avoid

- Inventing categories not grounded in source language.
- Smoothing contradictions to make the brief look tidy.
- Promoting weak findings to look strong.
- Listing the same finding multiple times under different sources (deduplicate).
- Including small talk, boilerplate, or off-topic content.
- Generating any finding without source attribution.
