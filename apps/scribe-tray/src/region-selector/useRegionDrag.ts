import { useState } from "react";
import { cancelRegion, completeRegion } from "./commands";
import { isLargeEnough, rectBetween, toFramePoint } from "./selection";
import type { ClientPoint, FramePoint, PixelRect, Size } from "./types";

const PRIMARY_BUTTON = 0;

const viewport = (): Size => ({ width: window.innerWidth, height: window.innerHeight });

const pointIn = (frame: Size, pointer: ClientPoint) => toFramePoint(pointer, viewport(), frame);

export function useRegionDrag(frame: Size | undefined) {
  const [anchor, setAnchor] = useState<FramePoint>();
  const [selection, setSelection] = useState<PixelRect>();

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
    const point = pointIn(frame, event);
    setAnchor(point);
    setSelection(rectBetween(point, point));
  };

  const extend = (event: React.PointerEvent) => {
    if (frame && anchor) {
      setSelection(rectBetween(anchor, pointIn(frame, event)));
    }
  };

  const release = (event: React.PointerEvent) => {
    if (!frame || !anchor) {
      return;
    }
    setAnchor(undefined);
    submit(rectBetween(anchor, pointIn(frame, event)));
  };

  return { selection, start, extend, release };
}
