import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { axe } from "vitest-axe";
import { ScreenshotShortcuts } from "./ScreenshotShortcuts";
import { SettingsView } from "./SettingsView";
import type { RebindOutcome, ShortcutView } from "./types";

const FULL_SCREEN_DEFAULT = "⌃⌥⌘1";
const REGION_DEFAULT = "⌃⌥⌘2";
const REGION_CHANGE = `Change Capture region shortcut, currently ${REGION_DEFAULT}`;
const LISTENING = "Press the new shortcut…";

const shortcutsWithRegion = (label: string): readonly ShortcutView[] => [
  { kind: "full_screen", label: FULL_SCREEN_DEFAULT },
  { kind: "region", label },
];

function backend(
  rebind: () => RebindOutcome = () => ({ outcome: "accepted", label: "⌃R" }),
  failures: Readonly<Record<string, () => never>> = {},
) {
  let shortcuts = shortcutsWithRegion(REGION_DEFAULT);
  const responses: Readonly<Record<string, () => unknown>> = {
    screenshot_shortcuts: () => shortcuts,
    transcripts_folder: () => "/Users/me/Documents/Scribe",
    rebind_screenshot_shortcut: () => {
      const outcome = rebind();
      shortcuts = outcome.outcome === "accepted" ? shortcutsWithRegion(outcome.label) : shortcuts;
      return outcome;
    },
    reset_screenshot_shortcut: () => {
      shortcuts = shortcutsWithRegion(REGION_DEFAULT);
      return { outcome: "accepted", label: REGION_DEFAULT };
    },
    ...failures,
  };
  const handler = vi.fn((command: string) => responses[command]?.());
  mockIPC(handler);
  return handler;
}

function failing(message: string): () => never {
  return () => {
    throw new Error(message);
  };
}

function commandsIssued(handler: ReturnType<typeof backend>) {
  return handler.mock.calls.map(([command]) => command);
}

async function startListeningOnRegion() {
  await userEvent.click(await screen.findByRole("button", { name: REGION_CHANGE }));
  await screen.findByText(LISTENING);
}

afterEach(() => {
  cleanup();
  clearMocks();
});

