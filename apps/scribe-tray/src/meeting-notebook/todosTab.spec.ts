import { describe, expect, it } from "vitest";
import { createFakeClaudeRuntime } from "./fakeClaudeRuntime";
import { type MountedNotebook, mountNotebook } from "./mountNotebook";
import { aMeeting, aTodo, seedMeeting, type TodoBody } from "./notebookRecords";

const NEWER_MEETING = "2026-10-21-112155";
const OLDER_MEETING = "2026-10-19-160000";
const NEWER_START = "2026-10-21T11:21:55Z";
const OLDER_START = "2026-10-19T16:00:00Z";

type TodoSeed = Partial<TodoBody> & { todo_id: string };

function seedTodos(
  runtime: ReturnType<typeof createFakeClaudeRuntime>,
  meetingId: string,
  startedAt: string,
  todos: TodoSeed[],
  title = "Weekly sync",
) {
  seedMeeting(runtime.database, {
    id: meetingId,
    meeting: aMeeting({ title, started_at: startedAt, todo_ids: todos.map(({ todo_id }) => todo_id) }),
    todos: todos.map((todo) =>
      aTodo(meetingId, { meeting_title: title, meeting_started_at: startedAt, ...todo }),
    ),
  });
}

async function openNotebookWith(todos: TodoSeed[], options: { storedFilter?: string } = {}) {
  const runtime = createFakeClaudeRuntime();
  seedTodos(runtime, NEWER_MEETING, NEWER_START, todos);
  const notebook = await mountNotebook(runtime, options);
  return { runtime, notebook };
}

function textsOf(notebook: MountedNotebook, selector: string): string[] {
  return Array.from(notebook.document.querySelectorAll(selector), (node) => node.textContent ?? "");
}

function onlyElement<T extends Element>(notebook: MountedNotebook, selector: string): T {
  const found = notebook.document.querySelector<T>(selector);
  if (!found) {
    throw new Error("Nothing on the page matches " + selector);
  }
  return found;
}

function pressKeyOnTabs(notebook: MountedNotebook, key: string): void {
  const event = new notebook.window.KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
  onlyElement(notebook, "#tabs [aria-selected='true']").dispatchEvent(event);
}

async function chooseFilter(notebook: MountedNotebook, label: string): Promise<void> {
  const option = Array.from(notebook.document.querySelectorAll<HTMLButtonElement>(".filter__option")).find(
    (candidate) => candidate.textContent === label,
  );
  option?.click();
  await notebook.settle();
}

function todosTabCheckbox(notebook: MountedNotebook): HTMLInputElement {
  return onlyElement<HTMLInputElement>(notebook, "#todos-list .todo__checkbox");
}

function meetingsTabCheckbox(notebook: MountedNotebook): HTMLInputElement {
  return onlyElement<HTMLInputElement>(notebook, "#panel-meetings .todo__checkbox");
}

