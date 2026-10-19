import { useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import {
  copyMcpConfig,
  hidePopover,
  openSession,
  openTranscriptsFolder,
  startRecording,
  stopRecording,
} from "./commands";
import { LiveTranscript } from "./LiveTranscript";
import { ModelDownloadPanel } from "./ModelDownloadPanel";
import { PopoverActions } from "./PopoverActions";
import { RecentSessions } from "./RecentSessions";
import { SettingsPanel } from "./SettingsPanel";
import { StatusBar } from "./StatusBar";
import { useClaudeDesktopStatus } from "./useClaudeDesktopStatus";
import { useModelDownload } from "./useModelDownload";
import { useRecordingFeed } from "./useRecordingFeed";
import { useTranscriptsFolder } from "./useTranscriptsFolder";

function useEscapeToHide() {
  useEffect(() => {
    const hideOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void hidePopover();
      }
    };
    document.addEventListener("keydown", hideOnEscape);
    return () => document.removeEventListener("keydown", hideOnEscape);
  }, []);
}

export function Popover() {
  const { feed, recent } = useRecordingFeed();
  const { download, retry } = useModelDownload();
  const claudeDesktop = useClaudeDesktopStatus();
  const transcriptsFolder = useTranscriptsFolder();
  const [view, setView] = useState<"main" | "settings">("main");
  const settingsButton = useRef<HTMLButtonElement>(null);
  useEscapeToHide();

  const leaveSettings = () => {
    flushSync(() => setView("main"));
    settingsButton.current?.focus();
  };

  if (view === "settings") {
    return (
      <main className="popover">
        <h1 className="visually-hidden">Scribe</h1>
        <SettingsPanel
          folder={transcriptsFolder.folder}
          isRecording={feed.phase === "recording"}
          onChangeFolder={() => void transcriptsFolder.change()}
          onBack={leaveSettings}
        />
      </main>
    );
  }

  return (
    <main className="popover">
      <h1 className="visually-hidden">Scribe</h1>
      <StatusBar
        phase={feed.phase}
        startedAt={feed.startedAt}
        startFailure={feed.startFailure}
        onStart={() => void startRecording()}
        onStop={() => void stopRecording()}
      />
      <ModelDownloadPanel download={download} onRetry={retry} />
      {feed.phase === "recording" ? (
        <LiveTranscript lines={feed.lines} partials={feed.partials} />
      ) : null}
      {recent === null ? null : (
        <RecentSessions sessions={recent} onOpen={(id) => void openSession(id)} />
      )}
      <PopoverActions
        claudeStatus={claudeDesktop.status}
        onConnectClaudeDesktop={claudeDesktop.connect}
        onCopyMcpConfig={() => void copyMcpConfig()}
        onOpenFolder={() => void openTranscriptsFolder()}
        onOpenSettings={() => setView("settings")}
        settingsButtonRef={settingsButton}
      />
    </main>
  );
}
