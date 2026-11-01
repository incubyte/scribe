import { type ReactNode, useEffect, useId, useRef } from "react";
import { RECORDING_REASON } from "./folderChangeMessage";

interface SettingsPanelProps {
  readonly folder: string | null;
  readonly status: string | null;
  readonly isRecording: boolean;
  readonly version: string | null;
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
  version,
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
      <header className="settings__header">
        <button
          className="actions__icon"
          type="button"
          aria-label="Back"
          onClick={onBack}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true" width="16" height="16">
            <path
              d="M10 3L5 8l5 5"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </button>
        <h2
          id={headingId}
          className="settings__heading"
          ref={heading}
          tabIndex={-1}
        >
          Settings
        </h2>
      </header>
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
      <p role="status" aria-label="Folder status" className="settings__status">
        {status}
      </p>
      {children}
      {version === null ? null : (
        <p className="settings__version">Version {version}</p>
      )}
    </section>
  );
}
