import { OutlookRow } from "./outlook/OutlookRow";
import { useOutlookConnection } from "./outlook/useOutlookConnection";
import { ScreenshotShortcuts } from "./ScreenshotShortcuts";
import { SettingsPanel } from "./SettingsPanel";
import { CANCELLED_MESSAGE, LISTENING_MESSAGE } from "./shortcutCapture";
import { useAnnouncement } from "./useAnnouncement";
import { useAppVersion } from "./useAppVersion";
import { useTranscriptsFolder } from "./useTranscriptsFolder";

interface SettingsViewProps {
  readonly isRecording: boolean;
  readonly onBack: () => void;
}

const SHORTCUT_STATUS_LABEL = "Shortcut status";
const MESSAGES_SHOWN_BY_THE_ROW = [LISTENING_MESSAGE, CANCELLED_MESSAGE];

export function SettingsView({ isRecording, onBack }: SettingsViewProps) {
  const { folder, status, change } = useTranscriptsFolder();
  const { message, announce } = useAnnouncement();
  const version = useAppVersion();
  const outlook = useOutlookConnection();
  const statusClass = MESSAGES_SHOWN_BY_THE_ROW.includes(message)
    ? "settings__status visually-hidden"
    : "settings__status";

  return (
    <SettingsPanel
      folder={folder}
      status={status}
      isRecording={isRecording}
      version={version}
      onChangeFolder={() => void change()}
      onBack={onBack}
    >
      <OutlookRow
        connection={outlook.connection}
        onConnect={outlook.connect}
        onDisconnect={outlook.disconnect}
      />
      <ScreenshotShortcuts onStatus={announce} />
      <p role="status" aria-label={SHORTCUT_STATUS_LABEL} className={statusClass}>
        {message}
      </p>
    </SettingsPanel>
  );
}
