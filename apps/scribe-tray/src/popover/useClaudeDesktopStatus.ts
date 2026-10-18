import { useCallback, useEffect, useState } from "react";
import { connectClaudeDesktop, fetchClaudeDesktopStatus } from "./commands";
import type { ClaudeDesktopStatus } from "./types";

export function useClaudeDesktopStatus() {
  const [status, setStatus] = useState<ClaudeDesktopStatus | null>(null);
  const forgetStatus = useCallback(() => setStatus(null), []);

  useEffect(() => {
    const refresh = () => void fetchClaudeDesktopStatus().then(setStatus, forgetStatus);
    refresh();
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, [forgetStatus]);

  const connect = useCallback(
    () => void connectClaudeDesktop().then(setStatus, forgetStatus),
    [forgetStatus],
  );

  return { status, connect };
}
