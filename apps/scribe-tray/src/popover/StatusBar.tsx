import { formatElapsed } from "./format";
import type { RecorderPhase } from "./types";
import { useNow } from "./useElapsed";
import { usePhaseAnnouncement } from "./usePhaseAnnouncement";
import { usePrefersReducedMotion } from "./usePrefersReducedMotion";

interface StatusBarProps {
  readonly phase: RecorderPhase;
  readonly startedAt: number | null;
  readonly startFailure: string | null;
  readonly onStart: () => void;
  readonly onStop: () => void;
}

interface ControlLook {
  readonly label: string;
  readonly className: string;
}

const STATUS_TEXT: Readonly<Record<RecorderPhase, string>> = {
  locked: "Not ready yet",
  idle: "Ready to record",
  recording: "Recording",
};

const DETAIL_TEXT: Readonly<Record<RecorderPhase, string>> = {
  locked: "Scribe is getting set up.",
  idle: "Your mic and this computer's sound, transcribed on this computer.",
  recording: "Recording",
};

const START_LOOK: ControlLook = { label: "Start recording", className: "record-button" };
const STOP_LOOK: ControlLook = {
  label: "Stop recording",
  className: "record-button record-button--recording",
};

const START_FAILURE_TEXT: Readonly<Record<string, string>> = {
  microphone_denied: "Scribe can't use the microphone.",
};
const GENERIC_START_FAILURE = "Recording could not start.";

function describeStartFailure(reasonKind: string): string {
  return START_FAILURE_TEXT[reasonKind] ?? GENERIC_START_FAILURE;
}

export function StatusBar({ phase, startedAt, startFailure, onStart, onStop }: StatusBarProps) {
  return (
    <header className="status-bar">
      <RecordingButton phase={phase} onStart={onStart} onStop={onStop} />
      <div className="status-bar__summary">
        {phase === "recording" ? (
          <Elapsed startedAt={startedAt} />
        ) : (
          <p className="status-bar__headline">{STATUS_TEXT[phase]}</p>
        )}
        <p className="status-bar__detail">
          <RecordingDot phase={phase} />
          <span>{DETAIL_TEXT[phase]}</span>
        </p>
      </div>
      <StatusMessage phase={phase} startFailure={startFailure} />
    </header>
  );
}

type RecordingButtonProps = Pick<StatusBarProps, "phase" | "onStart" | "onStop">;

function RecordingButton({ phase, onStart, onStop }: RecordingButtonProps) {
  const recording = phase === "recording";
  const look = recording ? STOP_LOOK : START_LOOK;

  return (
    <button
      className={look.className}
      type="button"
      disabled={phase === "locked"}
      onClick={recording ? onStop : onStart}
    >
      <RecordGlyph recording={recording} />
      <span className="visually-hidden">{look.label}</span>
    </button>
  );
}

function RecordGlyph({ recording }: Readonly<{ recording: boolean }>) {
  return (
    <svg className="record-button__glyph" viewBox="0 0 22 22" aria-hidden="true">
      {recording ? (
        <rect x="6" y="6" width="10" height="10" rx="2.5" fill="currentColor" />
      ) : (
        <circle cx="11" cy="11" r="7" fill="currentColor" />
      )}
    </svg>
  );
}

type StatusMessageProps = Pick<StatusBarProps, "phase" | "startFailure">;

function StatusMessage({ phase, startFailure }: StatusMessageProps) {
  const announcement = usePhaseAnnouncement(phase);
  const failure = startFailure === null ? null : describeStartFailure(startFailure);

  return (
    <p
      className={failure === null ? "visually-hidden" : "status-bar__failure"}
      role="status"
      aria-live="polite"
      aria-label="Recording status"
    >
      {failure ?? announcement}
    </p>
  );
}

function RecordingDot({ phase }: Readonly<{ phase: RecorderPhase }>) {
  const reducedMotion = usePrefersReducedMotion();
  const classes = ["status-bar__dot"];
  if (phase === "recording") {
    classes.push("status-bar__dot--live");
  }
  if (phase === "recording" && !reducedMotion) {
    classes.push("status-bar__dot--pulsing");
  }

  return <span data-testid="recording-dot" className={classes.join(" ")} aria-hidden="true" />;
}

function Elapsed({ startedAt }: Readonly<{ startedAt: number | null }>) {
  const now = useNow(startedAt);

  return (
    <span className="status-bar__elapsed" role="timer">
      {formatElapsed(now - (startedAt ?? now))}
    </span>
  );
}
