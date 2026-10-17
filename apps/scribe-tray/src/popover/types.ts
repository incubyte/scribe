export type Speaker = "Me" | "Them";

export type RecorderPhase = "locked" | "idle" | "recording";

export interface SnapshotLine {
  readonly speaker: Speaker;
  readonly start_ms: number;
  readonly text: string;
}

export interface RecorderSnapshot {
  readonly state: RecorderPhase;
  readonly started_at: string | null;
  readonly lines: readonly SnapshotLine[];
}

export type SessionStatus = "recording" | "complete" | "incomplete";

export interface SessionSummary {
  readonly id: string;
  readonly title: string;
  readonly started_at: string;
  readonly duration_seconds: number | null;
  readonly status: SessionStatus;
}
