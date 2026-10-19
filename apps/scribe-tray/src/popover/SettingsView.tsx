import { SettingsPanel } from "./SettingsPanel";
import { useTranscriptsFolder } from "./useTranscriptsFolder";

interface SettingsViewProps {
  readonly isRecording: boolean;
  readonly onBack: () => void;
}

export function SettingsView({ isRecording, onBack }: SettingsViewProps) {
  const { folder, status, change } = useTranscriptsFolder();

  return (
    <SettingsPanel
      folder={folder}
      status={status}
      isRecording={isRecording}
      onChangeFolder={() => void change()}
      onBack={onBack}
    />
  );
}
