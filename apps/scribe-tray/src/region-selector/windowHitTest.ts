import type { FramePoint, PixelRect } from "./types";

const contains = (rect: PixelRect, point: FramePoint) =>
  point.x >= rect.x &&
  point.x < rect.x + rect.width &&
  point.y >= rect.y &&
  point.y < rect.y + rect.height;

export const topmostOutlineAt = (
  point: FramePoint,
  outlines: readonly PixelRect[],
): PixelRect | undefined => outlines.find((outline) => contains(outline, point));
