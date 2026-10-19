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

export type ModelDownload =
  | { readonly state: "ready" }
  | { readonly state: "downloading"; readonly percent: number | null }
  | { readonly state: "failed"; readonly reason_kind: string };

export type FolderChange =
  | { readonly outcome: "cancelled" }
  | { readonly outcome: "unchanged" }
  | { readonly outcome: "recording" }
  | { readonly outcome: "changed"; readonly folder: string; readonly moved: number; readonly kept: number };

export type ClaudeDesktopStatus = "not_installed" | "not_connected" | "outdated" | "connected";
