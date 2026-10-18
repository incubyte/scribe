import { useEffect, useState } from "react";

const TICK_MS = 1000;

export function useNow(startedAt: number | null): number {
  const [now, setNow] = useState(Date.now);

  useEffect(() => {
    if (startedAt === null) {
      return undefined;
    }
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), TICK_MS);
    return () => clearInterval(timer);
  }, [startedAt]);

  return now;
}
