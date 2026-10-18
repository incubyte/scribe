import { useCallback, useEffect, useState } from "react";
import { connectClaudeDesktop, fetchClaudeDesktopStatus } from "./commands";
import type { ClaudeDesktopStatus } from "./types";

export function useClaudeDesktopStatus() {
  const [status, setStatus] = useState<ClaudeDesktopStatus | null>(null);

  useEffect(() => {
    const refresh = () => void fetchClaudeDesktopStatus().then(setStatus);
    refresh();
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, []);

  const connect = useCallback(() => void connectClaudeDesktop().then(setStatus), []);

  return { status, connect };
}
