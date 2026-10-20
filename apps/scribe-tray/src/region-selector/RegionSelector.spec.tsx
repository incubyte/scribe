import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "vitest-axe";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RegionSelector } from "./RegionSelector";
import type { PixelRect, RegionCompletion } from "./types";

const INSTRUCTION = "Region capture. Drag to select, Escape to cancel.";
const TWICE_THE_VIEWPORT = { frame_width: 2048, frame_height: 1536, outlines: [] };
const CAPTURED: RegionCompletion = { outcome: "captured" };

const BROWSER_WINDOW: PixelRect = { x: 200, y: 100, width: 600, height: 400 };

function backend(
  completion: RegionCompletion = CAPTURED,
  outlines: PixelRect[] = [],
  failures: Readonly<Record<string, () => never>> = {},
) {
  const responses: Readonly<Record<string, () => unknown>> = {
    region_context: () => ({ ...TWICE_THE_VIEWPORT, outlines }),
    region_frame: () => new ArrayBuffer(16),
    complete_region: () => completion,
    cancel_region: () => null,
    ...failures,
  };
  const handler = vi.fn((command: string) => responses[command]?.());
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

  it("outlines the window under the pointer with its size in output pixels", async () => {
    backend(CAPTURED, [BROWSER_WINDOW]);
    const selector = await renderWithFrame();

    fireEvent.pointerMove(selector, { clientX: 200, clientY: 100 });

    expect(screen.getByText("600 × 400")).toBeInTheDocument();
  });

  it("captures the highlighted window's rectangle when clicked without dragging", async () => {
    const handler = backend(CAPTURED, [BROWSER_WINDOW]);
    const selector = await renderWithFrame();

    fireEvent.pointerMove(selector, { clientX: 200, clientY: 100 });
    drag(selector, [200, 100], [200, 100]);

    await waitFor(() => {
      expect(handler).toHaveBeenCalledWith("complete_region", { area: BROWSER_WINDOW });
    });
  });

  it("clears the highlight and selects a free region once a drag starts", async () => {
    const handler = backend(CAPTURED, [BROWSER_WINDOW]);
    const selector = await renderWithFrame();

    fireEvent.pointerMove(selector, { clientX: 200, clientY: 100 });
    drag(selector, [200, 100], [250, 150]);

    expect(screen.queryByText("600 × 400")).not.toBeInTheDocument();
    await waitFor(() => {
      expect(handler).toHaveBeenCalledWith("complete_region", {
        area: { x: 400, y: 200, width: 100, height: 100 },
      });
    });
  });

  it("captures nothing and stays open when clicking where no window is highlighted", async () => {
    const handler = backend(CAPTURED, [BROWSER_WINDOW]);
    const selector = await renderWithFrame();

    drag(selector, [10, 20], [10, 20]);

    expect(handler).not.toHaveBeenCalledWith("complete_region", expect.anything());
    expect(handler).not.toHaveBeenCalledWith("cancel_region", expect.anything());
  });

  it("keeps outlining the same window as the pointer moves within it", async () => {
    const handler = backend(CAPTURED, [BROWSER_WINDOW]);
    const selector = await renderWithFrame();

    fireEvent.pointerMove(selector, { clientX: 200, clientY: 100 });
    fireEvent.pointerMove(selector, { clientX: 300, clientY: 150 });

    const contextFetches = handler.mock.calls.filter(([command]) => command === "region_context");
    expect(contextFetches).toHaveLength(1);
    expect(screen.getByText("600 × 400")).toBeInTheDocument();
  });

  it("keeps the size readout inside the screen when the selection reaches the bottom edge", async () => {
    backend();
    const selector = await renderWithFrame();

    drag(selector, [10, 20], [110, 760]);

    expect(screen.getByText("200 × 1480")).toHaveClass("region-selector__size--upward");
  });

  it("puts the size readout below a selection with room beneath it", async () => {
    backend();
    const selector = await renderWithFrame();

    fireEvent.pointerDown(selector, { clientX: 10, clientY: 20, button: 0 });
    fireEvent.pointerMove(selector, { clientX: 110, clientY: 70 });

    expect(screen.getByText("200 × 100")).not.toHaveClass("region-selector__size--upward");
  });

  it("cancels the capture when the frozen screen cannot be loaded", async () => {
    const unavailable = () => {
      throw new Error("nothing pending");
    };
    const handler = backend(CAPTURED, [], { region_frame: unavailable });
    render(<RegionSelector />);

    await waitFor(() => expect(handler).toHaveBeenCalledWith("cancel_region", {}));
  });

  it("cancels the capture when the window outlines cannot be loaded", async () => {
    const unavailable = () => {
      throw new Error("nothing pending");
    };
    const handler = backend(CAPTURED, [], { region_context: unavailable });
    render(<RegionSelector />);

    await waitFor(() => expect(handler).toHaveBeenCalledWith("cancel_region", {}));
  });

  it("cancels the capture when saving the selected area fails", async () => {
    const saveFails = () => {
      throw new Error("could not save");
    };
    const handler = backend(CAPTURED, [], { complete_region: saveFails });
    const selector = await renderWithFrame();

    drag(selector, [10, 20], [110, 70]);

    await waitFor(() => expect(handler).toHaveBeenCalledWith("cancel_region", {}));
  });

  it("has no accessibility violations", async () => {
    backend();
    const { container } = render(<RegionSelector />);
    await screen.findByText(INSTRUCTION);

    const { violations } = await axe(container);

    expect(violations).toEqual([]);
  });
});
