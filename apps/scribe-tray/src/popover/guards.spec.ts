import { describe, expect, it } from "vitest";
import { expecting } from "../shared/guards";
import {
  isAgendaView,
  isFolderChange,
  isModelDownload,
  isPartialPayload,
  isRebindOutcome,
  isRecorderSnapshot,
  isSegmentPayload,
  isSessionSummaries,
  isShortcutViewList,
  isStartFailedPayload,
  isStatePayload,
} from "./guards";

describe("messages from the app are checked before the popover trusts them", () => {
  it("accepts a well-formed recorder snapshot", () => {
    const snapshot = {
      state: "recording",
      started_at: "2026-10-17T09:00:00+00:00",
      lines: [{ speaker: "Them", start_ms: 0, text: "hi" }],
    };

    expect(isRecorderSnapshot(snapshot)).toBe(true);
  });

  it("rejects a snapshot in a state the popover does not know", () => {
    expect(isRecorderSnapshot({ state: "paused", started_at: null, lines: [] })).toBe(false);
  });

  it("rejects a partial line from an unknown speaker", () => {
    expect(isPartialPayload({ speaker: "Someone", text: "hi" })).toBe(false);
  });

  it("rejects a recent-sessions list containing a session without a title", () => {
    const sessions = [{ id: "2026-10-17-120000", started_at: "x", duration_seconds: null, status: "complete" }];

    expect(isSessionSummaries(sessions)).toBe(false);
  });

  it("accepts an install in progress whose size is unknown and rejects an unknown download state", () => {
    expect(isModelDownload({ state: "downloading", percent: null })).toBe(true);
    expect(isModelDownload({ state: "paused" })).toBe(false);
  });

  it("refuses an unexpected reply instead of passing it on", () => {
    const expectSnapshot = expecting(isRecorderSnapshot, "recorder snapshot");

    expect(() => expectSnapshot(null)).toThrow("Unexpected recorder snapshot from Scribe");
  });

  it("rejects a finished line that has no start time", () => {
    expect(isSegmentPayload({ speaker: "Me", text: "hi" })).toBe(false);
    expect(isSegmentPayload({ speaker: "Me", text: "hi", start_ms: 0 })).toBe(true);
  });

  it("rejects a state change to a phase the popover does not know", () => {
    expect(isStatePayload({ state: "recording" })).toBe(true);
    expect(isStatePayload({})).toBe(false);
  });

  it("rejects a start failure that gives no reason", () => {
    expect(isStartFailedPayload({ reason_kind: "model_missing" })).toBe(true);
    expect(isStartFailedPayload({})).toBe(false);
  });

  it("checks the details that belong to each model download state", () => {
    expect(isModelDownload({ state: "ready" })).toBe(true);
    expect(isModelDownload({ state: "failed", reason_kind: "network" })).toBe(true);
    expect(isModelDownload({ state: "failed" })).toBe(false);
    expect(isModelDownload({ state: "downloading", percent: "half" })).toBe(false);
    expect(isModelDownload("ready")).toBe(false);
    expect(isModelDownload({})).toBe(false);
  });

  it("checks the details that belong to each folder change outcome", () => {
    expect(isFolderChange({ outcome: "cancelled" })).toBe(true);
    expect(isFolderChange({ outcome: "unchanged" })).toBe(true);
    expect(isFolderChange({ outcome: "recording" })).toBe(true);
    expect(isFolderChange({ outcome: "changed", folder: "/a", moved: 2, kept: 0 })).toBe(true);
    expect(isFolderChange({ outcome: "changed" })).toBe(false);
    expect(isFolderChange({ outcome: "teleported" })).toBe(false);
    expect(isFolderChange({})).toBe(false);
    expect(isFolderChange("changed")).toBe(false);
  });

  it("accepts a list of screenshot shortcuts and rejects one with an unknown kind or no label", () => {
    expect(isShortcutViewList([{ kind: "region", label: "⌃R" }])).toBe(true);
    expect(isShortcutViewList([{ kind: "window", label: "⌃R" }])).toBe(false);
    expect(isShortcutViewList([{ kind: "region" }])).toBe(false);
  });

  it("checks the details that belong to each shortcut change outcome", () => {
    expect(isRebindOutcome({ outcome: "accepted", label: "⌃R" })).toBe(true);
    expect(isRebindOutcome({ outcome: "accepted" })).toBe(false);
    expect(isRebindOutcome({ outcome: "needs_modifier", message: "Use Ctrl" })).toBe(true);
    expect(isRebindOutcome({ outcome: "needs_modifier" })).toBe(false);
    expect(isRebindOutcome({ outcome: "duplicate_of", kind: "region" })).toBe(true);
    expect(isRebindOutcome({ outcome: "duplicate_of", kind: "window" })).toBe(false);
    expect(isRebindOutcome({ outcome: "in_use_by_another_app" })).toBe(true);
    expect(isRebindOutcome({ outcome: "teleported" })).toBe(false);
    expect(isRebindOutcome({})).toBe(false);
    expect(isRebindOutcome("accepted")).toBe(false);
  });

  it("accepts an agenda view and rejects one with an unknown status or a malformed block", () => {
    const view = {
      status: "ready",
      stale: false,
      day_start: "2026-10-26T00:00:00+00:00",
      fetched_at: null,
      blocks: [],
      overflow: [],
    };
    const block = {
      event_id: "a",
      subject: "Standup",
      starts_at: "2026-10-26T09:00:00+00:00",
      ends_at: "2026-10-26T09:15:00+00:00",
      is_teams: true,
      is_recurring: false,
      top_minutes: 540,
      height_minutes: 15,
      column: 0,
      columns: 1,
    };

    expect(isAgendaView(view)).toBe(true);
    expect(isAgendaView({ ...view, blocks: [block] })).toBe(true);
    expect(isAgendaView({ ...view, status: "stuck" })).toBe(false);
    expect(isAgendaView({ ...view, blocks: [{ ...block, columns: "2" }] })).toBe(false);
    expect(isAgendaView({ ...view, overflow: [{ top_minutes: 1, height_minutes: 1 }] })).toBe(false);
  });
});
