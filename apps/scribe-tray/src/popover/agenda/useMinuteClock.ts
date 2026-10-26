import { useEffect, useState } from "react";

const MINUTE_MS = 60_000;

export function useMinuteClock(): number {
  const [now, setNow] = useState(Date.now);

  useEffect(() => {
    const update = () => setNow(Date.now());
    const timer = setInterval(update, MINUTE_MS);
    window.addEventListener("focus", update);
    return () => {
      clearInterval(timer);
      window.removeEventListener("focus", update);
    };
  }, []);

  return now;
}
