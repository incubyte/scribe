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

export type ShortcutKind = "full_screen" | "region";

export interface ShortcutView {
  readonly kind: ShortcutKind;
  readonly label: string;
}

export interface PressedKeys {
  readonly code: string;
  readonly control: boolean;
  readonly alt: boolean;
  readonly shift: boolean;
  readonly meta: boolean;
}

export type RebindOutcome =
  | { readonly outcome: "accepted"; readonly label: string }
  | { readonly outcome: "needs_modifier"; readonly message: string }
  | { readonly outcome: "duplicate_of"; readonly kind: ShortcutKind }
  | { readonly outcome: "in_use_by_another_app" };

export type OutlookState = "disconnected" | "connecting" | "connected" | "reconnect_needed";

export interface OutlookConnection {
  readonly state: OutlookState;
  readonly display_name: string | null;
  readonly email: string | null;
  readonly error: string | null;
}

export type AgendaStatus = "ready" | "loading" | "failed";

export interface AgendaBlock {
  readonly event_id: string;
  readonly subject: string;
  readonly starts_at: string;
  readonly ends_at: string;
  readonly is_teams: boolean;
  readonly is_recurring: boolean;
  readonly top_minutes: number;
  readonly height_minutes: number;
  readonly column: number;
  readonly columns: number;
  readonly armed: boolean;
  readonly armed_state: ArmedState | null;
}

export type ArmedState = "missed";

export interface HiddenMeeting {
  readonly event_id: string;
  readonly subject: string;
  readonly starts_at: string;
  readonly ends_at: string;
}

export interface AgendaChip {
  readonly top_minutes: number;
  readonly height_minutes: number;
  readonly hidden: readonly HiddenMeeting[];
}

export interface AgendaView {
  readonly status: AgendaStatus;
  readonly stale: boolean;
  readonly day_start: string | null;
  readonly fetched_at: string | null;
  readonly blocks: readonly AgendaBlock[];
  readonly overflow: readonly AgendaChip[];
}

export type CallMeetingState = "matched" | "ambiguous" | "unmatched";

export interface MeetingCandidate {
  readonly event_id: string;
  readonly subject: string;
  readonly starts_at: string;
  readonly ends_at: string;
}

export interface CallMeetingView {
  readonly state: CallMeetingState;
  readonly selected_event_id: string | null;
  readonly not_a_meeting: boolean;
  readonly candidates: readonly MeetingCandidate[];
}

export interface SwitchPromptView {
  readonly from_subject: string;
  readonly to_subject: string;
}
