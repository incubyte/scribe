import { describe, expect, it } from "vitest";
import { createFakeClaudeRuntime } from "./fakeClaudeRuntime";
import { type MountedNotebook, mountNotebook } from "./mountNotebook";
import { aMeeting, seedMeeting } from "./notebookRecords";

const SHOW_OLDER_LABEL = "Show older meetings";
const FIRST_START_MS = Date.parse("2026-10-01T00:00:00Z");
const HOUR_MS = 60 * 60 * 1000;

function meetingNumberedAs(number: number): string {
  return "Meeting " + number;
}

function seedMeetings(runtime: ReturnType<typeof createFakeClaudeRuntime>, count: number): void {
  Array.from({ length: count }, (_, index) => index + 1).forEach((number) => {
    seedMeeting(runtime.database, {
      id: "meeting-" + String(number).padStart(2, "0"),
      meeting: aMeeting({
        title: meetingNumberedAs(number),
        started_at: new Date(FIRST_START_MS + number * HOUR_MS).toISOString(),
      }),
    });
  });
}

function openNotebookWith(count: number): Promise<MountedNotebook> {
  const runtime = createFakeClaudeRuntime();
  seedMeetings(runtime, count);
  return mountNotebook(runtime);
}

function listedMeetingTitles(notebook: MountedNotebook): string[] {
  return Array.from(notebook.document.querySelectorAll(".meeting-list .session__title"), (title) => title.textContent ?? "");
}

function pickerOptionLabels(notebook: MountedNotebook): string[] {
  return Array.from(notebook.document.querySelectorAll("#meeting-select option"), (option) => option.textContent ?? "");
}

function selectedMeetingInList(notebook: MountedNotebook): string | null | undefined {
  return notebook.document.querySelector('.session[aria-current="true"] .session__title')?.textContent;
}

function openMeetingTitle(notebook: MountedNotebook): string | null | undefined {
  return notebook.document.querySelector("#meeting-pane .meeting__title")?.textContent;
}

function meetingCountText(notebook: MountedNotebook): string | null | undefined {
  return notebook.document.getElementById("meeting-count")?.textContent;
}

function showOlderButton(notebook: MountedNotebook): HTMLButtonElement | null {
  return (
    Array.from(notebook.document.querySelectorAll<HTMLButtonElement>("button")).find(
      (candidate) => candidate.textContent === SHOW_OLDER_LABEL,
    ) ?? null
  );
}

function pressShowOlder(notebook: MountedNotebook): void {
  const button = showOlderButton(notebook);
  if (!button) {
    throw new Error("No " + SHOW_OLDER_LABEL + " button is on the page");
  }
  button.click();
}

function meetingButton(notebook: MountedNotebook, title: string): HTMLButtonElement {
  const button = Array.from(notebook.document.querySelectorAll<HTMLButtonElement>(".session")).find(
    (candidate) => candidate.querySelector(".session__title")?.textContent === title,
  );
  if (!button) {
    throw new Error("No meeting in the list is titled " + title);
  }
  return button;
}

function titlesFromNewestDownTo(newest: number, oldest: number): string[] {
  return Array.from({ length: newest - oldest + 1 }, (_, offset) => meetingNumberedAs(newest - offset));
}

describe("the notebook lists meetings a page at a time", () => {
  it("a long notebook shows the newest 20 meetings and offers older ones", async () => {
    const notebook = await openNotebookWith(45);

    expect(listedMeetingTitles(notebook)).toEqual(titlesFromNewestDownTo(45, 26));
    expect(showOlderButton(notebook)).not.toBeNull();
  });

  it("showing older meetings adds the next 20 and keeps the open meeting", async () => {
    const notebook = await openNotebookWith(45);
    meetingButton(notebook, meetingNumberedAs(30)).click();
    await notebook.settle();
    const firstRowBefore = notebook.document.querySelector(".meeting-list .session");

    pressShowOlder(notebook);
    await notebook.settle();

    expect(listedMeetingTitles(notebook)).toEqual(titlesFromNewestDownTo(45, 6));
    expect(notebook.document.querySelector(".meeting-list .session")).toBe(firstRowBefore);
    expect(selectedMeetingInList(notebook)).toBe(meetingNumberedAs(30));
    expect(openMeetingTitle(notebook)).toBe(meetingNumberedAs(30));
    expect(showOlderButton(notebook)).not.toBeNull();

    pressShowOlder(notebook);
    await notebook.settle();

    expect(listedMeetingTitles(notebook)).toEqual(titlesFromNewestDownTo(45, 1));
    expect(showOlderButton(notebook)).toBeNull();
  });

  it("moves focus to the picker when the meeting list is hidden on a narrow screen", async () => {
    const notebook = await openNotebookWith(25);
    const hideList = notebook.document.createElement("style");
    hideList.textContent = ".meeting-list { display: none; }";
    notebook.document.head.append(hideList);

    pressShowOlder(notebook);
    await notebook.settle();

    expect(notebook.document.activeElement?.id).toBe("meeting-select");
  });

  it("with exactly 20 meetings there is nothing older to show", async () => {
    const notebook = await openNotebookWith(20);

    expect(listedMeetingTitles(notebook)).toEqual(titlesFromNewestDownTo(20, 1));
    expect(showOlderButton(notebook)).toBeNull();
  });

  it("the header counts every meeting, not just the ones shown", async () => {
    const notebook = await openNotebookWith(45);

    expect(listedMeetingTitles(notebook)).toHaveLength(20);
    expect(meetingCountText(notebook)).toBe("45 meetings");
  });

  it("the narrow-screen picker offers the same older meetings", async () => {
    const notebook = await openNotebookWith(45);

    expect(pickerOptionLabels(notebook)).toHaveLength(20);
    pressShowOlder(notebook);
    await notebook.settle();

    expect(pickerOptionLabels(notebook)).toHaveLength(40);
    expect(showOlderButton(notebook)?.closest(".meeting-list")).toBeNull();
    expect(showOlderButton(notebook)?.parentElement?.id).toBe("sidebar");
  });
});
