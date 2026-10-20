import { describe, expect, it } from "vitest";
import { expecting } from "../shared/guards";
import {
  isModelDownload,
  isPartialPayload,
  isRecorderSnapshot,
  isSessionSummaries,
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
});
