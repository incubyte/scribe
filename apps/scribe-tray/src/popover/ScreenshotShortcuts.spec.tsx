import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
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

function backend(rebind: () => RebindOutcome = () => ({ outcome: "accepted", label: "⌃R" })) {
  let shortcuts = shortcutsWithRegion(REGION_DEFAULT);
  const handler = vi.fn((command: string) => {
    if (command === "screenshot_shortcuts") {
      return shortcuts;
    }
    if (command === "transcripts_folder") {
      return "/Users/me/Documents/Scribe";
    }
    if (command === "rebind_screenshot_shortcut") {
      const outcome = rebind();
      if (outcome.outcome === "accepted") {
        shortcuts = shortcutsWithRegion(outcome.label);
      }
      return outcome;
    }
    if (command === "reset_screenshot_shortcut") {
      shortcuts = shortcutsWithRegion(REGION_DEFAULT);
      return { outcome: "accepted", label: REGION_DEFAULT };
    }
    return undefined;
  });
  mockIPC(handler);
  return handler;
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
    render(<ScreenshotShortcuts onStatus={vi.fn()} />);
    await startListeningOnRegion();

    await userEvent.keyboard("{Escape}");

    expect(await screen.findByText(REGION_DEFAULT)).toBeInTheDocument();
    expect(screen.queryByText(LISTENING)).not.toBeInTheDocument();
    await waitFor(() =>
      expect(commandsIssued(handler)).toContain("resume_screenshot_shortcuts"),
    );
    expect(commandsIssued(handler)).not.toContain("rebind_screenshot_shortcut");
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
      { outcome: "needs_modifier" },
      "Use ⌘, ⌃ or ⌥ with a letter, number or function key",
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
      screen.getByRole("button", { name: "Reset Capture region shortcut to default" }),
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

  it("announces the outcome on the settings status line", async () => {
    backend();
    render(<SettingsView isRecording={false} onBack={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: REGION_CHANGE }));
    await screen.findByText(LISTENING);

    await userEvent.keyboard("{Control>}r{/Control}");

    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent("Capture region is now ⌃R"),
    );
  });
});
