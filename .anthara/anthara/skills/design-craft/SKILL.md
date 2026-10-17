---
name: design-craft
description: Domain expertise for UI / UX design decisions, layouts, color, typography, accessibility, motion, and the existing-vs-greenfield two-path flow. Concrete rules (4.5:1 contrast minimum, 44×44 touch targets, focus-visibility non-negotiable) and structured guidance for visual quality. Loaded by the reviewer when accessibility or design is in scope. Loaded as context by other agents — not user-invokable.
---

# design-craft

Reference principles loaded by any agent producing, evaluating, or documenting UI work. These apply universally; the project's `design.md` (when present) layers specific values on top.

## Two-path flow

Before applying any principle, determine which path you're on.

- **Path A — Existing design system.** The project has `design.md`, design tokens, a Tailwind config, CSS custom properties, or a component library. *Follow what exists. Never override. Never suggest alternatives.* Extract, document, and ensure new work is consistent.
- **Path B — Greenfield (no design system).** No system detected. Discovery's design probe captures direction (references, density, tone, mode, motion, accessibility level, worst-case environment). Spec-writer's layout probe captures feature-specific shape. Only then make visual decisions.

The first move on any UI-touching task is to determine which path the project is on.

## Priority system

When design concerns conflict, resolve by priority:

| Priority | Category | Severity |
|---|---|---|
| 1 | Accessibility | CRITICAL |
| 2 | Performance | HIGH |
| 3 | Layout and responsive | HIGH |
| 4 | Typography and color | MEDIUM |
| 5 | Animation and transitions | MEDIUM |
| 6 | Style and visual polish | LOW |

Accessibility always wins. A beautiful animation that breaks keyboard navigation is a defect, not a feature.

## Accessibility — non-negotiable minimums

- **Color contrast.** 4.5:1 for normal text, 3:1 for large text (WCAG AA). Bumps to 7:1 / 4.5:1 for AAA when the project commits to it.
- **Touch targets.** 44 × 44 px minimum for interactive elements (Apple HIG); 48 × 48 dp for Android.
- **Focus visibility.** Every interactive element has a visible focus indicator. Never remove `outline` without replacing it.
- **Semantic HTML over ARIA.** Use `<nav>`, `<main>`, `<aside>`, `<footer>` before reaching for ARIA roles.
- **Alt text.** Meaningful images get descriptive alt text; decorative images get `alt=""`.
- **Keyboard navigation.** Every interactive element reachable and operable via keyboard.
- **Color-blind safety.** Never rely on color alone to convey meaning. Pair color with a second signal — icon, label, pattern, shape. Test against deuteranopia (~8% of men) and protanopia.
- **Motion.** Respect `prefers-reduced-motion`; disable or reduce animations when set.

## Typography

- **Two faces maximum.** One sans for UI / body, optionally one serif for editorial copy or one mono for code / data. More creates noise.
- **Pair with contrast.** Body is humanist sans (Inter, Geist, IBM Plex); display is geometric (DM Sans, Manrope) or distinctive serif (Newsreader, Fraunces). Avoid pairing two similar faces.
- **Type scale.** Use a modular scale (1.125, 1.2, 1.25). Six steps is plenty for most products.
- **Line height.** 1.5-1.7 for body text; 1.1-1.3 for display. Tighter line height on display, looser on body.
- **Line length.** 60-75 characters for prose body. Wider for data tables and lists.
- **Hierarchy via weight + size + space**, not via color saturation. Colorful headings are a smell.

## Color

- **Anchor on a single neutral scale.** 9-11 steps from near-white to near-black. Tailwind's slate / zinc / gray / neutral are reasonable defaults.
- **One brand accent.** A second accent only if a structural reason demands it (success / error, dual-product). Three accents and your product looks generic.
- **Functional colors.** Success (green), warning (amber), error (red), info (blue). Use a muted version for surfaces, a saturated version for type and icons.
- **Dark mode.** Treat as a real mode, not a token inversion. Backgrounds shift; saturation drops; contrast adjusts. Test text-on-surface and surface-on-surface contrast in both modes.
- **Palette inspiration.** Sanzo Wada's *Dictionary of Color Combinations* is a good source when starting from scratch; pick a palette, refine, do not exceed five total colors including neutrals.

## Spatial composition

- **8-pt grid** for sizing and spacing (multiples of 4 for tighter fits). Tailwind's default is built on this.
- **Density** is a deliberate choice — Linear / Notion (dense), Stripe / Vercel (balanced), Apple / Granola (generous). Match the design context decided in discovery.
- **Vertical rhythm** matters as much as horizontal. Group related elements with tighter spacing, separate sections with larger gaps.
- **White space is structure**, not absence. Padding inside containers, margins between sections.

## Animation and motion

- **Subtle and infrequent** unless the design direction explicitly commits to expressive motion.
- **Easing.** `ease-out` for entries, `ease-in` for exits, `ease-in-out` for transitions between states. Avoid linear except for progress indicators.
- **Duration.** 150-300 ms for micro-interactions (button states, focus changes), 200-500 ms for transitions, 500-800 ms for substantial layout changes. Anything > 1s feels slow.
- **Purposeful, not decorative.** Animation should signal causation or progression, not decorate.

## Anti-patterns to avoid

- **Generic AI aesthetic.** Centered hero, gradient background, three-card feature row, dark blue / purple palette. Recognizable and forgettable.
- **Color as the only signal.** *"The red items are urgent"* fails for ~8% of users.
- **Custom shadows / borders not from the system.** One-off hand-tuned values fragment the design.
- **Auto-focus on page load.** Hostile to assistive technology; rarely what the user wants.
- **Removing `outline` without replacing it.** Breaks keyboard navigation.
- **Reaching for ARIA when semantic HTML suffices.** ARIA is for what HTML can't express, not a replacement for `<button>`.
- **Inventing components when the design system has one.** Every custom card / modal / select that diverges from the system is debt.

## Visual quality checklist

When evaluating built UI:

1. Does it pass automated WCAG checks (axe-core or equivalent) at the configured level?
2. Are all interactive elements reachable and operable via keyboard alone?
3. Do all colors meet the contrast minimums?
4. Are touch targets ≥ 44 × 44 px?
5. Does focus state look intentional, not afterthought?
6. Does the page work at 200% zoom without horizontal scrolling?
7. Does it work with `prefers-reduced-motion`?
8. Does it match the design system's tokens (no hand-tuned colors / shadows / spacing)?
9. Are error / empty / loading states present and well-formed?
10. Does it look like *this product*, not like every AI-built product?

## How to use this skill in your work

When loaded by the reviewer (accessibility / design dimensions):

1. **Determine path** (A or B) first. The rest depends on it.
2. **Read** `design.md` (Path A) or the discovery brief's Design context + spec's layout probe (Path B).
3. **Verify each UI-touching slice in the diff** against the priority system. Highest-priority issues become HIGH findings; lower-priority become MED / LOW.
4. **Run the visual quality checklist against the diff's UI surfaces.**
5. **Cite specific minimums** when a violation is found — *"Contrast 3.2:1 on the success-toast text against the green surface; WCAG AA minimum is 4.5:1."*
6. **Flag generic-AI-aesthetic drift** when new screens read like a stock template rather than the project's design.
