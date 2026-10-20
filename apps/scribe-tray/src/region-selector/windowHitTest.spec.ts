import { describe, expect, it } from "vitest";
import { topmostOutlineAt } from "./windowHitTest";

const FRONT_WINDOW = { x: 100, y: 100, width: 200, height: 100 };
const BACK_WINDOW = { x: 0, y: 0, width: 500, height: 300 };

describe("finding the window under the pointer", () => {
  it("picks the frontmost of two overlapping windows", () => {
    expect(topmostOutlineAt({ x: 150, y: 150 }, [FRONT_WINDOW, BACK_WINDOW])).toBe(FRONT_WINDOW);
  });

  it("picks the window behind where the front window does not reach", () => {
    expect(topmostOutlineAt({ x: 400, y: 250 }, [FRONT_WINDOW, BACK_WINDOW])).toBe(BACK_WINDOW);
  });

  it("finds nothing on a point outside every window", () => {
    expect(topmostOutlineAt({ x: 600, y: 400 }, [FRONT_WINDOW, BACK_WINDOW])).toBeUndefined();
  });

  it("treats the right and bottom edges as outside the window", () => {
    expect(topmostOutlineAt({ x: 300, y: 150 }, [FRONT_WINDOW])).toBeUndefined();
    expect(topmostOutlineAt({ x: 150, y: 200 }, [FRONT_WINDOW])).toBeUndefined();
  });

  it("treats the left and top edges as inside the window", () => {
    expect(topmostOutlineAt({ x: 100, y: 150 }, [FRONT_WINDOW])).toBe(FRONT_WINDOW);
    expect(topmostOutlineAt({ x: 150, y: 100 }, [FRONT_WINDOW])).toBe(FRONT_WINDOW);
  });

  it("finds nothing to the left of or above a window", () => {
    expect(topmostOutlineAt({ x: 50, y: 150 }, [FRONT_WINDOW])).toBeUndefined();
    expect(topmostOutlineAt({ x: 150, y: 50 }, [FRONT_WINDOW])).toBeUndefined();
  });
});
