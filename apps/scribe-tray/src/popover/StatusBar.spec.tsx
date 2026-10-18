import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { StatusBar } from "./StatusBar";

function prefersReducedMotion(reduced: boolean) {
  vi.stubGlobal(
    "matchMedia",
    vi.fn(() => ({
      matches: reduced,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })),
  );
}

function renderRecording() {
  render(
    <StatusBar
      phase="recording"
      startedAt={Date.now()}
      startFailure={null}
      onStart={vi.fn()}
      onStop={vi.fn()}
    />,
  );
  return screen.getByTestId("recording-dot");
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("the recording button", () => {
  it("stays the same control when the recording starts, so focus is not lost", () => {
    const { rerender } = render(
      <StatusBar phase="idle" startedAt={null} startFailure={null} onStart={vi.fn()} onStop={vi.fn()} />,
    );
    const button = screen.getByRole("button", { name: "Start recording" });

    rerender(
      <StatusBar
        phase="recording"
        startedAt={Date.now()}
        startFailure={null}
        onStart={vi.fn()}
        onStop={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: "Stop recording" })).toBe(button);
  });

  it("holds the dot still while nothing is recording", () => {
    render(<StatusBar phase="idle" startedAt={null} startFailure={null} onStart={vi.fn()} onStop={vi.fn()} />);

    expect(screen.getByTestId("recording-dot")).not.toHaveClass("status-bar__dot--live");
  });
});

describe("the recording indicator", () => {
  it("pulses while recording", () => {
    prefersReducedMotion(false);

    expect(renderRecording()).toHaveClass("status-bar__dot--pulsing");
  });

  it("holds still when the system asks to reduce motion", () => {
    prefersReducedMotion(true);

    expect(renderRecording()).not.toHaveClass("status-bar__dot--pulsing");
  });
});
