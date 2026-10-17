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
  render(<StatusBar phase="recording" startedAt={Date.now()} onStart={vi.fn()} onStop={vi.fn()} />);
  return screen.getByTestId("recording-dot");
}

afterEach(() => {
  vi.unstubAllGlobals();
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
