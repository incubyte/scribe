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

type Guard<T> = (value: unknown) => value is T;

const PHASES: readonly string[] = ["locked", "idle", "recording"];
const SPEAKERS: readonly string[] = ["Me", "Them"];
const STATUSES: readonly string[] = ["recording", "complete", "incomplete"];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function hasFields(
  value: unknown,
  fields: Readonly<Record<string, Guard<unknown>>>,
): value is Record<string, unknown> {
  return isRecord(value) && Object.entries(fields).every(([name, guard]) => guard(value[name]));
}

export function isOneOf<T extends string>(allowed: readonly string[]): Guard<T> {
  return (value: unknown): value is T => typeof value === "string" && allowed.includes(value);
}

export function isString(value: unknown): value is string {
  return typeof value === "string";
}

export function isNumber(value: unknown): value is number {
  return typeof value === "number";
}

function isNullable<T>(guard: Guard<T>): Guard<T | null> {
  return (value: unknown): value is T | null => value === null || guard(value);
}

export function isListOf<T>(guard: Guard<T>): Guard<readonly T[]> {
  return (value: unknown): value is readonly T[] => Array.isArray(value) && value.every(guard);
}

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

export function expecting<T>(guard: Guard<T>, what: string): (value: unknown) => T {
  return (value) => {
    if (!guard(value)) {
      throw new TypeError(`Unexpected ${what} from Scribe`);
    }
    return value;
  };
}

const CLAUDE_DESKTOP_STATUSES: readonly string[] = ["not_installed", "not_connected", "outdated", "connected"];

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

const isShortcutKind = isOneOf<ShortcutKind>(["full_screen", "region"]);

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
    needs_modifier: () => true,
    duplicate_of: () => isShortcutKind(value.kind),
    in_use_by_another_app: () => true,
  };
  return isString(value.outcome) && (checks[value.outcome]?.() ?? false);
}
