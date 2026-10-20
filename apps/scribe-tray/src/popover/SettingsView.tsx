import { useState } from "react";
import { ScreenshotShortcuts } from "./ScreenshotShortcuts";
import { SettingsPanel } from "./SettingsPanel";
import { useTranscriptsFolder } from "./useTranscriptsFolder";

interface SettingsViewProps {
  readonly isRecording: boolean;
  readonly onBack: () => void;
}

export function SettingsView({ isRecording, onBack }: SettingsViewProps) {
  const { folder, status, change } = useTranscriptsFolder();
  const [shortcutStatus, setShortcutStatus] = useState<string | null>(null);

  const changeFolder = () => {
    setShortcutStatus(null);
    void change();
  };

  return (
    <>
      <SettingsPanel
        folder={folder}
        status={shortcutStatus ?? status}
        isRecording={isRecording}
        onChangeFolder={changeFolder}
        onBack={onBack}
      />
      <ScreenshotShortcuts onStatus={setShortcutStatus} />
    </>
  );
}
