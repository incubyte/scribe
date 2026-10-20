import type {
  ClaudeDesktopStatus,
  FolderChange,
  ModelDownload,
  RebindOutcome,
  RecorderPhase,
  RecorderSnapshot,
  SessionStatus,
  SessionSummary,
  ShortcutKind,
  ShortcutView,
  SnapshotLine,
  Speaker,
} from "./types";
import {
  type Guard,
  hasFields,
  isListOf,
  isNullable,
  isNumber,
  isOneOf,
  isRecord,
  isString,
} from "../shared/guards";

const PHASES: readonly RecorderPhase[] = ["locked", "idle", "recording"];
const SPEAKERS: readonly Speaker[] = ["Me", "Them"];
const STATUSES: readonly SessionStatus[] = ["recording", "complete", "incomplete"];

export const isRecorderPhase = isOneOf<RecorderPhase>(PHASES);
export const isSpeaker = isOneOf<Speaker>(SPEAKERS);
const isSessionStatus = isOneOf<SessionStatus>(STATUSES);

export interface PartialPayload {
  readonly speaker: Speaker;
  readonly text: string;
}

export interface SegmentPayload extends PartialPayload {
  readonly start_ms: number;
}

export function isPartialPayload(value: unknown): value is PartialPayload {
  return hasFields(value, { speaker: isSpeaker, text: isString });
}

export function isSegmentPayload(value: unknown): value is SegmentPayload {
  return hasFields(value, { speaker: isSpeaker, text: isString, start_ms: isNumber });
}

function isSnapshotLine(value: unknown): value is SnapshotLine {
  return isSegmentPayload(value);
}

export function isRecorderSnapshot(value: unknown): value is RecorderSnapshot {
  return hasFields(value, {
    state: isRecorderPhase,
    started_at: isNullable(isString),
    lines: isListOf(isSnapshotLine),
  });
}

function isSessionSummary(value: unknown): value is SessionSummary {
  return hasFields(value, {
    id: isString,
    title: isString,
    started_at: isString,
    duration_seconds: isNullable(isNumber),
    status: isSessionStatus,
  });
}

export const isSessionSummaries = isListOf(isSessionSummary);

export function isModelDownload(value: unknown): value is ModelDownload {
  if (!isRecord(value)) {
    return false;
  }
  const checks: Readonly<Record<string, () => boolean>> = {
    ready: () => true,
    downloading: () => isNullable(isNumber)(value.percent),
    failed: () => isString(value.reason_kind),
  };
  return isString(value.state) && (checks[value.state]?.() ?? false);
}

export interface StatePayload {
  readonly state: RecorderPhase;
}

export function isStatePayload(value: unknown): value is StatePayload {
  return hasFields(value, { state: isRecorderPhase });
}

export interface StartFailedPayload {
  readonly reason_kind: string;
}

export function isStartFailedPayload(value: unknown): value is StartFailedPayload {
  return hasFields(value, { reason_kind: isString });
}

const CLAUDE_DESKTOP_STATUSES: readonly ClaudeDesktopStatus[] = ["not_installed", "not_connected", "outdated", "connected"];

export const isClaudeDesktopStatus: Guard<ClaudeDesktopStatus> = isOneOf(CLAUDE_DESKTOP_STATUSES);

export function isFolderChange(value: unknown): value is FolderChange {
  if (!isRecord(value)) {
    return false;
  }
  const checks: Readonly<Record<string, () => boolean>> = {
    cancelled: () => true,
    unchanged: () => true,
    recording: () => true,
    changed: () => hasFields(value, { folder: isString, moved: isNumber, kept: isNumber }),
  };
  return isString(value.outcome) && (checks[value.outcome]?.() ?? false);
}

const SHORTCUT_KINDS: readonly ShortcutKind[] = ["full_screen", "region"];

const isShortcutKind = isOneOf(SHORTCUT_KINDS);

function isShortcutView(value: unknown): value is ShortcutView {
  return hasFields(value, { kind: isShortcutKind, label: isString });
}

export const isShortcutViewList = isListOf(isShortcutView);

export function isRebindOutcome(value: unknown): value is RebindOutcome {
  if (!isRecord(value)) {
    return false;
  }
  const checks: Readonly<Record<string, () => boolean>> = {
    accepted: () => isString(value.label),
    needs_modifier: () => isString(value.message),
    duplicate_of: () => isShortcutKind(value.kind),
    in_use_by_another_app: () => true,
  };
  return isString(value.outcome) && (checks[value.outcome]?.() ?? false);
}
