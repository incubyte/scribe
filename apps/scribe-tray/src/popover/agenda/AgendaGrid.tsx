import { AgendaBlockItem } from "./AgendaBlockItem";
import { agendaItems, itemKey } from "./agendaItems";
import { hasEnded, hourLabel, minutesSince } from "./agendaTimes";
import { OverflowChip } from "./OverflowChip";
import { useMinuteClock } from "./useMinuteClock";
import { useScrollNowIntoView } from "./useScrollNowIntoView";
import type { AgendaView } from "../types";

const HOURS = Array.from({ length: 24 }, (_, hour) => hour);

interface AgendaGridProps {
  readonly view: AgendaView;
  readonly dayStart: string;
  readonly labelledBy: string;
}

export function AgendaGrid({ view, dayStart, labelledBy }: AgendaGridProps) {
  const now = useMinuteClock();
  const minutesNow = minutesSince(dayStart, now);
  const scroller = useScrollNowIntoView(minutesNow);

  return (
    <div className="agenda__scroller" ref={scroller}>
      <div className="agenda__day">
        {HOURS.map((hour) => (
          <span key={hour} className="agenda__hour" style={{ top: `${hour * 60}px` }} aria-hidden="true">
            {hourLabel(hour)}
          </span>
        ))}
        <ul className="agenda__track" aria-labelledby={labelledBy}>
          {agendaItems(view).map((item) =>
            item.kind === "block" ? (
              <AgendaBlockItem
                key={itemKey(item)}
                block={item.block}
                ended={hasEnded(item.block, now)}
              />
            ) : (
              <OverflowChip key={itemKey(item)} chip={item.chip} />
            ),
          )}
        </ul>
        <div className="agenda__now" style={{ top: `${minutesNow}px` }} aria-hidden="true" />
      </div>
    </div>
  );
}