describe("the Todos tab lists every todo across meetings", () => {
  it("the Todos tab shows how many todos are still open", async () => {
    const { notebook } = await openNotebookWith([
      { todo_id: "t1" },
      { todo_id: "t2" },
      { todo_id: "t3", done: true, done_at: "2026-10-21T12:00:00Z" },
    ]);

    expect(onlyElement(notebook, "#tab-todos").textContent).toBe("Todos (2)");
  });

  it("the tabs are announced as a tab list and switch with the arrow keys", async () => {
    const { notebook } = await openNotebookWith([{ todo_id: "t1" }]);
    const todosPanel = onlyElement<HTMLElement>(notebook, "#panel-todos");

    pressKeyOnTabs(notebook, "ArrowRight");

    expect(onlyElement(notebook, "[role='tablist']").getAttribute("aria-label")).toBe("Notebook views");
    expect(onlyElement(notebook, "#tab-todos").getAttribute("aria-selected")).toBe("true");
    expect(onlyElement(notebook, "#tab-meetings").getAttribute("aria-selected")).toBe("false");
    expect(onlyElement(notebook, "#tab-todos").getAttribute("aria-controls")).toBe("panel-todos");
    expect(todosPanel.hidden).toBe(false);
    expect(onlyElement<HTMLElement>(notebook, "#panel-meetings").hidden).toBe(true);
    expect(notebook.document.activeElement?.id).toBe("tab-todos");

    pressKeyOnTabs(notebook, "ArrowLeft");

    expect(onlyElement(notebook, "#tab-meetings").getAttribute("aria-selected")).toBe("true");
    expect(todosPanel.hidden).toBe(true);
  });

  it("your own todos come first, then others by name, then unassigned", async () => {
    const { notebook } = await openNotebookWith([
      { todo_id: "t1", owner: "Unassigned" },
      { todo_id: "t2", owner: "Zed" },
      { todo_id: "t3", owner: "Me" },
      { todo_id: "t4", owner: "" },
      { todo_id: "t5", owner: "Ana" },
    ]);

    expect(textsOf(notebook, "#todos-list .todo-group__heading")).toEqual([
      "Mine (1)",
      "Waiting on others (2)",
      "Unassigned (2)",
    ]);
    expect(textsOf(notebook, "#todos-list .todo-group__owner")).toEqual(["Ana (1)", "Zed (1)"]);
  });

  it("todos from the newest meeting come first within a group", async () => {
    const runtime = createFakeClaudeRuntime();
    seedTodos(runtime, OLDER_MEETING, OLDER_START, [{ todo_id: "t1", text: "Older first" }], "Retro");
    seedTodos(runtime, NEWER_MEETING, NEWER_START, [
      { todo_id: "t1", text: "Newer first" },
      { todo_id: "t2", text: "Newer second" },
    ]);
    const notebook = await mountNotebook(runtime);

    expect(textsOf(notebook, "#todos-list .todo__text")).toEqual(["Newer first", "Newer second", "Older first"]);
  });

  it("the todo list opens on Open", async () => {
    const { notebook } = await openNotebookWith([
      { todo_id: "t1", text: "Still to do" },
      { todo_id: "t2", text: "Finished", done: true, done_at: "2026-10-21T12:00:00Z" },
    ]);

    expect(textsOf(notebook, "#todos-list .todo__text")).toEqual(["Still to do"]);
    expect(onlyElement(notebook, ".filter__option[aria-pressed='true']").textContent).toBe("Open");
  });

  it("the todo list remembers Done for next time", async () => {
    const todos = [
      { todo_id: "t1", text: "Still to do" },
      { todo_id: "t2", text: "Finished", done: true, done_at: "2026-10-21T12:00:00Z" },
    ];
    const first = await openNotebookWith(todos);

    await chooseFilter(first.notebook, "Done");

    expect(first.notebook.window.localStorage.getItem("scribe-notebook-todo-filter")).toBe("done");
    const second = await openNotebookWith(todos, { storedFilter: "done" });
    expect(textsOf(second.notebook, "#todos-list .todo__text")).toEqual(["Finished"]);
    expect(onlyElement(second.notebook, ".filter__option[aria-pressed='true']").textContent).toBe("Done");
  });

  it("an unreadable stored filter falls back to Open", async () => {
    const { notebook } = await openNotebookWith([{ todo_id: "t1" }], { storedFilter: "everything" });

    expect(onlyElement(notebook, ".filter__option[aria-pressed='true']").textContent).toBe("Open");
  });

  it("each todo links to its meeting and shows its due phrase and actions", async () => {
    const { notebook } = await openNotebookWith([
      { todo_id: "t1", due: "by Friday", actions: [{ label: "Draft the recap", prompt: "Write the recap" }] },
    ]);

    expect(onlyElement(notebook, "#todos-list .todo__meeting-link").textContent).toContain("Weekly sync");
    expect(onlyElement(notebook, "#todos-list .todo__meta").textContent).toContain("Due: by Friday");
    expect(onlyElement(notebook, "#todos-list .todo__actions .button").textContent).toContain("Draft the recap");
  });

  it("following a todo's meeting link opens that meeting and focuses the todo", async () => {
    const runtime = createFakeClaudeRuntime();
    seedTodos(runtime, OLDER_MEETING, OLDER_START, [{ todo_id: "t1", text: "Buried follow-up" }], "Retro");
    Array.from({ length: 22 }, (_, day) => day + 1).forEach((day) => {
      const started = "2026-09-" + String(day).padStart(2, "0") + "T10:00:00Z";
      seedTodos(runtime, "2026-09-" + String(day).padStart(2, "0") + "-100000", started, [], "Daily " + day);
    });
    const notebook = await mountNotebook(runtime);
    pressKeyOnTabs(notebook, "ArrowRight");

    onlyElement<HTMLAnchorElement>(notebook, "#todos-list .todo__meeting-link").click();
    await notebook.settle();

    expect(onlyElement(notebook, "#tab-meetings").getAttribute("aria-selected")).toBe("true");
    expect(onlyElement(notebook, "#meeting-pane .meeting__title").textContent).toBe("Retro");
    expect(notebook.document.activeElement).toBe(meetingsTabCheckbox(notebook));
    expect(meetingsTabCheckbox(notebook).closest(".todo")?.classList.contains("todo--highlight")).toBe(true);
  });

  it("following the link of a todo from an older meeting reveals that meeting in the list", async () => {
    const runtime = createFakeClaudeRuntime();
    seedTodos(runtime, "2026-08-01-100000", "2026-08-01T10:00:00Z", [{ todo_id: "t1", text: "Ancient follow-up" }], "Ancient retro");
    Array.from({ length: 22 }, (_, day) => day + 1).forEach((day) => {
      const dayText = String(day).padStart(2, "0");
      seedTodos(runtime, "2026-09-" + dayText + "-100000", "2026-09-" + dayText + "T10:00:00Z", [], "Daily " + day);
    });
    const notebook = await mountNotebook(runtime);
    pressKeyOnTabs(notebook, "ArrowRight");

    onlyElement<HTMLAnchorElement>(notebook, "#todos-list .todo__meeting-link").click();
    await notebook.settle();

    expect(onlyElement(notebook, "#meeting-pane .meeting__title").textContent).toBe("Ancient retro");
    expect(onlyElement(notebook, '.session[aria-current="true"] .session__title').textContent).toBe("Ancient retro");
    expect(onlyElement<HTMLSelectElement>(notebook, "#meeting-select").selectedOptions[0]?.textContent).toContain(
      "Ancient retro",
    );
  });

  it("ticking a todo under Open keeps keyboard focus on its checkbox", async () => {
    const { notebook } = await openNotebookWith([{ todo_id: "t1" }, { todo_id: "t2" }]);
    const box = todosTabCheckbox(notebook);
    box.focus();

    box.click();
    await notebook.settle();

    expect(notebook.document.activeElement).toBe(box);
    expect(box.checked).toBe(true);
    expect(onlyElement(notebook, "#tab-todos").textContent).toBe("Todos (1)");
  });

  it("a ticked todo leaves the Open list once the filter changes", async () => {
    const { notebook } = await openNotebookWith([{ todo_id: "t1", text: "First" }, { todo_id: "t2", text: "Second" }]);
    todosTabCheckbox(notebook).click();
    await notebook.settle();

    await chooseFilter(notebook, "All");
    await chooseFilter(notebook, "Open");

    expect(textsOf(notebook, "#todos-list .todo__text")).toEqual(["Second"]);
  });

  it("a todo whose meeting was removed says so beside its link", async () => {
    const runtime = createFakeClaudeRuntime();
    runtime.database.seed("todos", "gone--t1", aTodo("gone", { meeting_title: "Removed sync" }));
    seedTodos(runtime, NEWER_MEETING, NEWER_START, [{ todo_id: "t1" }]);
    const notebook = await mountNotebook(runtime);
    const removedLink = Array.from(
      notebook.document.querySelectorAll<HTMLAnchorElement>("#todos-list .todo__meeting-link"),
    ).find((link) => link.textContent?.includes("Removed sync"));

    removedLink?.click();
    await notebook.settle();

    expect(removedLink?.nextElementSibling?.textContent).toBe("That meeting is no longer in the notebook");
    expect(onlyElement(notebook, "#tab-todos").getAttribute("aria-selected")).toBe("false");
  });

  it("ticking a todo in one tab updates the other and the open count", async () => {
    const { notebook } = await openNotebookWith([{ todo_id: "t1" }], { storedFilter: "all" });

    meetingsTabCheckbox(notebook).click();
    await notebook.settle();

    expect(todosTabCheckbox(notebook).checked).toBe(true);
    expect(onlyElement(notebook, "#tab-todos").textContent).toBe("Todos (0)");

    todosTabCheckbox(notebook).click();
    await notebook.settle();

    expect(meetingsTabCheckbox(notebook).checked).toBe(false);
    expect(onlyElement(notebook, "#tab-todos").textContent).toBe("Todos (1)");
  });

  it("an empty filter says there is nothing to show", async () => {
    const { notebook } = await openNotebookWith([
      { todo_id: "t1", done: true, done_at: "2026-10-21T12:00:00Z" },
    ]);

    expect(onlyElement(notebook, "#todos-list .empty__body").textContent).toBe("No open todos");

    await chooseFilter(notebook, "All");
    expect(notebook.document.querySelector("#todos-list .empty__body")).toBeNull();
  });
});
