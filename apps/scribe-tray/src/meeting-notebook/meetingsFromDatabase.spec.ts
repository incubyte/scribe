import { describe, expect, it } from "vitest";
import { createFakeClaudeRuntime } from "./fakeClaudeRuntime";
import { type MountedNotebook, mountNotebook } from "./mountNotebook";
import { aMeeting, aTodo, seedMeeting, writeMeeting } from "./notebookRecords";

const SUNDAY_RETRO = "2026-10-19-160000";
const MONDAY_PLANNING = "2026-10-20-090000";
const TUESDAY_STANDUP = "2026-10-21-112155";
const WEDNESDAY_DEMO = "2026-10-22-140000";

function listedMeetingTitles(notebook: MountedNotebook): string[] {
  return Array.from(notebook.document.querySelectorAll(".session__title"), (title) => title.textContent ?? "");
}

function selectedMeetingInList(notebook: MountedNotebook): string | null | undefined {
  return notebook.document.querySelector('.session[aria-current="true"] .session__title')?.textContent;
}

function openMeetingTitle(notebook: MountedNotebook): string | null | undefined {
  return notebook.document.querySelector("#meeting-pane .meeting__title")?.textContent;
}

function textsOf(notebook: MountedNotebook, selector: string): string[] {
  return Array.from(notebook.document.querySelectorAll(selector), (node) => node.textContent ?? "");
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

function onlyElement<T extends Element>(notebook: MountedNotebook, selector: string): T {
  const found = notebook.document.querySelector<T>(selector);
  if (!found) {
    throw new Error("Nothing on the page matches " + selector);
  }
  return found;
}

describe("the notebook shows meetings from its database", () => {
  it("lists meetings newest first and opens the newest", async () => {
    const runtime = createFakeClaudeRuntime();
    seedMeeting(runtime.database, {
      id: MONDAY_PLANNING,
      meeting: aMeeting({ title: "Monday planning", started_at: "2026-10-20T09:00:00Z" }),
    });
    seedMeeting(runtime.database, {
      id: TUESDAY_STANDUP,
      meeting: aMeeting({ title: "Tuesday stand-up", started_at: "2026-10-21T11:21:55Z" }),
    });
    seedMeeting(runtime.database, {
      id: SUNDAY_RETRO,
      meeting: aMeeting({ title: "Sunday retro", started_at: "2026-10-19T16:00:00Z" }),
    });

    const notebook = await mountNotebook(runtime);

    expect(listedMeetingTitles(notebook)).toEqual(["Tuesday stand-up", "Monday planning", "Sunday retro"]);
    expect(selectedMeetingInList(notebook)).toBe("Tuesday stand-up");
    expect(openMeetingTitle(notebook)).toBe("Tuesday stand-up");
    expect(notebook.document.getElementById("meeting-count")?.textContent).toBe("3 meetings");
  });

  it("shows a meeting's speakers, summary, blocks and todos", async () => {
    const runtime = createFakeClaudeRuntime();
    seedMeeting(runtime.database, {
      id: TUESDAY_STANDUP,
      meeting: aMeeting({
        title: "Iteration ceremonies and QA bottleneck",
        type: "Team coaching",
        speakers: [
          { label: "Me", name: "", confidence: "stated", evidence: "" },
          { label: "Them", name: "Awni", confidence: "stated", evidence: "Introduced himself at the start." },
        ],
        summary: "The team agreed to split QA across pairs.",
        blocks: [
          { kind: "heading", text: "QA flow" },
          { kind: "paragraph", text: "Testing waits on one person." },
          { kind: "list", items: ["Pair on test plans", "Rotate QA duty"] },
          { kind: "decision", text: "Pairs own QA for their stories.", by: "Me" },
          { kind: "metric", label: "Stories waiting on QA", value: "7", note: "up from 3" },
          { kind: "quote", text: "We ship when QA frees up.", speaker: "Awni" },
          { kind: "table", columns: ["Owner", "Stories"], rows: [["Awni", 4], ["Me", 3]] },
          { kind: "screen", offset: "12:30", caption: "Sprint board" },
          { kind: "callout", tone: "risk", text: "Release slips if QA stays single-threaded." },
        ],
        todo_ids: ["t1", "t2"],
      }),
      todos: [
        aTodo(TUESDAY_STANDUP, { todo_id: "t2", text: "Share the QA rota", owner: "Awni", due: "Friday" }),
        aTodo(TUESDAY_STANDUP, { todo_id: "t1", text: "Draft the pairing plan", owner: "Me" }),
      ],
    });

    const notebook = await mountNotebook(runtime);

    expect(onlyElement(notebook, ".meeting__meta .badge").textContent).toBe("Team coaching");
    expect(textsOf(notebook, ".speaker__name")).toEqual(["Me", "Awni (named in the call)"]);
    expect(textsOf(notebook, ".speaker__evidence")).toEqual(["Introduced himself at the start."]);
    expect(textsOf(notebook, ".panel__body")).toEqual(["The team agreed to split QA across pairs."]);
    expect(textsOf(notebook, ".block-list > *")).toEqual([
      "QA flow",
      "Testing waits on one person.",
      "Pair on test plansRotate QA duty",
      "Decision · MePairs own QA for their stories.",
      "Stories waiting on QA7up from 3",
      "We ship when QA frees up.— Awni",
      "OwnerStoriesAwni4Me3",
      "12:30Screen: Sprint board",
      "RiskRelease slips if QA stays single-threaded.",
    ]);
    expect(Array.from(onlyElement(notebook, ".block-list").children, (block) => block.className)).toEqual([
      "block block--heading",
      "block",
      "block item-list",
      "block block--decision",
      "block block--metric",
      "block block--quote",
      "block block__table-scroll",
      "block block--screen",
      "block block--callout block--risk",
    ]);
    expect(textsOf(notebook, "#meeting-pane .todo__text")).toEqual(["Draft the pairing plan", "Share the QA rota"]);
    expect(textsOf(notebook, "#meeting-pane .todo__meta")).toEqual(["Owner: Me", "Owner: Awni · Due: Friday"]);
  });

  it("adds a meeting written while the page is open without losing the selection", async () => {
    const runtime = createFakeClaudeRuntime();
    seedMeeting(runtime.database, {
      id: MONDAY_PLANNING,
      meeting: aMeeting({ title: "Monday planning", started_at: "2026-10-20T09:00:00Z" }),
    });
    seedMeeting(runtime.database, {
      id: TUESDAY_STANDUP,
      meeting: aMeeting({ title: "Tuesday stand-up", started_at: "2026-10-21T11:21:55Z" }),
    });
    const notebook = await mountNotebook(runtime);
    meetingButton(notebook, "Monday planning").click();
    await notebook.settle();

    writeMeeting(runtime.database, {
      id: WEDNESDAY_DEMO,
      meeting: aMeeting({ title: "Wednesday demo", started_at: "2026-10-22T14:00:00Z" }),
    });
    await notebook.settle();

    expect(listedMeetingTitles(notebook)).toEqual(["Wednesday demo", "Tuesday stand-up", "Monday planning"]);
    expect(selectedMeetingInList(notebook)).toBe("Monday planning");
    expect(openMeetingTitle(notebook)).toBe("Monday planning");
    expect(notebook.document.getElementById("meeting-count")?.textContent).toBe("3 meetings");
  });

  it("updates a meeting rewritten while the page is open and keeps it open", async () => {
    const runtime = createFakeClaudeRuntime();
    seedMeeting(runtime.database, {
      id: MONDAY_PLANNING,
      meeting: aMeeting({ title: "Monday planning", started_at: "2026-10-20T09:00:00Z" }),
    });
    seedMeeting(runtime.database, {
      id: TUESDAY_STANDUP,
      meeting: aMeeting({ title: "Tuesday stand-up", started_at: "2026-10-21T11:21:55Z" }),
    });
    const notebook = await mountNotebook(runtime);

    writeMeeting(runtime.database, {
      id: TUESDAY_STANDUP,
      meeting: aMeeting({ title: "Tuesday stand-up on PRs", started_at: "2026-10-21T11:21:55Z" }),
    });
    await notebook.settle();

    expect(listedMeetingTitles(notebook)).toEqual(["Tuesday stand-up on PRs", "Monday planning"]);
    expect(selectedMeetingInList(notebook)).toBe("Tuesday stand-up on PRs");
    expect(openMeetingTitle(notebook)).toBe("Tuesday stand-up on PRs");
  });

  it("keeps your place in the open meeting when another meeting arrives", async () => {
    const runtime = createFakeClaudeRuntime();
    seedMeeting(runtime.database, {
      id: TUESDAY_STANDUP,
      meeting: aMeeting({ title: "Tuesday stand-up", started_at: "2026-10-21T11:21:55Z" }),
      todos: [aTodo(TUESDAY_STANDUP, { text: "Review the open PRs" })],
    });
    const notebook = await mountNotebook(runtime);
    const todoCheckbox = onlyElement<HTMLInputElement>(notebook, ".todo__checkbox");
    todoCheckbox.focus();

    writeMeeting(runtime.database, {
      id: MONDAY_PLANNING,
      meeting: aMeeting({ title: "Monday planning", started_at: "2026-10-20T09:00:00Z" }),
    });
    await notebook.settle();

    expect(listedMeetingTitles(notebook)).toEqual(["Tuesday stand-up", "Monday planning"]);
    expect(notebook.document.activeElement).toBe(todoCheckbox);
  });

  it("invites you to add a meeting when the notebook is empty", async () => {
    const runtime = createFakeClaudeRuntime();

    const notebook = await mountNotebook(runtime);

    expect(onlyElement(notebook, "#meeting-pane .empty__title").textContent).toBe("No meetings yet");
    expect(onlyElement(notebook, "#meeting-pane .empty__body").textContent).toBe(
      "Run /meeting-notes in Claude to add your first meeting.",
    );
    expect(notebook.document.querySelector(".meeting-list")).toBeNull();
    expect(notebook.document.getElementById("meeting-count")?.textContent).toBe("0 meetings");
  });

  it("explains that the notebook needs Claude when the database is unavailable", async () => {
    const runtime = createFakeClaudeRuntime({ database: "unavailable" });
    seedMeeting(runtime.database, { id: TUESDAY_STANDUP, meeting: aMeeting({ title: "Tuesday stand-up" }) });

    const notebook = await mountNotebook(runtime);

    expect(notebook.document.getElementById("meeting-pane")?.textContent).toBe(
      "This notebook needs Claude to load. Open it on claude.ai while signed in.",
    );
    expect(notebook.document.querySelector(".meeting-list")).toBeNull();
    expect(notebook.document.querySelector("#meeting-pane .empty__title")).toBeNull();
    expect(notebook.errors).toEqual([]);
  });

  it("shows markup in a meeting title as plain text", async () => {
    const runtime = createFakeClaudeRuntime();
    seedMeeting(runtime.database, {
      id: TUESDAY_STANDUP,
      meeting: aMeeting({ title: "<b>Launch</b> review" }),
      todos: [aTodo(TUESDAY_STANDUP, { text: '<img src="x" onerror="alert(1)">Ship it' })],
    });

    const notebook = await mountNotebook(runtime);

    expect(listedMeetingTitles(notebook)).toEqual(["<b>Launch</b> review"]);
    expect(openMeetingTitle(notebook)).toBe("<b>Launch</b> review");
    expect(textsOf(notebook, "#meeting-pane .todo__text")).toEqual(['<img src="x" onerror="alert(1)">Ship it']);
    expect(notebook.document.querySelector("b, img")).toBeNull();
  });

  it("runs an AI action through Claude and shows the answer", async () => {
    const runtime = createFakeClaudeRuntime({ answer: "Ticket drafted: share the QA rota by Friday." });
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

    onlyElement<HTMLButtonElement>(notebook, ".todo__actions .button").click();
    await notebook.settle();

    expect(textsOf(notebook, ".action-output__text")).toEqual(["Ticket drafted: share the QA rota by Friday."]);
  });

  it("disables AI actions with a reason when Claude is unavailable", async () => {
    const runtime = createFakeClaudeRuntime({ askClaude: "unavailable" });
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

    const actionButton = onlyElement<HTMLButtonElement>(notebook, ".todo__actions .button");
    const reasonId = actionButton.getAttribute("aria-describedby") ?? "";
    expect(actionButton.disabled).toBe(true);
    expect(notebook.document.getElementById(reasonId)?.textContent).toBe(
      "AI actions need Claude web or Desktop on a paid plan.",
    );
  });
});
