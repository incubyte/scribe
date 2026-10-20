import { hasFields, isListOf, isNumber, isOneOf } from "../popover/guards";
import type { PixelRect, RegionCompletion, RegionContext } from "./types";

const COMPLETION_OUTCOMES: readonly string[] = ["captured", "too_small"];

const isCompletionOutcome = isOneOf<RegionCompletion["outcome"]>(COMPLETION_OUTCOMES);

function isPixelRect(value: unknown): value is PixelRect {
  return hasFields(value, { x: isNumber, y: isNumber, width: isNumber, height: isNumber });
}

export function isRegionContext(value: unknown): value is RegionContext {
  return hasFields(value, {
    frame_width: isNumber,
    frame_height: isNumber,
    outlines: isListOf(isPixelRect),
  });
}

export function isRegionCompletion(value: unknown): value is RegionCompletion {
  return hasFields(value, { outcome: isCompletionOutcome });
}

export function isArrayBuffer(value: unknown): value is ArrayBuffer {
  return value instanceof ArrayBuffer;
}
