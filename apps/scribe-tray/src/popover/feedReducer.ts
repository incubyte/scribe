import type { RecorderPhase, RecorderSnapshot, Speaker } from "./types";

export interface LiveLine {
  readonly speaker: Speaker;
  readonly text: string;
}

export interface FeedState {
  readonly phase: RecorderPhase;
  readonly startedAt: number | null;
  readonly caption: string | null;
  readonly latestLine: LiveLine | null;
  readonly startFailure: string | null;
}

export type FeedEvent = (state: FeedState) => FeedState;

export const initialFeed: FeedState = {
  phase: "idle",
  startedAt: null,
  caption: null,
  latestLine: null,
  startFailure: null,
};

export const feedEvents = {
  snapshot:
    (snapshot: RecorderSnapshot): FeedEvent =>
    () => ({
      ...initialFeed,
      ...heard(lastLineOf(snapshot)),
      phase: snapshot.state,
      startedAt: snapshot.started_at === null ? null : Date.parse(snapshot.started_at),
    }),
  started:
    (at: number): FeedEvent =>
    () => ({ ...initialFeed, phase: "recording", startedAt: at }),
  partial:
    (text: string): FeedEvent =>
    (state) => ({ ...state, caption: text }),
  segment:
    (line: LiveLine): FeedEvent =>
    (state) => ({ ...state, ...heard(line) }),
  stopped: (): FeedEvent => () => initialFeed,
  phaseChanged:
    (phase: RecorderPhase): FeedEvent =>
    (state) => ({ ...state, phase }),
  startFailed:
    (reasonKind: string): FeedEvent =>
    (state) => ({ ...state, startFailure: reasonKind }),
};

export function feedReducer(state: FeedState, event: FeedEvent): FeedState {
  return event(state);
}

function heard(line: LiveLine | null): Pick<FeedState, "caption" | "latestLine"> {
  return { caption: line?.text ?? null, latestLine: line };
}

function lastLineOf(snapshot: RecorderSnapshot): LiveLine | null {
  const last = snapshot.lines.at(-1);
  return last === undefined ? null : { speaker: last.speaker, text: last.text };
}
