import { describe, expect, it } from "vitest";
import { isArrayBuffer, isRegionCompletion, isRegionContext } from "./guards";

const OUTLINE = { x: 1, y: 2, width: 3, height: 4 };

describe("region selector messages are checked before the overlay trusts them", () => {
  it("accepts a frame description with its window outlines", () => {
    expect(isRegionContext({ frame_width: 10, frame_height: 5, outlines: [OUTLINE] })).toBe(true);
  });

  it("rejects an outline that lacks a corner or a size", () => {
    expect(isRegionContext({ frame_width: 10, frame_height: 5, outlines: [{ x: 1, y: 2 }] })).toBe(false);
  });

  it("rejects a frame description without a size", () => {
    expect(isRegionContext({ outlines: [] })).toBe(false);
  });

  it("accepts a captured or too-small completion and rejects any other outcome", () => {
    expect(isRegionCompletion({ outcome: "captured" })).toBe(true);
    expect(isRegionCompletion({ outcome: "too_small" })).toBe(true);
    expect(isRegionCompletion({ outcome: "exploded" })).toBe(false);
    expect(isRegionCompletion({})).toBe(false);
  });

  it("accepts raw pixel bytes and rejects anything else", () => {
    expect(isArrayBuffer(new ArrayBuffer(4))).toBe(true);
    expect(isArrayBuffer([1, 2])).toBe(false);
  });
});
