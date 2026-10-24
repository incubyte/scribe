import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import {
  act,
  cleanup,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { axe } from "vitest-axe";
import { Popover } from "./Popover";
import { CHANGE_LABEL } from "./SettingsPanel";
import type { ModelDownload, RecorderSnapshot, SessionSummary } from "./types";

const NOW = Date.parse("2026-10-17T09:10:00+00:00");

function session(day: number, durationSeconds: number | null): SessionSummary {
  return {
    id: `2026-10-${day}-120000`,
    title: `Call 2026-10-${day} 12:00`,
    started_at: `2026-10-${day}T12:00:00+00:00`,
    duration_seconds: durationSeconds,
    status: durationSeconds === null ? "recording" : "complete",
  };
}

const FIVE_SESSIONS = [
  session(17, 1961),
  session(16, 600),
  session(15, 600),
  session(14, 600),
  session(13, 600),
];

const INITIAL_FOLDER = "/Users/me/Documents/Scribe";
const CHANGED_FOLDER = "/Users/me/Notes/Calls";
let currentFolder = INITIAL_FOLDER;
let readFolder: () => unknown = () => currentFolder;
const defaultReadFolder = readFolder;
let changeFolder: () => unknown = () => {
  currentFolder = CHANGED_FOLDER;
  return { outcome: "changed", folder: CHANGED_FOLDER, moved: 0, kept: 0 };
};
const defaultChangeFolder = changeFolder;

const READY: ModelDownload = { state: "ready" };

const idle: RecorderSnapshot = { state: "idle", started_at: null, lines: [] };
const locked: RecorderSnapshot = {
  state: "locked",
  started_at: null,
  lines: [],
};
const recordingForSixtyFiveSeconds: RecorderSnapshot = {
  state: "recording",
  started_at: new Date(NOW - 65_000).toISOString(),
  lines: [{ speaker: "Them", start_ms: 0, text: "Good morning" }],
};

const SCRIBE_VERSION = "0.5.1";

function backend(
  snapshot: RecorderSnapshot,
  recent: readonly SessionSummary[] = FIVE_SESSIONS,
  modelDownload: ModelDownload = READY,
) {
  const responses: Readonly<Record<string, unknown>> = {
    recorder_snapshot: snapshot,
    model_download_status: modelDownload,
    recent_sessions: recent,
    claude_desktop_status: "not_connected",
    "plugin:app|version": SCRIBE_VERSION,
    transcripts_folder: () => readFolder(),
    change_transcripts_folder: () => changeFolder(),
    screenshot_shortcuts: [
      { kind: "full_screen", label: "⌃⌥⌘1" },
      { kind: "region", label: "⌃⌥⌘2" },
    ],
  };
  const handler = vi.fn((command: string) => {
    const response = responses[command];
    return typeof response === "function" ? response() : response;
  });
  mockIPC(handler, { shouldMockEvents: true });
  return handler;
}

beforeEach(() => {
  currentFolder = INITIAL_FOLDER;
  readFolder = defaultReadFolder;
  changeFolder = defaultChangeFolder;
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(NOW);
});

afterEach(async () => {
  cleanup();
  await new Promise((resolve) => setTimeout(resolve));
  vi.useRealTimers();
  clearMocks();
});

describe("the popover while nothing is recording", () => {
  it("offers Start and the five most recent sessions", async () => {
    backend(idle);
    render(<Popover />);

    const sessions = await screen.findByRole("list", {
      name: "Recent sessions",
    });

    expect(
      screen.getByRole("button", { name: "Start recording" }),
    ).toBeEnabled();
    expect(within(sessions).getAllByRole("button")).toHaveLength(5);
    expect(
      within(sessions).getByText("Call 2026-10-17 12:00"),
    ).toBeInTheDocument();
    expect(within(sessions).getByText(/Oct 17/)).toBeInTheDocument();
    expect(within(sessions).getByText("32 min")).toBeInTheDocument();
  });

  it("opens a session's file when it is chosen", async () => {
    const handler = backend(idle);
    render(<Popover />);

    await userEvent.click(
      await screen.findByRole("button", { name: /Call 2026-10-16 12:00/ }),
    );

    expect(handler).toHaveBeenCalledWith("open_session", {
      id: "2026-10-16-120000",
    });
  });

  it("starts a recording when Start is pressed", async () => {
    const handler = backend(idle);
    render(<Popover />);

    await userEvent.click(
      await screen.findByRole("button", { name: "Start recording" }),
    );

    expect(handler).toHaveBeenCalledWith("start_recording", {});
  });

  it("disables Start while the app is not ready to record", async () => {
    backend(locked);
    render(<Popover />);

    await screen.findByRole("list", { name: "Recent sessions" });

    expect(
      screen.getByRole("button", { name: "Start recording" }),
    ).toBeDisabled();
  });

  it("enables Start once the app becomes ready to record", async () => {
    backend(locked);
    render(<Popover />);
    await screen.findByRole("list", { name: "Recent sessions" });

    await act(() => emit("recorder://state", { state: "idle" }));

    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Start recording" }),
      ).toBeEnabled(),
    );
  });

  it("keeps focus on the recording button when the recording starts and announces it", async () => {
    backend(idle);
    render(<Popover />);
    await screen.findByRole("list", { name: "Recent sessions" });
    await userEvent.tab();

    await act(() => emit("recorder://state", { state: "recording" }));
    await act(() => emit("recording://started", { id: "2026-10-17-091000" }));

    expect(
      await screen.findByRole("button", { name: "Stop recording" }),
    ).toHaveFocus();
    expect(
      screen.getByRole("status", { name: "Recording status" }),
    ).toHaveTextContent("Recording started");
  });

  it("says why a recording could not start", async () => {
    backend(idle);
    render(<Popover />);
    await screen.findByRole("list", { name: "Recent sessions" });

    await act(() =>
      emit("recording://start-failed", { reason_kind: "microphone_denied" }),
    );

    expect(
      await screen.findByText("Scribe can't use the microphone."),
    ).toBeInTheDocument();
  });

  it("says so when there are no sessions yet", async () => {
    backend(idle, []);
    render(<Popover />);

    expect(await screen.findByText("No sessions yet")).toBeInTheDocument();
  });

  it("moves focus from Start to the recent sessions", async () => {
    backend(idle);
    render(<Popover />);
    await screen.findByRole("list", { name: "Recent sessions" });

    await userEvent.tab();
    expect(
      screen.getByRole("button", { name: "Start recording" }),
    ).toHaveFocus();

    await userEvent.tab();
    expect(
      screen.getByRole("button", { name: /Call 2026-10-17 12:00/ }),
    ).toHaveFocus();
  });
});

