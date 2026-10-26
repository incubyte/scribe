import { invoke } from "@tauri-apps/api/core";
import { useEffect, useId, useRef } from "react";

const STATEMENTS = [
  "Speech is transcribed on-device.",
  "Audio is never saved.",
  "Audio and transcripts are never sent to any server.",
  "If you connect Outlook, Scribe signs you in at login.microsoftonline.com and reads today's meetings (times, subjects and invitees) from graph.microsoft.com.",
  "Transcripts are saved as text files in your transcripts folder, Documents/Scribe unless you change it in Settings.",
] as const;

export function PrivacyStatement() {
  const acknowledgeButton = useRef<HTMLButtonElement>(null);
  const statementsId = useId();

  useEffect(() => {
    acknowledgeButton.current?.focus();
  }, []);

  return (
    <main className="window">
      <ScribeMark />
      <h1 className="window__title">Scribe runs entirely on this computer</h1>
      <ul id={statementsId} className="checklist">
        {STATEMENTS.map((statement) => (
          <li key={statement} className="checklist__item">
            {statement}
          </li>
        ))}
      </ul>
      <button
        ref={acknowledgeButton}
        className="button"
        type="button"
        aria-describedby={statementsId}
        onClick={() => void invoke("acknowledge_privacy")}
      >
        I understand
      </button>
    </main>
  );
}

function ScribeMark() {
  return (
    <svg className="window__mark" viewBox="0 0 36 36" aria-hidden="true">
      <g stroke="currentColor" strokeWidth="3.6" strokeLinecap="round">
        <line x1="7" y1="14" x2="7" y2="22" />
        <line x1="13.7" y1="8" x2="13.7" y2="28" />
        <line x1="20.3" y1="11" x2="20.3" y2="25" />
        <line x1="27" y1="15.5" x2="27" y2="20.5" />
      </g>
    </svg>
  );
}
