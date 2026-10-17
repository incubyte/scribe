import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { LiveTranscript } from "./LiveTranscript";
import type { LiveLine } from "./feedReducer";

const FIRST: LiveLine = { speaker: "Me", startMs: 0, text: "first" };
const SECOND: LiveLine = { speaker: "Them", startMs: 1000, text: "second" };

function layOutScroller(region: HTMLElement, scrolledTo: number) {
  let top = scrolledTo;
  Object.defineProperty(region, "scrollHeight", { configurable: true, value: 500 });
  Object.defineProperty(region, "clientHeight", { configurable: true, value: 100 });
  Object.defineProperty(region, "scrollTop", {
    configurable: true,
    get: () => top,
    set: (value: number) => {
      top = value;
    },
  });
  return () => top;
}

describe("the live transcript scrolling", () => {
  it("follows the newest line while the reader is at the bottom", () => {
    const { rerender } = render(<LiveTranscript lines={[FIRST]} partials={{}} />);
    const region = screen.getByRole("region", { name: "Live transcript" });
    const scrollTop = layOutScroller(region, 400);

    rerender(<LiveTranscript lines={[FIRST, SECOND]} partials={{}} />);

    expect(scrollTop()).toBe(500);
  });

  it("stays put once the reader has scrolled up", () => {
    const { rerender } = render(<LiveTranscript lines={[FIRST]} partials={{}} />);
    const region = screen.getByRole("region", { name: "Live transcript" });
    const scrollTop = layOutScroller(region, 0);
    fireEvent.scroll(region);

    rerender(<LiveTranscript lines={[FIRST, SECOND]} partials={{}} />);

    expect(scrollTop()).toBe(0);
  });
});
