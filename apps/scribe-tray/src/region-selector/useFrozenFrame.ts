import { type RefObject, useEffect, useState } from "react";
import { cancelRegion, fetchRegionContext, fetchRegionFrame } from "./commands";
import type { FrozenFrame, Size } from "./types";

function paintFrame(canvas: HTMLCanvasElement | null, rgba: ArrayBuffer, frame: Size) {
  const context = canvas?.getContext("2d");
  if (!canvas || !context) {
    return;
  }
  canvas.width = frame.width;
  canvas.height = frame.height;
  const pixels = new ImageData(new Uint8ClampedArray(rgba), frame.width, frame.height);
  context.putImageData(pixels, 0, 0);
}

export function useFrozenFrame(canvas: RefObject<HTMLCanvasElement | null>): FrozenFrame | undefined {
  const [frame, setFrame] = useState<FrozenFrame>();

  useEffect(() => {
    let mounted = true;
    Promise.all([fetchRegionContext(), fetchRegionFrame()])
      .then(([context, rgba]) => {
        const size = { width: context.frame_width, height: context.frame_height };
        if (mounted) {
          paintFrame(canvas.current, rgba, size);
          setFrame({ ...size, outlines: context.outlines });
        }
      })
      .catch(() => void cancelRegion());
    return () => {
      mounted = false;
    };
  }, [canvas]);

  return frame;
}
