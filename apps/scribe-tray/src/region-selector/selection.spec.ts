import { describe, expect, it } from "vitest";
import {
  hasDraggedPastThreshold,
  isLargeEnough,
  rectBetween,
  sizeLabel,
  sizeLabelAnchor,
  toFramePoint,
} from "./selection";

const RETINA_VIEWPORT = { width: 1440, height: 900 };
const RETINA_FRAME = { width: 2880, height: 1800 };
const EXTERNAL_MONITOR = { width: 1920, height: 1080 };

describe("pointer positions become frame pixels on the monitor they are on", () => {
  it("doubles a pointer position on a 2x display", () => {
    const point = toFramePoint({ clientX: 100, clientY: 50 }, RETINA_VIEWPORT, RETINA_FRAME);

    expect(point).toEqual({ x: 200, y: 100 });
  });

  it("keeps a pointer position one to one on a 1x display", () => {
    const point = toFramePoint({ clientX: 100, clientY: 50 }, EXTERNAL_MONITOR, EXTERNAL_MONITOR);

    expect(point).toEqual({ x: 100, y: 50 });
  });

  it("stops at the edges of the monitor when the pointer goes past them", () => {
    const point = toFramePoint({ clientX: 1500, clientY: -20 }, RETINA_VIEWPORT, RETINA_FRAME);

    expect(point).toEqual({ x: 2880, y: 0 });
  });
});

describe("a dragged selection", () => {
  it("is the same rectangle whichever way the drag goes", () => {
    const backwards = rectBetween({ x: 300, y: 200 }, { x: 100, y: 50 });

    expect(backwards).toEqual({ x: 100, y: 50, width: 200, height: 150 });
  });

  it("is too small to capture below 4 by 4 pixels", () => {
    expect(isLargeEnough({ x: 0, y: 0, width: 3, height: 3 })).toBe(false);
    expect(isLargeEnough({ x: 0, y: 0, width: 4, height: 3 })).toBe(false);
    expect(isLargeEnough({ x: 0, y: 0, width: 3, height: 4 })).toBe(false);
    expect(isLargeEnough({ x: 0, y: 0, width: 4, height: 4 })).toBe(true);
  });

  it("reads out its size as width by height in output pixels", () => {
    expect(sizeLabel({ x: 10, y: 20, width: 1240, height: 680 })).toBe("1240 × 680");
  });
});

describe("telling a click from a drag", () => {
  const pressed = { clientX: 10, clientY: 10 };

  it("is still a click while the pointer stays within 4 pixels", () => {
    expect(hasDraggedPastThreshold(pressed, { clientX: 14, clientY: 10 })).toBe(false);
  });

  it("is a drag once the pointer moves further than 4 pixels", () => {
    expect(hasDraggedPastThreshold(pressed, { clientX: 15, clientY: 10 })).toBe(true);
    expect(hasDraggedPastThreshold(pressed, { clientX: 10, clientY: 15 })).toBe(true);
  });
});

describe("where the size readout sits next to a selection", () => {
  const frame = { width: 2000, height: 1000 };
  const viewport = frame;

  it("opens below the selection when there is room", () => {
    const area = { x: 100, y: 100, width: 200, height: 300 };

    expect(sizeLabelAnchor(area, frame, viewport)).toEqual({ x: 100, y: 400, opensUpward: false });
  });

  it("opens upward from the top edge when the selection reaches the bottom of the screen", () => {
    const area = { x: 100, y: 300, width: 200, height: 700 };

    expect(sizeLabelAnchor(area, frame, viewport)).toEqual({ x: 100, y: 300, opensUpward: true });
  });

  it("opens upward from inside the bottom edge when the selection fills the screen height", () => {
    const area = { x: 0, y: 0, width: 2000, height: 1000 };

    expect(sizeLabelAnchor(area, frame, viewport)).toEqual({ x: 0, y: 1000, opensUpward: true });
  });

  it("keeps the same on-screen clearance on a high density display", () => {
    const retinaFrame = { width: 4000, height: 2000 };
    const area = { x: 100, y: 200, width: 200, height: 1700 };

    expect(sizeLabelAnchor(area, retinaFrame, viewport)).toEqual({
      x: 100,
      y: 200,
      opensUpward: true,
    });
  });

  it("shifts left so the readout stays inside the right edge", () => {
    const area = { x: 1950, y: 100, width: 50, height: 100 };

    const anchor = sizeLabelAnchor(area, frame, viewport);

    expect(anchor.x).toBeLessThan(1950);
    expect(anchor.x).toBeGreaterThan(1700);
  });

  it("opens below when the readout fits exactly above the bottom edge", () => {
    const area = { x: 0, y: 0, width: 100, height: 936 };

    expect(sizeLabelAnchor(area, frame, viewport)).toEqual({ x: 0, y: 936, opensUpward: false });
  });

  it("opens upward from the top edge when the selection starts exactly one clearance below the top", () => {
    const area = { x: 0, y: 64, width: 100, height: 936 };

    expect(sizeLabelAnchor(area, frame, viewport)).toEqual({ x: 0, y: 64, opensUpward: true });
  });
});
