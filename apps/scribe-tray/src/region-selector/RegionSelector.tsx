import { useEffect, useRef, useState } from "react";
import { cancelRegion } from "./commands";
import { sizeLabel } from "./selection";
import type { PixelRect, Size } from "./types";
import { useFrozenFrame } from "./useFrozenFrame";
import { useRegionDrag } from "./useRegionDrag";
import "./regionSelector.css";

const INSTRUCTION = "Region capture. Drag to select, Escape to cancel.";
const HINT = "Drag to select · Esc to cancel";

const percentOf = (length: number, total: number) => `${(length / total) * 100}%`;

function useAnnouncementOnMount(message: string): string {
  const [announcement, setAnnouncement] = useState("");
  useEffect(() => setAnnouncement(message), [message]);
  return announcement;
}

function useEscapeToCancel() {
  useEffect(() => {
    const cancelOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void cancelRegion();
      }
    };
    window.addEventListener("keydown", cancelOnEscape);
    return () => window.removeEventListener("keydown", cancelOnEscape);
  }, []);
}

function SelectedArea({ area, frame }: Readonly<{ area: PixelRect; frame: Size }>) {
  const left = percentOf(area.x, frame.width);
  return (
    <>
      <div
        className="region-selector__selection"
        style={{
          left,
          top: percentOf(area.y, frame.height),
          width: percentOf(area.width, frame.width),
          height: percentOf(area.height, frame.height),
        }}
      />
      <span
        className="region-selector__size"
        style={{ left, top: percentOf(area.y + area.height, frame.height) }}
      >
        {sizeLabel(area)}
      </span>
    </>
  );
}

export function RegionSelector() {
  const canvas = useRef<HTMLCanvasElement>(null);
  const frame = useFrozenFrame(canvas);
  const { area, start, extend, release } = useRegionDrag(frame);
  const announcement = useAnnouncementOnMount(INSTRUCTION);
  useEscapeToCancel();

  const cancelOnRightClick = (event: React.MouseEvent) => {
    event.preventDefault();
    void cancelRegion();
  };

  return (
    <main
      className="region-selector"
      aria-busy={frame === undefined}
      onPointerDown={start}
      onPointerMove={extend}
      onPointerUp={release}
      onContextMenu={cancelOnRightClick}
    >
      <canvas ref={canvas} className="region-selector__frame" aria-hidden="true" />
      {area && frame ? (
        <SelectedArea area={area} frame={frame} />
      ) : (
        <div className="region-selector__dim" />
      )}
      <p className="region-selector__hint" aria-hidden="true">
        {HINT}
      </p>
      <p className="visually-hidden" aria-live="assertive">
        {announcement}
      </p>
    </main>
  );
}
