import type { ClientPoint, FramePoint, PixelRect, Size } from "./types";

export const MIN_SELECTION_EDGE_PX = 4;
export const DRAG_THRESHOLD_PX = 4;

const clamp = (value: number, lowest: number, highest: number) =>
  Math.min(Math.max(value, lowest), highest);

const scaleAlong = (client: number, viewportLength: number, frameLength: number) =>
  clamp(Math.round((client * frameLength) / viewportLength), 0, frameLength);

export function toFramePoint(pointer: ClientPoint, viewport: Size, frame: Size): FramePoint {
  return {
    x: scaleAlong(pointer.clientX, viewport.width, frame.width),
    y: scaleAlong(pointer.clientY, viewport.height, frame.height),
  };
}

export function rectBetween(a: FramePoint, b: FramePoint): PixelRect {
  return {
    x: Math.min(a.x, b.x),
    y: Math.min(a.y, b.y),
    width: Math.abs(a.x - b.x),
    height: Math.abs(a.y - b.y),
  };
}

export const isLargeEnough = (rect: PixelRect) =>
  rect.width >= MIN_SELECTION_EDGE_PX && rect.height >= MIN_SELECTION_EDGE_PX;

export const sizeLabel = (rect: PixelRect) => `${rect.width} × ${rect.height}`;

export const hasDraggedPastThreshold = (origin: ClientPoint, current: ClientPoint) =>
  Math.hypot(current.clientX - origin.clientX, current.clientY - origin.clientY) >
  DRAG_THRESHOLD_PX;
