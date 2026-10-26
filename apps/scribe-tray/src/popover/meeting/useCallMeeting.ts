import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { chooseMeeting, fetchCallMeeting } from "../commands";
import { isCallMeetingView } from "../guards";
import type { CallMeetingView } from "../types";

const MATCH_EVENT = "meeting://match";

export function useCallMeeting() {
  const [view, setView] = useState<CallMeetingView | null>(null);
  const refresh = useCallback(
    () => void fetchCallMeeting().then(setView, () => setView(null)),
    [],
  );

  useEffect(() => {
    refresh();
    window.addEventListener("focus", refresh);
    const unlisten = listen<unknown>(MATCH_EVENT, ({ payload }) => {
      if (isCallMeetingView(payload)) {
        setView(payload);
      }
    });
    return () => {
      window.removeEventListener("focus", refresh);
      void unlisten.then((stop) => stop());
    };
  }, [refresh]);

  const choose = useCallback(
    (eventId: string | null) => void chooseMeeting(eventId).then(setView, refresh),
    [refresh],
  );

  return { view, choose };
}