describe("the screenshot shortcuts section", () => {
  it("lists both shortcuts with their current combination", async () => {
    backend();

    render(<ScreenshotShortcuts onStatus={vi.fn()} />);

    expect(
      await screen.findByRole("heading", { name: "Screenshot shortcuts" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Capture full screen")).toBeInTheDocument();
    expect(screen.getByText(FULL_SCREEN_DEFAULT)).toBeInTheDocument();
    expect(screen.getByText("Capture region")).toBeInTheDocument();
    expect(screen.getByText(REGION_DEFAULT)).toBeInTheDocument();
  });

  it("pauses the global shortcuts and waits for a new combination when Change is pressed", async () => {
    const handler = backend();
    render(<ScreenshotShortcuts onStatus={vi.fn()} />);

    await startListeningOnRegion();

    expect(commandsIssued(handler)).toContain("pause_screenshot_shortcuts");
    expect(commandsIssued(handler)).not.toContain("resume_screenshot_shortcuts");
    expect(screen.queryByText(REGION_DEFAULT)).not.toBeInTheDocument();
  });

  it("restores the old combination and resumes the shortcuts when Escape cancels", async () => {
    const handler = backend();
    const onStatus = vi.fn();
    render(<ScreenshotShortcuts onStatus={onStatus} />);
    await startListeningOnRegion();

    await userEvent.keyboard("{Escape}");

    expect(await screen.findByText(REGION_DEFAULT)).toBeInTheDocument();
    expect(screen.queryByText(LISTENING)).not.toBeInTheDocument();
    await waitFor(() =>
      expect(commandsIssued(handler)).toContain("resume_screenshot_shortcuts"),
    );
    expect(commandsIssued(handler)).not.toContain("rebind_screenshot_shortcut");
    expect(onStatus).toHaveBeenLastCalledWith("Shortcut change cancelled");
  });

  it("resumes the shortcuts when the section goes away while listening", async () => {
    const handler = backend();
    const { unmount } = render(<ScreenshotShortcuts onStatus={vi.fn()} />);
    await startListeningOnRegion();

    unmount();

    await waitFor(() =>
      expect(commandsIssued(handler)).toContain("resume_screenshot_shortcuts"),
    );
  });

  it("keeps listening when only a modifier is pressed", async () => {
    const handler = backend();
    render(<ScreenshotShortcuts onStatus={vi.fn()} />);
    await startListeningOnRegion();

    await userEvent.keyboard("{Shift}");

    expect(screen.getByText(LISTENING)).toBeInTheDocument();
    expect(commandsIssued(handler)).not.toContain("rebind_screenshot_shortcut");
  });

  it("sends the physical key and modifiers and shows the accepted combination", async () => {
    const handler = backend();
    const onStatus = vi.fn();
    render(<ScreenshotShortcuts onStatus={onStatus} />);
    await startListeningOnRegion();

    await userEvent.keyboard("{Control>}{Alt>}r{/Alt}{/Control}");

    expect(await screen.findByText("⌃R")).toBeInTheDocument();
    expect(handler).toHaveBeenCalledWith("rebind_screenshot_shortcut", {
      kind: "region",
      pressed: { code: "KeyR", control: true, alt: true, shift: false, meta: false },
    });
    expect(onStatus).toHaveBeenCalledWith("Capture region is now ⌃R");
    await waitFor(() =>
      expect(commandsIssued(handler)).toContain("resume_screenshot_shortcuts"),
    );
  });

  it.each<[string, RebindOutcome, string]>([
    [
      "a combination without a usable modifier",
      {
        outcome: "needs_modifier",
        message: "Use Ctrl or Alt with a letter, number or function key",
      },
      "Use Ctrl or Alt with a letter, number or function key",
    ],
    [
      "a combination the other row uses",
      { outcome: "duplicate_of", kind: "full_screen" },
      "Already used for Capture full screen",
    ],
    [
      "a combination another app holds",
      { outcome: "in_use_by_another_app" },
      "Another app is using this shortcut",
    ],
  ])("keeps the old combination and says why for %s", async (_name, outcome, message) => {
    const handler = backend(() => outcome);
    const onStatus = vi.fn();
    render(<ScreenshotShortcuts onStatus={onStatus} />);
    await startListeningOnRegion();

    await userEvent.keyboard("{Shift>}r{/Shift}");

    expect(await screen.findByText(REGION_DEFAULT)).toBeInTheDocument();
    expect(onStatus).toHaveBeenCalledWith(message);
    await waitFor(() =>
      expect(commandsIssued(handler)).toContain("resume_screenshot_shortcuts"),
    );
  });

  it("restores a row's default when Reset to default is pressed", async () => {
    const handler = backend();
    render(<ScreenshotShortcuts onStatus={vi.fn()} />);
    await startListeningOnRegion();
    await userEvent.keyboard("{Control>}r{/Control}");
    await screen.findByText("⌃R");

    await userEvent.click(
      screen.getByRole("button", { name: "Reset to default for Capture region shortcut" }),
    );

    expect(await screen.findByText(REGION_DEFAULT)).toBeInTheDocument();
    expect(handler).toHaveBeenCalledWith("reset_screenshot_shortcut", { kind: "region" });
  });

  it("moves focus to the row being changed and keeps it there afterwards", async () => {
    backend();
    render(<ScreenshotShortcuts onStatus={vi.fn()} />);
    await startListeningOnRegion();

    const row = screen.getByText("Capture region").closest("li");
    expect(row).toHaveFocus();

    await userEvent.keyboard("{Control>}r{/Control}");
    await screen.findByText("⌃R");

    expect(row).toHaveFocus();
  });

  it("announces that it is listening", async () => {
    backend();
    const onStatus = vi.fn();
    render(<ScreenshotShortcuts onStatus={onStatus} />);

    await startListeningOnRegion();

    expect(onStatus).toHaveBeenCalledWith(LISTENING);
  });

  it("stops listening and resumes the shortcuts when the window loses focus", async () => {
    const handler = backend();
    render(<ScreenshotShortcuts onStatus={vi.fn()} />);
    await startListeningOnRegion();

    fireEvent.blur(window);

    expect(await screen.findByText(REGION_DEFAULT)).toBeInTheDocument();
    expect(screen.queryByText(LISTENING)).not.toBeInTheDocument();
    await waitFor(() =>
      expect(commandsIssued(handler)).toContain("resume_screenshot_shortcuts"),
    );
    expect(commandsIssued(handler)).not.toContain("rebind_screenshot_shortcut");
  });

  it("says the shortcuts cannot be read when they fail to load", async () => {
    backend(undefined, { screenshot_shortcuts: failing("unavailable") });
    const onStatus = vi.fn();

    render(<ScreenshotShortcuts onStatus={onStatus} />);

    await waitFor(() =>
      expect(onStatus).toHaveBeenCalledWith("Scribe could not read the screenshot shortcuts."),
    );
  });

  it("says the shortcut could not be changed and keeps the shortcuts live when pausing fails", async () => {
    const handler = backend(undefined, { pause_screenshot_shortcuts: failing("cannot pause") });
    const onStatus = vi.fn();
    render(<ScreenshotShortcuts onStatus={onStatus} />);

    await userEvent.click(await screen.findByRole("button", { name: REGION_CHANGE }));

    await waitFor(() =>
      expect(onStatus).toHaveBeenCalledWith("The shortcut could not be changed."),
    );
    expect(screen.queryByText(LISTENING)).not.toBeInTheDocument();
    expect(commandsIssued(handler)).toContain("resume_screenshot_shortcuts");
  });

  it("says the shortcut could not be changed and keeps the old combination when rebinding fails", async () => {
    backend(undefined, { rebind_screenshot_shortcut: failing("cannot rebind") });
    const onStatus = vi.fn();
    render(<ScreenshotShortcuts onStatus={onStatus} />);
    await startListeningOnRegion();

    await userEvent.keyboard("{Control>}r{/Control}");

    await waitFor(() =>
      expect(onStatus).toHaveBeenCalledWith("The shortcut could not be changed."),
    );
    expect(await screen.findByText(REGION_DEFAULT)).toBeInTheDocument();
  });

  it("has no accessibility violations while idle or listening", async () => {
    backend();
    const { container } = render(<ScreenshotShortcuts onStatus={vi.fn()} />);
    await screen.findByRole("button", { name: REGION_CHANGE });
    const idle = await axe(container);

    await startListeningOnRegion();
    const listening = await axe(container);

    expect(idle.violations).toEqual([]);
    expect(listening.violations).toEqual([]);
  });
});

describe("the screenshot shortcuts in the settings view", () => {
  it("lets the shortcuts change while a recording is in progress", async () => {
    backend();

    render(<SettingsView isRecording onBack={vi.fn()} />);

    expect(await screen.findByRole("button", { name: REGION_CHANGE })).toBeEnabled();
    expect(
      screen.getByRole("button", { name: "Change Transcripts Folder" }),
    ).toBeDisabled();
  });

  it("announces the outcome in its own status region, apart from the folder status", async () => {
    backend();
    render(<SettingsView isRecording={false} onBack={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: REGION_CHANGE }));
    await screen.findAllByText(LISTENING);

    await userEvent.keyboard("{Control>}r{/Control}");

    const outcome = await screen.findByText("Capture region is now ⌃R");
    expect(outcome).toHaveAttribute("role", "status");
  });

  it("announces a repeated identical outcome again", async () => {
    backend(() => ({ outcome: "in_use_by_another_app" }));
    render(<SettingsView isRecording={false} onBack={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: REGION_CHANGE }));
    await screen.findAllByText(LISTENING);
    await userEvent.keyboard("{Control>}r{/Control}");
    const first = await screen.findByText("Another app is using this shortcut");

    await userEvent.click(await screen.findByRole("button", { name: REGION_CHANGE }));
    await screen.findAllByText(LISTENING);
    await userEvent.keyboard("{Control>}r{/Control}");

    await waitFor(() =>
      expect(screen.getByText("Another app is using this shortcut")).not.toBe(first),
    );
  });

  it("keeps Back as the last control in tab order, after the shortcuts", async () => {
    backend();
    render(<SettingsView isRecording={false} onBack={vi.fn()} />);
    const back = await screen.findByRole("button", { name: "Back" });
    const change = await screen.findByRole("button", { name: REGION_CHANGE });

    expect(change.compareDocumentPosition(back)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    expect(back.closest("section")?.lastElementChild).toBe(back);
  });
});
