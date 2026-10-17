import type { SessionSummary } from "./types";

const SECONDS_PER_MINUTE = 60;
const MINUTES_PER_HOUR = 60;
const MS_PER_SECOND = 1000;

const STATUS_LABELS = {
  recording: "In progress",
  complete: "Complete",
  incomplete: "Incomplete",
} as const;

function twoDigits(value: number): string {
  return String(value).padStart(2, "0");
}

export function formatElapsed(elapsedMs: number): string {
  const totalSeconds = Math.max(0, Math.floor(elapsedMs / MS_PER_SECOND));
  const seconds = totalSeconds % SECONDS_PER_MINUTE;
  const totalMinutes = Math.floor(totalSeconds / SECONDS_PER_MINUTE);
  const minutes = totalMinutes % MINUTES_PER_HOUR;
  const hours = Math.floor(totalMinutes / MINUTES_PER_HOUR);
  const clock = `${twoDigits(minutes)}:${twoDigits(seconds)}`;
  return hours > 0 ? `${hours}:${clock}` : clock;
}

export function formatSessionDate(startedAt: string): string {
  return new Date(startedAt).toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

export function describeSessionLength(session: SessionSummary): string {
  if (session.duration_seconds === null) {
    return STATUS_LABELS[session.status];
  }
  const minutes = Math.floor(session.duration_seconds / SECONDS_PER_MINUTE);
  return minutes === 0 ? "<1 min" : `${minutes} min`;
}
