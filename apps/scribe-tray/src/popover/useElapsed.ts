import { useEffect, useState } from "react";

const TICK_MS = 1000;

export function useNow(running: boolean): number {
  const [now, setNow] = useState(Date.now);

  useEffect(() => {
    if (!running) {
      return undefined;
    }
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), TICK_MS);
    return () => clearInterval(timer);
  }, [running]);

  return now;
}
