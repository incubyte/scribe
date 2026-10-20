import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "vitest-axe";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RegionSelector } from "./RegionSelector";
import type { RegionCompletion } from "./types";

const INSTRUCTION = "Region capture. Drag to select, Escape to cancel.";
const TWICE_THE_VIEWPORT = { frame_width: 2048, frame_height: 1536, outlines: [] };
const CAPTURED: RegionCompletion = { outcome: "captured" };

function backend(completion: RegionCompletion = CAPTURED) {
  const responses: Readonly<Record<string, unknown>> = {
    region_context: TWICE_THE_VIEWPORT,
    region_frame: new ArrayBuffer(16),
    complete_region: completion,
    cancel_region: null,
  };
  const handler = vi.fn((command: string) => responses[command]);
  mockIPC(handler);
  return handler;
}

async function renderWithFrame() {
  render(<RegionSelector />);
  const selector = screen.getByRole("main");
  await waitFor(() => expect(selector).toHaveAttribute("aria-busy", "false"));
  return selector;
}

function drag(selector: HTMLElement, from: [number, number], to: [number, number]) {
  fireEvent.pointerDown(selector, { clientX: from[0], clientY: from[1], button: 0 });
  fireEvent.pointerMove(selector, { clientX: to[0], clientY: to[1] });
  fireEvent.pointerUp(selector, { clientX: to[0], clientY: to[1] });
}

beforeEach(() => {
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(() => null);
});

afterEach(() => {
  vi.restoreAllMocks();
  clearMocks();
});

describe("the region selector", () => {
  it("announces how to use it to screen readers when it opens", async () => {
    backend();
    render(<RegionSelector />);

    const announcement = await screen.findByText(INSTRUCTION);

    expect(announcement).toHaveAttribute("aria-live", "assertive");
  });

  it("shows the selection size in output pixels while dragging", async () => {
    backend();
    const selector = await renderWithFrame();

    fireEvent.pointerDown(selector, { clientX: 10, clientY: 20, button: 0 });
    fireEvent.pointerMove(selector, { clientX: 110, clientY: 70 });

    expect(screen.getByText("200 × 100")).toBeInTheDocument();
  });

  it("captures the dragged rectangle in frame pixels on release", async () => {
    const handler = backend();
    const selector = await renderWithFrame();

    drag(selector, [10, 20], [110, 70]);

    await waitFor(() => {
      expect(handler).toHaveBeenCalledWith("complete_region", {
        area: { x: 20, y: 40, width: 200, height: 100 },
      });
    });
  });

  it("cancels on Escape without capturing", async () => {
    const handler = backend();
    await renderWithFrame();

    await userEvent.keyboard("{Escape}");

    await waitFor(() => expect(handler).toHaveBeenCalledWith("cancel_region", {}));
    expect(handler).not.toHaveBeenCalledWith("complete_region", expect.anything());
  });

  it("cancels on a right-click without capturing", async () => {
    const handler = backend();
    const selector = await renderWithFrame();

    fireEvent.pointerDown(selector, { clientX: 10, clientY: 20, button: 2 });
    fireEvent.contextMenu(selector, { clientX: 10, clientY: 20 });

    await waitFor(() => expect(handler).toHaveBeenCalledWith("cancel_region", {}));
    expect(handler).not.toHaveBeenCalledWith("complete_region", expect.anything());
  });

  it("captures nothing and stays open after a drag smaller than 4 by 4 pixels", async () => {
    const handler = backend();
    const selector = await renderWithFrame();

    drag(selector, [10, 20], [11, 21]);

    expect(handler).not.toHaveBeenCalledWith("complete_region", expect.anything());
    expect(handler).not.toHaveBeenCalledWith("cancel_region", expect.anything());
    expect(screen.queryByText("2 × 2")).not.toBeInTheDocument();
  });

  it("captures nothing after a click", async () => {
    const handler = backend();
    const selector = await renderWithFrame();

    drag(selector, [10, 20], [10, 20]);

    expect(handler).not.toHaveBeenCalledWith("complete_region", expect.anything());
  });

  it("stays open with the selection cleared when Scribe refuses the area", async () => {
    const handler = backend({ outcome: "too_small" });
    const selector = await renderWithFrame();

    drag(selector, [10, 20], [110, 70]);

    await waitFor(() => expect(screen.queryByText("200 × 100")).not.toBeInTheDocument());
    expect(handler).not.toHaveBeenCalledWith("cancel_region", expect.anything());
  });

  it("has no accessibility violations", async () => {
    backend();
    const { container } = render(<RegionSelector />);
    await screen.findByText(INSTRUCTION);

    const { violations } = await axe(container);

    expect(violations).toEqual([]);
  });
});
