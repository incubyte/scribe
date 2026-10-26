import { useId } from "react";
import { AgendaGrid } from "./AgendaGrid";
import { clockLabel, todayHeading } from "./agendaTimes";
import { useTodayAgenda } from "./useTodayAgenda";
import type { AgendaView } from "../types";

interface AgendaBodyProps {
  readonly view: AgendaView;
  readonly headingId: string;
  readonly onRetry: () => void;
}

function hasMeetings(view: AgendaView): boolean {
  return view.blocks.length > 0 || view.overflow.length > 0;
}

function AgendaBody({ view, headingId, onRetry }: AgendaBodyProps) {
  if (view.status === "loading") {
    return <p className="agenda__empty">Loading your calendar</p>;
  }
  if (view.status === "failed") {
    return (
      <p className="agenda__empty">
        Couldn&apos;t load your calendar{" "}
        <button type="button" className="actions__link" onClick={onRetry}>
          Retry
        </button>
      </p>
    );
  }
  if (!hasMeetings(view) || view.day_start === null) {
    return <p className="agenda__empty">No meetings today</p>;
  }
  return <AgendaGrid view={view} dayStart={view.day_start} labelledBy={headingId} />;
}

export function TodayAgenda() {
  const headingId = useId();
  const { view, refresh } = useTodayAgenda();

  return (
    <section className="agenda" aria-labelledby={headingId}>
      <h2 id={headingId} className="recent__heading">
        {todayHeading(Date.now())}
      </h2>
      {view === null ? null : <AgendaBody view={view} headingId={headingId} onRetry={refresh} />}
      {view?.stale && view.fetched_at !== null ? (
        <p className="agenda__updated">Updated {clockLabel(view.fetched_at)}</p>
      ) : null}
    </section>
  );
}
