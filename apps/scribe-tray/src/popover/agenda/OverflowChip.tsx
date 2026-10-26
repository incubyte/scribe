import { useId, useState } from "react";
import { rangeLabel } from "./agendaTimes";
import type { AgendaChip } from "../types";

interface OverflowChipProps {
  readonly chip: AgendaChip;
}

export function OverflowChip({ chip }: OverflowChipProps) {
  const [isOpen, setIsOpen] = useState(false);
  const listId = useId();
  const count = chip.hidden.length;

  return (
    <li className="agenda__chip" style={{ top: `${chip.top_minutes}px` }}>
      <button
        type="button"
        className="agenda__chip-button"
        aria-expanded={isOpen}
        aria-controls={listId}
        aria-label={`${count} more ${count === 1 ? "meeting" : "meetings"}`}
        onClick={() => setIsOpen((open) => !open)}
      >
        +{count}
      </button>
      <ul id={listId} className="agenda__chip-list" hidden={!isOpen}>
        {chip.hidden.map((meeting) => (
          <li key={meeting.event_id}>{`${meeting.subject}, ${rangeLabel(meeting)}`}</li>
        ))}
      </ul>
    </li>
  );
}
