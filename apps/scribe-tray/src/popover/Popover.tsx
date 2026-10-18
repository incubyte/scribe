import { useEffect } from "react";
import { hidePopover, openSession, startRecording, stopRecording } from "./commands";
import { LiveTranscript } from "./LiveTranscript";
import { ModelDownloadPanel } from "./ModelDownloadPanel";
import { RecentSessions } from "./RecentSessions";
import { StatusBar } from "./StatusBar";
import { useModelDownload } from "./useModelDownload";
import { useRecordingFeed } from "./useRecordingFeed";

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
  useEscapeToHide();

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
    </main>
  );
}
