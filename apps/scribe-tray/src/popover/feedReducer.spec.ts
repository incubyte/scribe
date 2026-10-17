import { describe, expect, it } from "vitest";
import { feedReducer, type FeedEvent, type FeedState, initialFeed } from "./feedReducer";

function replay(events: readonly FeedEvent[], from: FeedState = initialFeed): FeedState {
  return events.reduce(feedReducer, from);
}

const recordingStarted: FeedEvent = { type: "started", at: 1_000 };

describe("the live transcript feed", () => {
  it("replaces a partial line with the final wording", () => {
    const feed = replay([
      recordingStarted,
      { type: "partial", speaker: "Me", text: "hello wor" },
      { type: "segment", speaker: "Me", startMs: 500, text: "hello world" },
    ]);

    expect(feed.lines).toEqual([{ speaker: "Me", startMs: 500, text: "hello world" }]);
    expect(feed.partials).toEqual({});
  });

  it("keeps the partial lines of the two speakers apart", () => {
    const feed = replay([
      recordingStarted,
      { type: "partial", speaker: "Me", text: "can you" },
      { type: "partial", speaker: "Them", text: "sure I" },
      { type: "segment", speaker: "Me", startMs: 0, text: "can you hear me" },
    ]);

    expect(feed.partials).toEqual({ Them: "sure I" });
  });

  it("keeps only the newest partial wording of a speaker", () => {
    const feed = replay([
      recordingStarted,
      { type: "partial", speaker: "Them", text: "sure" },
      { type: "partial", speaker: "Them", text: "sure I can" },
    ]);

    expect(feed.partials).toEqual({ Them: "sure I can" });
  });

  it("clears the live lines and returns to idle when the recording stops", () => {
    const feed = replay([
      recordingStarted,
      { type: "segment", speaker: "Them", startMs: 0, text: "hi" },
      { type: "partial", speaker: "Me", text: "hel" },
      { type: "stopped" },
    ]);

    expect(feed).toEqual(initialFeed);
  });

  it("starts a new recording with an empty transcript and its start time", () => {
    const feed = replay(
      [recordingStarted],
      { ...initialFeed, lines: [{ speaker: "Me", startMs: 0, text: "left over" }] },
    );

    expect(feed).toMatchObject({ phase: "recording", startedAt: 1_000, lines: [] });
  });

  it("catches up from a snapshot taken while recording", () => {
    const feed = replay([
      {
        type: "snapshot",
        snapshot: {
          state: "recording",
          started_at: "2026-10-17T09:00:00+00:00",
          lines: [{ speaker: "Them", start_ms: 250, text: "morning" }],
        },
      },
    ]);

    expect(feed).toEqual({
      phase: "recording",
      startedAt: Date.parse("2026-10-17T09:00:00+00:00"),
      lines: [{ speaker: "Them", startMs: 250, text: "morning" }],
      partials: {},
    });
  });

  it("catches up from a snapshot taken while locked", () => {
    const feed = replay([
      { type: "snapshot", snapshot: { state: "locked", started_at: null, lines: [] } },
    ]);

    expect(feed.phase).toBe("locked");
  });
});
