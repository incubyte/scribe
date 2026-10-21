import { describe, expect, it } from "vitest";
import { createFakeClaudeRuntime } from "./fakeClaudeRuntime";
import { type MountedNotebook, mountNotebook } from "./mountNotebook";
import { aMeeting, aTodo, seedMeeting, writeMeeting } from "./notebookRecords";

const STANDUP = "2026-10-21-112155";
const TODO_DOC_ID = STANDUP + "--t1";

type Options = Parameters<typeof createFakeClaudeRuntime>[0];

async function openStandupWithOneTodo(options?: Options, todoOverrides = {}) {
  const runtime = createFakeClaudeRuntime(options);
  const stored = { id: STANDUP, meeting: aMeeting({ todo_ids: ["t1"] }), todos: [aTodo(STANDUP, todoOverrides)] };
  seedMeeting(runtime.database, stored);
  const notebook = await mountNotebook(runtime);
  return { runtime, notebook, stored };
}

function checkbox(notebook: MountedNotebook): HTMLInputElement {
  const found = notebook.document.querySelector<HTMLInputElement>(".todo__checkbox");
  if (!found) {
    throw new Error("The open meeting shows no todo checkbox");
  }
  return found;
}

async function clickCheckbox(notebook: MountedNotebook): Promise<void> {
  checkbox(notebook).click();
  await notebook.settle();
}

function storedTodo(runtime: Awaited<ReturnType<typeof openStandupWithOneTodo>>["runtime"]) {
  return runtime.database.read("todos", TODO_DOC_ID);
}

describe("ticking a todo saves it to the notebook database for everyone", () => {
  it("ticking a todo marks it done with the time it was ticked", async () => {
    const { runtime, notebook } = await openStandupWithOneTodo();

    await clickCheckbox(notebook);

    const saved = storedTodo(runtime);
    expect(saved?.done).toBe(true);
    expect(Number.isNaN(Date.parse(String(saved?.done_at)))).toBe(false);
  });

  it("unticking reopens the todo and clears the time", async () => {
    const { runtime, notebook } = await openStandupWithOneTodo(undefined, {
      done: true,
      done_at: "2026-10-21T12:00:00Z",
    });

    await clickCheckbox(notebook);

    expect(storedTodo(runtime)).toMatchObject({ done: false, done_at: null });
  });

  it("a todo ticked elsewhere shows as done here", async () => {
    const { runtime, notebook, stored } = await openStandupWithOneTodo();

    writeMeeting(runtime.database, {
      ...stored,
      todos: [aTodo(STANDUP, { done: true, done_at: "2026-10-21T12:00:00Z" })],
    });
    await notebook.settle();

    expect(checkbox(notebook).checked).toBe(true);
  });

  it("a view-only reader sees todos but cannot tick them", async () => {
    const { runtime, notebook } = await openStandupWithOneTodo({ canWriteData: false });
    const box = checkbox(notebook);
    const reasonId = box.getAttribute("aria-describedby") ?? "";

    await clickCheckbox(notebook);

    expect(box.disabled).toBe(true);
    expect(notebook.document.getElementById(reasonId)?.textContent).toBe(
      "You can view this notebook but not tick todos.",
    );
    expect(storedTodo(runtime)?.done).toBe(false);
  });

  it("a tick that fails to save is undone with a message", async () => {
    const { runtime, notebook } = await openStandupWithOneTodo();
    runtime.database.failNextUpdate("permission_denied");

    await clickCheckbox(notebook);

    expect(checkbox(notebook).checked).toBe(false);
    expect(notebook.document.querySelector(".todo__error")?.textContent).toBe("Couldn't save that change. Try again.");
    expect(storedTodo(runtime)?.done).toBe(false);
  });

  it("rapid ticks save the last choice", async () => {
    const { runtime, notebook } = await openStandupWithOneTodo();
    const box = checkbox(notebook);

    box.click();
    box.click();
    box.click();
    await notebook.settle();

    expect(storedTodo(runtime)?.done).toBe(true);
    expect(box.checked).toBe(true);
  });
});
