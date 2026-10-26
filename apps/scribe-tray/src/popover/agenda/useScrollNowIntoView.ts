import { useCallback, useEffect, useRef } from "react";

const TOP_THIRD = 3;

export function useScrollNowIntoView(minutesNow: number) {
  const scroller = useRef<HTMLDivElement>(null);
  const latestMinutes = useRef(minutesNow);
  useEffect(() => {
    latestMinutes.current = minutesNow;
  }, [minutesNow]);

  const scrollToNow = useCallback(() => {
    if (scroller.current) {
      scroller.current.scrollTop = Math.max(
        0,
        latestMinutes.current - scroller.current.clientHeight / TOP_THIRD,
      );
    }
  }, []);

  useEffect(() => {
    scrollToNow();
    window.addEventListener("focus", scrollToNow);
    return () => window.removeEventListener("focus", scrollToNow);
  }, [scrollToNow]);

  return scroller;
}
