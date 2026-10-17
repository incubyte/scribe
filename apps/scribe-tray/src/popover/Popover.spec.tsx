import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { axe } from "vitest-axe";
import { Popover } from "./Popover";
import type { RecorderSnapshot, SessionSummary } from "./types";

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

const idle: RecorderSnapshot = { state: "idle", started_at: null, lines: [] };
const locked: RecorderSnapshot = { state: "locked", started_at: null, lines: [] };
const recordingForSixtyFiveSeconds: RecorderSnapshot = {
  state: "recording",
  started_at: new Date(NOW - 65_000).toISOString(),
  lines: [{ speaker: "Them", start_ms: 0, text: "Good morning" }],
};

function backend(snapshot: RecorderSnapshot, recent: readonly SessionSummary[] = FIVE_SESSIONS) {
  const handler = vi.fn((command: string) => {
    if (command === "recorder_snapshot") {
      return snapshot;
    }
    return command === "recent_sessions" ? recent : undefined;
  });
  mockIPC(handler, { shouldMockEvents: true });
  return handler;
}

beforeEach(() => {
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

    const sessions = await screen.findByRole("list", { name: "Recent sessions" });

    expect(screen.getByRole("button", { name: "Start recording" })).toBeEnabled();
    expect(within(sessions).getAllByRole("button")).toHaveLength(5);
    expect(within(sessions).getByText("Call 2026-10-17 12:00")).toBeInTheDocument();
    expect(within(sessions).getByText(/Oct 17/)).toBeInTheDocument();
    expect(within(sessions).getByText("32 min")).toBeInTheDocument();
  });

  it("opens a session's file when it is chosen", async () => {
    const handler = backend(idle);
    render(<Popover />);

    await userEvent.click(await screen.findByRole("button", { name: /Call 2026-10-16 12:00/ }));

    expect(handler).toHaveBeenCalledWith("open_session", { id: "2026-10-16-120000" });
  });

  it("starts a recording when Start is pressed", async () => {
    const handler = backend(idle);
    render(<Popover />);

    await userEvent.click(await screen.findByRole("button", { name: "Start recording" }));

    expect(handler).toHaveBeenCalledWith("start_recording", {});
  });

  it("disables Start while the app is not ready to record", async () => {
    backend(locked);
    render(<Popover />);

    await screen.findByRole("list", { name: "Recent sessions" });

    expect(screen.getByRole("button", { name: "Start recording" })).toBeDisabled();
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
    expect(screen.getByRole("button", { name: "Start recording" })).toHaveFocus();

    await userEvent.tab();
    expect(screen.getByRole("button", { name: /Call 2026-10-17 12:00/ })).toHaveFocus();
  });
});

describe("the popover while recording", () => {
  it("shows the elapsed time and a Stop button", async () => {
    backend(recordingForSixtyFiveSeconds);
    render(<Popover />);

    expect(await screen.findByText("01:05")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Stop recording" })).toBeEnabled();
  });

  it("stops the recording when Stop is pressed", async () => {
    const handler = backend(recordingForSixtyFiveSeconds);
    render(<Popover />);

    await userEvent.click(await screen.findByRole("button", { name: "Stop recording" }));

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

    await act(() => emit("recording://partial", { speaker: "Me", text: "hello wor" }));
    expect(await screen.findByText("hello wor")).toBeInTheDocument();

    await act(() =>
      emit("recording://segment", { speaker: "Me", start_ms: 900, text: "hello world" }),
    );

    await waitFor(() => expect(screen.queryByText("hello wor")).not.toBeInTheDocument());
    expect(screen.getAllByText("hello world").length).toBeGreaterThan(0);
  });

  it("announces only finished lines to screen readers", async () => {
    backend(recordingForSixtyFiveSeconds);
    const { container } = render(<Popover />);
    await screen.findByText("Good morning");

    await act(() => emit("recording://partial", { speaker: "Me", text: "still talk" }));
    const partialLine = await screen.findByText("still talk");
    await act(() =>
      emit("recording://segment", { speaker: "Them", start_ms: 900, text: "All done" }),
    );

    const announcements = container.querySelector('[aria-live="polite"]');
    expect(announcements).toHaveTextContent("Them: All done");
    expect(announcements).not.toHaveTextContent("still talk");
    expect(partialLine.closest("li")).toHaveAttribute("aria-hidden", "true");
  });

  it("returns to Start and refreshes recent sessions when the recording stops", async () => {
    const handler = backend(recordingForSixtyFiveSeconds);
    render(<Popover />);
    await screen.findByRole("button", { name: "Stop recording" });
    const callsBefore = handler.mock.calls.filter(([c]) => c === "recent_sessions").length;

    await act(() => emit("recording://stopped", { id: "2026-10-17-091000" }));

    expect(await screen.findByRole("button", { name: "Start recording" })).toBeInTheDocument();
    const callsAfter = handler.mock.calls.filter(([c]) => c === "recent_sessions").length;
    expect(callsAfter).toBeGreaterThan(callsBefore);
  });

  it("moves focus from Stop to the transcript", async () => {
    backend(recordingForSixtyFiveSeconds);
    render(<Popover />);
    await screen.findByText("Good morning");

    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Stop recording" })).toHaveFocus();

    await userEvent.tab();
    expect(screen.getByRole("region", { name: "Live transcript" })).toHaveFocus();
  });

  it("has no accessibility violations", async () => {
    backend(recordingForSixtyFiveSeconds);
    const { container } = render(<Popover />);
    await screen.findByText("Good morning");

    const { violations } = await axe(container);

    expect(violations).toEqual([]);
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

describe("the idle popover accessibility", () => {
  it("has no accessibility violations", async () => {
    backend(idle);
    const { container } = render(<Popover />);
    await screen.findByRole("list", { name: "Recent sessions" });

    const { violations } = await axe(container);

    expect(violations).toEqual([]);
  });
});
