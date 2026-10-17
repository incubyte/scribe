import { useId } from "react";
import { describeSessionLength, formatSessionDate } from "./format";
import type { SessionSummary } from "./types";

interface RecentSessionsProps {
  readonly sessions: readonly SessionSummary[];
  readonly onOpen: (id: string) => void;
}

export function RecentSessions({ sessions, onOpen }: RecentSessionsProps) {
  const headingId = useId();

  return (
    <section className="recent" aria-labelledby={headingId}>
      <h2 id={headingId} className="recent__heading">
        Recent sessions
      </h2>
      {sessions.length === 0 ? (
        <p className="recent__empty">No sessions yet</p>
      ) : (
        <ul className="recent__list" aria-labelledby={headingId}>
          {sessions.map((session) => (
            <li key={session.id}>
              <button className="session" type="button" onClick={() => onOpen(session.id)}>
                <span className="session__title">{session.title}</span>
                <span className="session__meta">{formatSessionDate(session.started_at)}</span>
                <span className="session__meta">{describeSessionLength(session)}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
