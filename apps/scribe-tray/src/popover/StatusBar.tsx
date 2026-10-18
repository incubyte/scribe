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
  idle: "Not recording",
  recording: "Recording",
};

const START_LOOK: ControlLook = { label: "Start recording", className: "button" };
const STOP_LOOK: ControlLook = { label: "Stop recording", className: "button button--recording" };

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
      <RecordingDot phase={phase} />
      <span className="status-bar__text">{STATUS_TEXT[phase]}</span>
      {phase === "recording" ? <Elapsed startedAt={startedAt} /> : null}
      <RecordingButton phase={phase} onStart={onStart} onStop={onStop} />
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
      {look.label}
    </button>
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
