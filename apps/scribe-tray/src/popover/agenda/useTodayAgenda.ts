import { fetchTodaysAgenda, refreshAgenda } from "../commands";
import { isAgendaView } from "../guards";
import type { AgendaView } from "../types";
import { useBackendValue } from "../useBackendValue";

const AGENDA_EVENT = "agenda://updated";

const UNREADABLE: AgendaView = {
  status: "failed",
  stale: false,
  day_start: null,
  fetched_at: null,
  blocks: [],
  overflow: [],
};

const refreshThenFetch = () =>
  refreshAgenda()
    .then(fetchTodaysAgenda)
    .catch(() => UNREADABLE);

export function useTodayAgenda() {
  const { value: view, refresh } = useBackendValue(refreshThenFetch, AGENDA_EVENT, isAgendaView);
  return { view, refresh };
}
