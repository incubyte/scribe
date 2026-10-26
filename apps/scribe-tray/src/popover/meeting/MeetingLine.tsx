import { useId, useState } from "react";
import { rangeLabel } from "../agenda/agendaTimes";
import type { CallMeetingView, MeetingCandidate } from "../types";

interface MeetingLineProps {
  readonly view: CallMeetingView | null;
  readonly onChoose: (eventId: string | null) => void;
}

interface ChoicesProps {
  readonly view: CallMeetingView;
  readonly onChoose: (eventId: string | null) => void;
  readonly id: string;
}

const QUESTION = "Which meeting is this?";
const NOT_A_MEETING = "Not a calendar meeting";
const NOT_A_MEETING_VALUE = "";
const GROUP_NAME = "call-meeting";

const describe = (candidate: MeetingCandidate) => `${candidate.subject} · ${rangeLabel(candidate)}`;

function summaryText(view: CallMeetingView): string {
  const selected = view.candidates.find((candidate) => candidate.event_id === view.selected_event_id);
  if (view.state === "ambiguous" || (selected === undefined && !view.not_a_meeting)) {
    return QUESTION;
  }
  return selected === undefined ? NOT_A_MEETING : describe(selected);
}

export function MeetingLine({ view, onChoose }: MeetingLineProps) {
  if (view === null || view.state === "unmatched") {
    return null;
  }
  return <OpenableLine view={view} onChoose={onChoose} />;
}

function OpenableLine({ view, onChoose }: Readonly<{ view: CallMeetingView } & Pick<MeetingLineProps, "onChoose">>) {
  const [expanded, setExpanded] = useState<boolean | null>(null);
  const choicesId = useId();
  const open = expanded ?? view.state === "ambiguous";

  return (
    <section className="meeting-line" aria-label="Meeting">
      <button
        className="meeting-line__summary"
        type="button"
        aria-expanded={open}
        aria-controls={choicesId}
        onClick={() => setExpanded(!open)}
      >
        <span>{summaryText(view)}</span>
        <span aria-hidden="true">{open ? "▴" : "▾"}</span>
      </button>
      {open ? <Choices view={view} onChoose={onChoose} id={choicesId} /> : null}
    </section>
  );
}

function Choices({ view, onChoose, id }: ChoicesProps) {
  return (
    <fieldset id={id} className="meeting-line__choices">
      <legend className="visually-hidden">{QUESTION}</legend>
      {view.candidates.map((candidate) => (
        <label key={candidate.event_id} className="meeting-line__choice">
          <input
            type="radio"
            name={GROUP_NAME}
            value={candidate.event_id}
            checked={candidate.event_id === view.selected_event_id}
            onChange={() => onChoose(candidate.event_id)}
          />
          <span>{describe(candidate)}</span>
        </label>
      ))}
      <label className="meeting-line__choice">
        <input
          type="radio"
          name={GROUP_NAME}
          value={NOT_A_MEETING_VALUE}
          checked={view.not_a_meeting}
          onChange={() => onChoose(null)}
        />
        <span>{NOT_A_MEETING}</span>
      </label>
    </fieldset>
  );
}
