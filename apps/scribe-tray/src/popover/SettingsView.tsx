import { ScreenshotShortcuts } from "./ScreenshotShortcuts";
import { SettingsPanel } from "./SettingsPanel";
import { useAnnouncement } from "./useAnnouncement";
import { useTranscriptsFolder } from "./useTranscriptsFolder";

interface SettingsViewProps {
  readonly isRecording: boolean;
  readonly onBack: () => void;
}

export function SettingsView({ isRecording, onBack }: SettingsViewProps) {
  const { folder, status, change } = useTranscriptsFolder();
  const { announcement, announce } = useAnnouncement();

  return (
    <SettingsPanel
      folder={folder}
      status={status}
      isRecording={isRecording}
      onChangeFolder={() => void change()}
      onBack={onBack}
    >
      <ScreenshotShortcuts onStatus={announce} />
      <p key={announcement.sequence} role="status" className="settings__status">
        {announcement.message}
      </p>
    </SettingsPanel>
  );
}
