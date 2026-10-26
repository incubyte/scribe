import { TodayAgenda } from "./agenda/TodayAgenda";
import { RecentSessions } from "./RecentSessions";
import type { SessionSummary } from "./types";

interface SessionsOrAgendaProps {
  readonly isOutlookConnected: boolean;
  readonly recent: readonly SessionSummary[] | null;
  readonly onOpen: (id: string) => void;
}

export function SessionsOrAgenda({ isOutlookConnected, recent, onOpen }: SessionsOrAgendaProps) {
  if (isOutlookConnected) {
    return <TodayAgenda />;
  }
  return recent === null ? null : <RecentSessions sessions={recent} onOpen={onOpen} />;
}
