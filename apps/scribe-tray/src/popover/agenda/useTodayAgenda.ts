import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { fetchTodaysAgenda, refreshAgenda } from "../commands";
import { isAgendaView } from "../guards";
import type { AgendaView } from "../types";

const AGENDA_EVENT = "agenda://updated";

const UNREADABLE: AgendaView = {
  status: "failed",
  stale: false,
  day_start: null,
  fetched_at: null,
  blocks: [],
  overflow: [],
};

export function useTodayAgenda() {
  const [view, setView] = useState<AgendaView | null>(null);
  const refresh = useCallback(
    () =>
      void refreshAgenda()
        .then(fetchTodaysAgenda)
        .then(setView, () => setView(UNREADABLE)),
    [],
  );

  useEffect(() => {
    refresh();
    window.addEventListener("focus", refresh);
    const unlisten = listen<unknown>(AGENDA_EVENT, ({ payload }) => {
      if (isAgendaView(payload)) {
        setView(payload);
      }
    });
    return () => {
      window.removeEventListener("focus", refresh);
      void unlisten.then((stop) => stop());
    };
  }, [refresh]);

  return { view, refresh };
}
