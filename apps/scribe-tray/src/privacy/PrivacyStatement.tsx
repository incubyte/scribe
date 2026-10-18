import { invoke } from "@tauri-apps/api/core";
import { useEffect, useId, useRef } from "react";

const STATEMENTS = [
  "Speech is transcribed on-device.",
  "Audio is never saved.",
  "Nothing — audio or text — is sent to any server.",
  "Transcripts are saved as text files in Documents/Scribe.",
] as const;

export function PrivacyStatement() {
  const acknowledgeButton = useRef<HTMLButtonElement>(null);
  const statementsId = useId();

  useEffect(() => {
    acknowledgeButton.current?.focus();
  }, []);

  return (
    <main className="window">
      <h1 className="window__title">Scribe runs entirely on this computer</h1>
      <ul id={statementsId}>
        {STATEMENTS.map((statement) => (
          <li key={statement}>{statement}</li>
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
