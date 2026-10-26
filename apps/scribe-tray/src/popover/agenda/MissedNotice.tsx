import { recordMeeting } from "../commands";
import type { AgendaBlock } from "../types";

interface MissedNoticeProps {
  readonly block: AgendaBlock;
}

export function MissedNotice({ block }: MissedNoticeProps) {
  return (
    <span className="agenda__missed">
      <span>No call detected</span>
      <button
        type="button"
        className="agenda__record"
        aria-label={`Record ${block.subject}`}
        onClick={() => void recordMeeting(block.event_id)}
      >
        Record
      </button>
    </span>
  );
}
