import { useEffect, useRef } from "react";

interface SettingsPanelProps {
  readonly folder: string | null;
  readonly isRecording: boolean;
  readonly onChangeFolder: () => void;
  readonly onBack: () => void;
}

const CHANGE_LABEL = "Change Transcripts Folder";
const RECORDING_REASON = "Stop recording to change the folder.";

export function SettingsPanel({ folder, isRecording, onChangeFolder, onBack }: SettingsPanelProps) {
  const heading = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    heading.current?.focus();
  }, []);

  return (
    <section className="settings" aria-labelledby="settings-heading">
      <h2 id="settings-heading" className="settings__heading" ref={heading} tabIndex={-1}>
        Settings
      </h2>
      <p className="settings__folder">
        Transcripts folder: {folder}
      </p>
      <button
        className="actions__primary"
        type="button"
        disabled={isRecording}
        aria-describedby={isRecording ? "settings-reason" : undefined}
        onClick={onChangeFolder}
      >
        {CHANGE_LABEL}
      </button>
      {isRecording ? (
        <p id="settings-reason" className="settings__reason">
          {RECORDING_REASON}
        </p>
      ) : null}
      <button className="actions__link" type="button" onClick={onBack}>
        Back
      </button>
    </section>
  );
}
