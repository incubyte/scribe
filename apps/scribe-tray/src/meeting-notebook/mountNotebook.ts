import { type DOMWindow, JSDOM, VirtualConsole } from "jsdom";
import notebookHtml from "../../../../plugins/scribe/skills/meeting-notes/notebook.html?raw";
import type { FakeClaudeRuntime } from "./fakeClaudeRuntime";

export type MountedNotebook = {
  document: Document;
  window: DOMWindow;
  errors: Error[];
  settle(): Promise<void>;
};

type MountOptions = { storedFilter?: string };

const NOTEBOOK_URL = "https://notebook.test/";
const TODO_FILTER_STORAGE_KEY = "scribe-notebook-todo-filter";

export async function mountNotebook(runtime: FakeClaudeRuntime, options: MountOptions = {}): Promise<MountedNotebook> {
  const errors: Error[] = [];
  const virtualConsole = new VirtualConsole().on("jsdomError", (error) => errors.push(error));
  const dom = new JSDOM(notebookHtml, {
    url: NOTEBOOK_URL,
    runScripts: "dangerously",
    virtualConsole,
    beforeParse: (window) => prepareWindow(window, runtime, options),
  });
  const settle = () => settleRuntime(runtime);
  await settle();
  return { document: dom.window.document, window: dom.window, errors, settle };
}

function prepareWindow(window: DOMWindow, runtime: FakeClaudeRuntime, options: MountOptions): void {
  Object.assign(window, { claude: runtime.claude });
  window.Element.prototype.scrollIntoView = () => undefined;
  if (options.storedFilter !== undefined) {
    window.localStorage.setItem(TODO_FILTER_STORAGE_KEY, options.storedFilter);
  }
}

async function settleRuntime(runtime: FakeClaudeRuntime): Promise<void> {
  do {
    await nextMacrotask();
  } while (!runtime.isIdle());
}

function nextMacrotask(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}
