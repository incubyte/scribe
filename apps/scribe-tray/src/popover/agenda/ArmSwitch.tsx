import { armMeeting, disarmMeeting } from "../commands";
import type { AgendaBlock } from "../types";

interface ArmSwitchProps {
  readonly block: AgendaBlock;
}

export function ArmSwitch({ block }: ArmSwitchProps) {
  const toggle = (armed: boolean) => (armed ? armMeeting : disarmMeeting)(block.event_id);

  return (
    <label className="agenda__switch">
      <input
        type="checkbox"
        role="switch"
        className="agenda__switch-input"
        checked={block.armed}
        onChange={(event) => void toggle(event.target.checked)}
      />
      <span className="visually-hidden">Record {block.subject} automatically</span>
    </label>
  );
}
