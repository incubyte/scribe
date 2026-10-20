import { useRef, useState } from "react";
import { cancelRegion, completeRegion } from "./commands";
import { hasDraggedPastThreshold, isLargeEnough, rectBetween, toFramePoint } from "./selection";
import type { ClientPoint, FramePoint, FrozenFrame, PixelRect, Size } from "./types";
import { topmostOutlineAt } from "./windowHitTest";

const PRIMARY_BUTTON = 0;

const viewport = (): Size => ({ width: window.innerWidth, height: window.innerHeight });

const pointIn = (frame: Size, pointer: ClientPoint) => toFramePoint(pointer, viewport(), frame);

export function useRegionDrag(frame: FrozenFrame | undefined) {
  const [anchor, setAnchor] = useState<FramePoint>();
  const [selection, setSelection] = useState<PixelRect>();
  const [highlight, setHighlight] = useState<PixelRect>();
  const pressedAt = useRef<ClientPoint>({ clientX: 0, clientY: 0 });

  const submit = (area: PixelRect) => {
    if (!isLargeEnough(area)) {
      setSelection(undefined);
      return;
    }
    setSelection(area);
    completeRegion(area)
      .then((completion) => {
        if (completion.outcome === "too_small") {
          setSelection(undefined);
        }
      })
      .catch(() => void cancelRegion());
  };

  const start = (event: React.PointerEvent) => {
    if (!frame || event.button !== PRIMARY_BUTTON) {
      return;
    }
    pressedAt.current = { clientX: event.clientX, clientY: event.clientY };
    setAnchor(pointIn(frame, event));
  };

  const extend = (event: React.PointerEvent) => {
    if (!frame) {
      return;
    }
    const point = pointIn(frame, event);
    if (!anchor) {
      setHighlight(topmostOutlineAt(point, frame.outlines));
    } else if (selection || hasDraggedPastThreshold(pressedAt.current, event)) {
      setHighlight(undefined);
      setSelection(rectBetween(anchor, point));
    }
  };

  const release = (event: React.PointerEvent) => {
    if (!frame || !anchor) {
      return;
    }
    setAnchor(undefined);
    if (selection) {
      submit(rectBetween(anchor, pointIn(frame, event)));
      return;
    }
    const clicked = topmostOutlineAt(anchor, frame.outlines);
    if (clicked) {
      submit(clicked);
    }
  };

  return { area: selection ?? highlight, start, extend, release };
}
