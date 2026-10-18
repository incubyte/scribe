import type { RecorderPhase, RecorderSnapshot, Speaker } from "./types";

export interface LiveLine {
  readonly speaker: Speaker;
  readonly startMs: number;
  readonly text: string;
}

export type Partials = Readonly<Partial<Record<Speaker, string>>>;

export interface FeedState {
  readonly phase: RecorderPhase;
  readonly startedAt: number | null;
  readonly lines: readonly LiveLine[];
  readonly partials: Partials;
  readonly startFailure: string | null;
}

export type FeedEvent = (state: FeedState) => FeedState;

export const initialFeed: FeedState = {
  phase: "idle",
  startedAt: null,
  lines: [],
  partials: {},
  startFailure: null,
};

export const feedEvents = {
  snapshot:
    (snapshot: RecorderSnapshot): FeedEvent =>
    () => ({
      ...initialFeed,
      phase: snapshot.state,
      startedAt: snapshot.started_at === null ? null : Date.parse(snapshot.started_at),
      lines: snapshot.lines.map((line) => ({
        speaker: line.speaker,
        startMs: line.start_ms,
        text: line.text,
      })),
    }),
  started:
    (at: number): FeedEvent =>
    () => ({ ...initialFeed, phase: "recording", startedAt: at }),
  partial:
    (speaker: Speaker, text: string): FeedEvent =>
    (state) => ({ ...state, partials: { ...state.partials, [speaker]: text } }),
  segment:
    (line: LiveLine): FeedEvent =>
    (state) => ({
      ...state,
      lines: [...state.lines, line],
      partials: withoutSpeaker(state.partials, line.speaker),
    }),
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

function withoutSpeaker(partials: Partials, speaker: Speaker): Partials {
  return Object.fromEntries(Object.entries(partials).filter(([key]) => key !== speaker));
}
