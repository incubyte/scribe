import { describe, expect, it } from "vitest";
import { feedEvents, feedReducer, type FeedEvent, type FeedState, initialFeed } from "./feedReducer";

function replay(events: readonly FeedEvent[], from: FeedState = initialFeed): FeedState {
  return events.reduce(feedReducer, from);
}

const recordingStarted = feedEvents.started(1_000);

describe("the live recording feed", () => {
  it("captions the words being spoken before they are final", () => {
    const feed = replay([recordingStarted, feedEvents.partial("hello wor")]);

    expect(feed.caption).toBe("hello wor");
    expect(feed.latestLine).toBeNull();
  });

  it("swaps the caption for the final wording of a line", () => {
    const feed = replay([
      recordingStarted,
      feedEvents.partial("hello wor"),
      feedEvents.segment({ speaker: "Me", text: "hello world" }),
    ]);

    expect(feed.caption).toBe("hello world");
    expect(feed.latestLine).toEqual({ speaker: "Me", text: "hello world" });
  });

  it("captions whoever spoke most recently", () => {
    const feed = replay([
      recordingStarted,
      feedEvents.segment({ speaker: "Me", text: "can you hear me" }),
      feedEvents.partial("sure I"),
    ]);

    expect(feed.caption).toBe("sure I");
    expect(feed.latestLine).toEqual({ speaker: "Me", text: "can you hear me" });
  });

  it("clears the caption and returns to idle when the recording stops", () => {
    const feed = replay([
      recordingStarted,
      feedEvents.segment({ speaker: "Them", text: "hi" }),
      feedEvents.partial("hel"),
      feedEvents.stopped(),
    ]);

    expect(feed).toEqual(initialFeed);
  });

  it("starts a new recording with no caption and its start time", () => {
    const feed = replay([recordingStarted], {
      ...initialFeed,
      caption: "left over",
      latestLine: { speaker: "Me", text: "left over" },
    });

    expect(feed).toMatchObject({ phase: "recording", startedAt: 1_000, caption: null, latestLine: null });
  });

  it("catches up from a snapshot taken while recording", () => {
    const feed = replay([
      feedEvents.snapshot({
        state: "recording",
        started_at: "2026-10-17T09:00:00+00:00",
        lines: [
          { speaker: "Me", start_ms: 0, text: "hello" },
          { speaker: "Them", start_ms: 250, text: "morning" },
        ],
      }),
    ]);

    expect(feed).toEqual({
      phase: "recording",
      startedAt: Date.parse("2026-10-17T09:00:00+00:00"),
      caption: "morning",
      latestLine: { speaker: "Them", text: "morning" },
      startFailure: null,
    });
  });

  it("catches up from a snapshot taken while locked", () => {
    const feed = replay([feedEvents.snapshot({ state: "locked", started_at: null, lines: [] })]);

    expect(feed.phase).toBe("locked");
  });

  it("follows the recorder becoming ready to record", () => {
    const feed = replay([
      feedEvents.snapshot({ state: "locked", started_at: null, lines: [] }),
      feedEvents.phaseChanged("idle"),
    ]);

    expect(feed.phase).toBe("idle");
  });

  it("remembers why a recording could not start until the next one starts", () => {
    const failed = replay([feedEvents.startFailed("microphone_denied")]);
    const restarted = replay([recordingStarted], failed);

    expect(failed.startFailure).toBe("microphone_denied");
    expect(restarted.startFailure).toBeNull();
  });
});
