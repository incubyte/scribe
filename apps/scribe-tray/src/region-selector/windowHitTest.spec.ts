import { describe, expect, it } from "vitest";
import { topmostOutlineAt } from "./windowHitTest";

const front = { x: 100, y: 100, width: 200, height: 100 };
const back = { x: 0, y: 0, width: 500, height: 300 };

describe("finding the window under the pointer", () => {
  it("picks the frontmost of two overlapping windows", () => {
    expect(topmostOutlineAt({ x: 150, y: 150 }, [front, back])).toBe(front);
  });

  it("picks the window behind where the front window does not reach", () => {
    expect(topmostOutlineAt({ x: 400, y: 250 }, [front, back])).toBe(back);
  });

  it("finds nothing on a point outside every window", () => {
    expect(topmostOutlineAt({ x: 600, y: 400 }, [front, back])).toBeUndefined();
  });

  it("treats the right and bottom edges as outside the window", () => {
    expect(topmostOutlineAt({ x: 300, y: 150 }, [front])).toBeUndefined();
    expect(topmostOutlineAt({ x: 150, y: 200 }, [front])).toBeUndefined();
  });
});