describe("the popover while recording", () => {
  it("shows the elapsed time and a Stop button", async () => {
    backend(recordingForSixtyFiveSeconds);
    render(<Popover />);

    expect(await screen.findByText("01:05")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Stop recording" }),
    ).toBeEnabled();
  });

  it("stops the recording when Stop is pressed", async () => {
    const handler = backend(recordingForSixtyFiveSeconds);
    render(<Popover />);

    await userEvent.click(
      await screen.findByRole("button", { name: "Stop recording" }),
    );

    expect(handler).toHaveBeenCalledWith("stop_recording", {});
  });

  it("shows who said what with a text label for each speaker", async () => {
    backend(recordingForSixtyFiveSeconds);
    render(<Popover />);

    const line = await screen.findByText("Good morning");

    expect(line.closest("li")).toHaveTextContent("Them");
  });

  it("shows a partial line as it is spoken and swaps in the final wording", async () => {
    backend(recordingForSixtyFiveSeconds);
    render(<Popover />);
    await screen.findByText("Good morning");

    await act(() =>
      emit("recording://partial", { speaker: "Me", text: "hello wor" }),
    );
    expect(await screen.findByText("hello wor")).toBeInTheDocument();

    await act(() =>
      emit("recording://segment", {
        speaker: "Me",
        start_ms: 900,
        text: "hello world",
      }),
    );

    await waitFor(() =>
      expect(screen.queryByText("hello wor")).not.toBeInTheDocument(),
    );
    expect(screen.getAllByText("hello world").length).toBeGreaterThan(0);
  });

  it("announces only finished lines to screen readers", async () => {
    backend(recordingForSixtyFiveSeconds);
    render(<Popover />);
    await screen.findByText("Good morning");

    await act(() =>
      emit("recording://partial", { speaker: "Me", text: "still talk" }),
    );
    const partialLine = await screen.findByText("still talk");
    await act(() =>
      emit("recording://segment", {
        speaker: "Them",
        start_ms: 900,
        text: "All done",
      }),
    );

    const transcript = screen.getByRole("region", { name: "Live transcript" });
    const announcements = transcript.querySelector('[aria-live="polite"]');
    expect(announcements).toHaveTextContent("Them: All done");
    expect(announcements).not.toHaveTextContent("still talk");
    expect(partialLine.closest("li")).toHaveAttribute("aria-hidden", "true");
  });

  it("returns to Start and refreshes recent sessions when the recording stops", async () => {
    const handler = backend(recordingForSixtyFiveSeconds);
    render(<Popover />);
    await screen.findByRole("button", { name: "Stop recording" });
    const callsBefore = handler.mock.calls.filter(
      ([c]) => c === "recent_sessions",
    ).length;

    await act(() => emit("recording://stopped", { id: "2026-10-17-091000" }));

    expect(
      await screen.findByRole("button", { name: "Start recording" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("status", { name: "Recording status" }),
    ).toHaveTextContent("Recording stopped");
    const callsAfter = handler.mock.calls.filter(
      ([c]) => c === "recent_sessions",
    ).length;
    expect(callsAfter).toBeGreaterThan(callsBefore);
  });

  it("moves focus from Stop to the transcript", async () => {
    backend(recordingForSixtyFiveSeconds);
    render(<Popover />);
    await screen.findByText("Good morning");

    await userEvent.tab();
    expect(
      screen.getByRole("button", { name: "Stop recording" }),
    ).toHaveFocus();

    await userEvent.tab();
    expect(
      screen.getByRole("region", { name: "Live transcript" }),
    ).toHaveFocus();
  });

  it("has no accessibility violations", async () => {
    backend(recordingForSixtyFiveSeconds);
    const { container } = render(<Popover />);
    await screen.findByText("Good morning");

    const { violations } = await axe(container);

    expect(violations).toEqual([]);
  });
});

describe("the popover while the speech model downloads", () => {
  it("shows the download progress and keeps Start disabled", async () => {
    backend(locked, FIVE_SESSIONS, { state: "downloading", percent: 42 });
    render(<Popover />);

    expect(
      await screen.findByRole("progressbar", { name: "Speech model download" }),
    ).toHaveAttribute("value", "42");
    expect(
      screen.getByRole("button", { name: "Start recording" }),
    ).toBeDisabled();
  });

  it("follows the progress as the download advances", async () => {
    backend(locked, FIVE_SESSIONS, { state: "downloading", percent: 42 });
    render(<Popover />);
    await screen.findByRole("progressbar", { name: "Speech model download" });

    await act(() =>
      emit("model-download://progressed", {
        state: "downloading",
        percent: 43,
      }),
    );

    await waitFor(() =>
      expect(
        screen.getByRole("progressbar", { name: "Speech model download" }),
      ).toHaveAttribute("value", "43"),
    );
  });

  it("offers Retry after a failure and asks the app to download again", async () => {
    const handler = backend(locked, FIVE_SESSIONS, {
      state: "failed",
      reason_kind: "network",
    });
    render(<Popover />);

    await userEvent.click(await screen.findByRole("button", { name: "Retry" }));

    expect(handler).toHaveBeenCalledWith("retry_model_download", {});
  });

  it("removes the panel once the model is ready", async () => {
    backend(locked, FIVE_SESSIONS, { state: "downloading", percent: 99 });
    render(<Popover />);
    await screen.findByRole("progressbar", { name: "Speech model download" });

    await act(() => emit("model-download://completed", { state: "ready" }));

    await waitFor(() =>
      expect(screen.queryByRole("progressbar")).not.toBeInTheDocument(),
    );
  });
});

describe("dismissing the popover", () => {
  it("asks for the popover to be hidden when Escape is pressed", async () => {
    const handler = backend(idle);
    render(<Popover />);
    await screen.findByRole("list", { name: "Recent sessions" });

    await userEvent.keyboard("{Escape}");

    expect(handler).toHaveBeenCalledWith("hide_popover", {});
  });
});

describe("the popover while a shortcut is being rebound", () => {
  it("keeps the popover open when Escape cancels the rebind", async () => {
    const handler = backend(idle);
    render(<Popover />);
    await userEvent.click(
      await screen.findByRole("button", { name: "Settings" }),
    );
    await userEvent.click(
      await screen.findByRole("button", {
        name: "Change Capture region shortcut, currently ⌃⌥⌘2",
      }),
    );
    await screen.findAllByText("Press the new shortcut…");

    await userEvent.keyboard("{Escape}");

    await waitFor(() =>
      expect(screen.queryByText("Press the new shortcut…")).not.toBeInTheDocument(),
    );
    expect(handler).not.toHaveBeenCalledWith("hide_popover", {});
  });
});

describe("the idle popover accessibility", () => {
  it("has no accessibility violations", async () => {
    backend(idle);
    const { container } = render(<Popover />);
    await screen.findByRole("list", { name: "Recent sessions" });

    const { violations } = await axe(container);

    expect(violations).toEqual([]);
  });
});

describe("the popover settings", () => {
  async function openSettings() {
    await userEvent.click(
      await screen.findByRole("button", { name: "Settings" }),
    );
  }

  it("shows which version of Scribe is running", async () => {
    backend(idle);
    render(<Popover />);

    await openSettings();

    expect(await screen.findByText("Version 0.5.1")).toBeInTheDocument();
  });

  it("shows the current transcripts folder", async () => {
    backend(idle);
    render(<Popover />);

    await openSettings();

    expect(
      await screen.findByText(INITIAL_FOLDER, { exact: false }),
    ).toBeInTheDocument();
  });

  it("shows the new folder after it is changed", async () => {
    backend(idle);
    render(<Popover />);
    await openSettings();

    await userEvent.click(
      screen.getByRole("button", { name: CHANGE_LABEL }),
    );

    expect(
      await screen.findByText(`Transcripts folder: ${CHANGED_FOLDER}`),
    ).toBeInTheDocument();
  });

  it("announces when the transcripts folder cannot be read", async () => {
    backend(idle);
    readFolder = () => {
      throw "settings unreadable";
    };
    render(<Popover />);

    await openSettings();

    await waitFor(() =>
      expect(folderStatus()).toHaveTextContent(
        "Scribe could not read the transcripts folder setting.",
      ),
    );
  });

  const folderStatus = () => screen.getByRole("status", { name: "Folder status" });

  async function changeFolderAndReadStatus(outcome: () => unknown) {
    backend(idle);
    changeFolder = outcome;
    render(<Popover />);
    await openSettings();
    await userEvent.click(
      screen.getByRole("button", { name: CHANGE_LABEL }),
    );
    return folderStatus();
  }

  it("announces the new folder and how many transcripts moved", async () => {
    const status = await changeFolderAndReadStatus(() => ({
      outcome: "changed",
      folder: CHANGED_FOLDER,
      moved: 3,
      kept: 0,
    }));

    await waitFor(() =>
      expect(status).toHaveTextContent(
        `Transcripts will be saved in ${CHANGED_FOLDER}. Moved 3 transcripts.`,
      ),
    );
  });

  it("uses the singular when one transcript moved", async () => {
    const status = await changeFolderAndReadStatus(() => ({
      outcome: "changed",
      folder: CHANGED_FOLDER,
      moved: 1,
      kept: 0,
    }));

    await waitFor(() =>
      expect(status).toHaveTextContent("Moved 1 transcript."),
    );
  });

  it("announces only the new folder when nothing moved", async () => {
    const status = await changeFolderAndReadStatus(() => ({
      outcome: "changed",
      folder: CHANGED_FOLDER,
      moved: 0,
      kept: 0,
    }));

    await waitFor(() =>
      expect(status).toHaveTextContent(/^Transcripts will be saved in \S+\.$/),
    );
  });

  it("tells the user to stop recording when the backend refuses the change", async () => {
    const status = await changeFolderAndReadStatus(() => ({
      outcome: "recording",
    }));

    await waitFor(() =>
      expect(status).toHaveTextContent("Stop recording to change the folder."),
    );
  });

  it("announces a failure when the folder could not be changed", async () => {
    const status = await changeFolderAndReadStatus(() => {
      throw "disk full";
    });

    await waitFor(() =>
      expect(status).toHaveTextContent("The folder could not be changed."),
    );
  });

  it("says nothing when the folder picker is cancelled", async () => {
    const status = await changeFolderAndReadStatus(() => ({
      outcome: "cancelled",
    }));

    await waitFor(() => expect(status).toBeEmptyDOMElement());
  });

  it("clears the previous message when a later change is cancelled", async () => {
    const status = await changeFolderAndReadStatus(() => ({
      outcome: "recording",
    }));
    await waitFor(() => expect(status).toHaveTextContent("Stop recording"));
    changeFolder = () => ({ outcome: "unchanged" });

    await userEvent.click(
      screen.getByRole("button", { name: CHANGE_LABEL }),
    );

    await waitFor(() => expect(status).toBeEmptyDOMElement());
  });

  it("returns focus to the settings button when going back", async () => {
    backend(idle);
    render(<Popover />);
    await openSettings();

    await userEvent.click(screen.getByRole("button", { name: "Back" }));

    expect(screen.getByRole("button", { name: "Settings" })).toHaveFocus();
  });
});
