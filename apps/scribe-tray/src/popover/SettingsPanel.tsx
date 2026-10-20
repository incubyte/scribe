import { type ReactNode, useEffect, useId, useRef } from "react";
import { RECORDING_REASON } from "./folderChangeMessage";

interface SettingsPanelProps {
  readonly folder: string | null;
  readonly status: string | null;
  readonly isRecording: boolean;
  readonly onChangeFolder: () => void;
  readonly onBack: () => void;
  readonly children?: ReactNode;
}

export const CHANGE_LABEL = "Change Transcripts Folder";
const LOADING_TEXT = "Loading…";

export function SettingsPanel({
  folder,
  status,
  isRecording,
  onChangeFolder,
  onBack,
  children,
}: SettingsPanelProps) {
  const heading = useRef<HTMLHeadingElement>(null);
  const headingId = useId();
  const reasonId = useId();

  useEffect(() => {
    heading.current?.focus();
  }, []);

  return (
    <section className="settings" aria-labelledby={headingId}>
      <h2
        id={headingId}
        className="settings__heading"
        ref={heading}
        tabIndex={-1}
      >
        Settings
      </h2>
      <p className="settings__folder">
        {folder === null ? LOADING_TEXT : `Transcripts folder: ${folder}`}
      </p>
      <button
        className="actions__primary"
        type="button"
        disabled={isRecording}
        aria-describedby={isRecording ? reasonId : undefined}
        onClick={onChangeFolder}
      >
        {CHANGE_LABEL}
      </button>
      {isRecording ? (
        <p id={reasonId} className="settings__reason">
          {RECORDING_REASON}
        </p>
      ) : null}
      <p role="status" className="settings__status">
        {status}
      </p>
      {children}
      <button className="actions__link" type="button" onClick={onBack}>
        Back
      </button>
    </section>
  );
}
