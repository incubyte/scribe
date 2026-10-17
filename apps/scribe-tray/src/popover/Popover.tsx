import { useEffect } from "react";
import { hidePopover, openSession, startRecording, stopRecording } from "./commands";
import { LiveTranscript } from "./LiveTranscript";
import { RecentSessions } from "./RecentSessions";
import { StatusBar } from "./StatusBar";
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
  useEscapeToHide();

  return (
    <main className="popover">
      <h1 className="visually-hidden">Scribe</h1>
      <StatusBar
        phase={feed.phase}
        startedAt={feed.startedAt}
        onStart={() => void startRecording()}
        onStop={() => void stopRecording()}
      />
      {feed.phase === "recording" ? (
        <LiveTranscript lines={feed.lines} partials={feed.partials} />
      ) : null}
      {recent === null ? null : (
        <RecentSessions sessions={recent} onOpen={(id) => void openSession(id)} />
      )}
    </main>
  );
}
