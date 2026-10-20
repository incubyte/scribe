import { describe, expect, it } from "vitest";
import { isLargeEnough, rectBetween, sizeLabel, toFramePoint } from "./selection";

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
    expect(isLargeEnough({ x: 0, y: 0, width: 4, height: 4 })).toBe(true);
  });

  it("reads out its size as width by height in output pixels", () => {
    expect(sizeLabel({ x: 10, y: 20, width: 1240, height: 680 })).toBe("1240 × 680");
  });
});
