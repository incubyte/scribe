import { formatElapsed } from "./format";
import type { RecorderPhase } from "./types";
import { useNow } from "./useElapsed";
import { usePrefersReducedMotion } from "./usePrefersReducedMotion";

interface StatusBarProps {
  readonly phase: RecorderPhase;
  readonly startedAt: number | null;
  readonly onStart: () => void;
  readonly onStop: () => void;
}

const STATUS_TEXT: Readonly<Record<RecorderPhase, string>> = {
  locked: "Not ready yet",
  idle: "Not recording",
  recording: "Recording",
};

export function StatusBar({ phase, startedAt, onStart, onStop }: StatusBarProps) {
  const recording = phase === "recording";

  return (
    <header className="status-bar">
      <RecordingDot recording={recording} />
      <span className="status-bar__text">{STATUS_TEXT[phase]}</span>
      {recording ? <Elapsed startedAt={startedAt} /> : null}
      {recording ? (
        <button className="button button--recording" type="button" onClick={onStop}>
          Stop recording
        </button>
      ) : (
        <button className="button" type="button" disabled={phase === "locked"} onClick={onStart}>
          Start recording
        </button>
      )}
    </header>
  );
}

function RecordingDot({ recording }: Readonly<{ recording: boolean }>) {
  const reducedMotion = usePrefersReducedMotion();
  const classes = ["status-bar__dot"];
  if (recording) {
    classes.push("status-bar__dot--live");
  }
  if (recording && !reducedMotion) {
    classes.push("status-bar__dot--pulsing");
  }

  return <span data-testid="recording-dot" className={classes.join(" ")} aria-hidden="true" />;
}

function Elapsed({ startedAt }: Readonly<{ startedAt: number | null }>) {
  const now = useNow(startedAt !== null);

  return (
    <span className="status-bar__elapsed" role="timer">
      {formatElapsed(now - (startedAt ?? now))}
    </span>
  );
}
