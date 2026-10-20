import { useCallback, useEffect, useRef, useState } from "react";

export function useAnnouncement() {
  const [message, setMessage] = useState("");
  const pendingAnnouncement = useRef<number | undefined>(undefined);

  useEffect(() => () => window.clearTimeout(pendingAnnouncement.current), []);

  const announce = useCallback((next: string) => {
    window.clearTimeout(pendingAnnouncement.current);
    setMessage("");
    pendingAnnouncement.current = window.setTimeout(() => setMessage(next), 0);
  }, []);

  return { message, announce };
}
