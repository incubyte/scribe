import { describe, expect, it } from "vitest";
import notebookHtml from "../../../../plugins/scribe/skills/meeting-notes/notebook.html?raw";
import { type ColorScheme, contrastRatio, themeTokens } from "./contrast";
import { createFakeClaudeRuntime } from "./fakeClaudeRuntime";
import { mountNotebook } from "./mountNotebook";
import { aMeeting, aTodo, seedMeeting } from "./notebookRecords";

const SCHEMES: ColorScheme[] = ["light", "dark"];
const TEXT_CONTRAST = 4.5;
const UI_PART_CONTRAST = 3;
const TUESDAY_STANDUP = "2026-10-21-112155";

function contrastOf(scheme: ColorScheme, foreground: string, background: string): number {
  const tokens = themeTokens(notebookHtml, scheme);
  return contrastRatio(tokens[foreground], tokens[background]);
}

describe("the notebook wears the Incubyte theme", () => {
  it.each(SCHEMES)("body text stays readable on the page in %s mode", (scheme) => {
    expect(contrastOf(scheme, "--text", "--surface")).toBeGreaterThanOrEqual(TEXT_CONTRAST);
  });

  it.each(SCHEMES)("muted text stays readable on the page in %s mode", (scheme) => {
    expect(contrastOf(scheme, "--text-muted", "--surface")).toBeGreaterThanOrEqual(TEXT_CONTRAST);
  });

  it.each(SCHEMES)("risk text stays readable on the page in %s mode", (scheme) => {
    expect(contrastOf(scheme, "--recording", "--surface")).toBeGreaterThanOrEqual(TEXT_CONTRAST);
  });

  it.each(SCHEMES)("headings stay readable on the page in %s mode", (scheme) => {
    expect(contrastOf(scheme, "--heading", "--surface")).toBeGreaterThanOrEqual(TEXT_CONTRAST);
  });

  it.each(SCHEMES)("header text stays readable on the green band in %s mode", (scheme) => {
    expect(contrastOf(scheme, "--on-header", "--header-band")).toBeGreaterThanOrEqual(TEXT_CONTRAST);
  });

  it.each(SCHEMES)("AI action buttons keep readable labels in %s mode", (scheme) => {
    expect(contrastOf(scheme, "--on-button", "--button-fill")).toBeGreaterThanOrEqual(TEXT_CONTRAST);
  });

  it.each(SCHEMES)("focus ring is visible against the page in %s mode", (scheme) => {
    expect(contrastOf(scheme, "--accent", "--surface")).toBeGreaterThanOrEqual(UI_PART_CONTRAST);
  });

  it.each(SCHEMES)("control borders are visible against the page in %s mode", (scheme) => {
    expect(contrastOf(scheme, "--control-border", "--surface")).toBeGreaterThanOrEqual(UI_PART_CONTRAST);
  });

  it("page background follows the system colour scheme", () => {
    expect(themeTokens(notebookHtml, "light")["--surface"]).toBe("#ebfff6");
    expect(themeTokens(notebookHtml, "dark")["--surface"]).toBe("#062925");
    expect(notebookHtml).toMatch(/body\s*\{[^}]*background:\s*var\(--surface\)/);
  });

  it("loads Fraunces and Inclusive Sans from Google Fonts with system fallbacks", () => {
    const tokens = themeTokens(notebookHtml, "light");
    expect(notebookHtml).toContain("https://fonts.googleapis.com/css2?family=Fraunces");
    expect(notebookHtml).toContain("family=Inclusive+Sans");
    expect(tokens["--font-heading"]).toMatch(/^"Fraunces".*serif$/);
    expect(tokens["--font-body"]).toMatch(/^"Inclusive Sans".*sans-serif$/);
  });

  it("uses the decorative green only as a decorative token", () => {
    const usages = notebookHtml.match(/#40a578/gi) ?? [];
    expect(usages).toHaveLength(1);
    expect(themeTokens(notebookHtml, "light")["--decor"]).toBe("#40a578");
    expect(notebookHtml).not.toMatch(/var\(--decor\)/);
  });

  it("AI action buttons show an arrow hidden from screen readers", async () => {
    const runtime = createFakeClaudeRuntime();
    seedMeeting(runtime.database, {
      id: TUESDAY_STANDUP,
      meeting: aMeeting({ title: "Tuesday stand-up" }),
      todos: [
        aTodo(TUESDAY_STANDUP, {
          text: "Share the QA rota",
          actions: [{ label: "Create ticket", prompt: "Write a ticket for sharing the QA rota." }],
        }),
      ],
    });

    const notebook = await mountNotebook(runtime);

    const arrow = notebook.document.querySelector(".todo__actions .button .button__arrow svg");
    expect(arrow?.getAttribute("aria-hidden")).toBe("true");
    expect(arrow?.getAttribute("focusable")).toBe("false");
  });

  it("keeps focus, forced-colours, reduced-motion, reduced-transparency and narrow-screen rules", () => {
    expect(notebookHtml).toContain(":focus-visible");
    expect(notebookHtml).toContain("@media (forced-colors: active)");
    expect(notebookHtml).toContain("@media (prefers-reduced-motion: reduce)");
    expect(notebookHtml).toContain("@media (prefers-reduced-transparency: reduce)");
    expect(notebookHtml).toContain("@media (max-width: 640px)");
  });
});
