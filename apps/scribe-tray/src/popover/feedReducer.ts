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
}

export type FeedEvent =
  | { readonly type: "snapshot"; readonly snapshot: RecorderSnapshot }
  | { readonly type: "started"; readonly at: number }
  | { readonly type: "partial"; readonly speaker: Speaker; readonly text: string }
  | {
      readonly type: "segment";
      readonly speaker: Speaker;
      readonly startMs: number;
      readonly text: string;
    }
  | { readonly type: "stopped" };

export const initialFeed: FeedState = {
  phase: "idle",
  startedAt: null,
  lines: [],
  partials: {},
};

type Handlers = {
  [Type in FeedEvent["type"]]: (
    state: FeedState,
    event: Extract<FeedEvent, { type: Type }>,
  ) => FeedState;
};

const HANDLERS: Handlers = {
  snapshot: (_state, { snapshot }) => ({
    phase: snapshot.state,
    startedAt: snapshot.started_at === null ? null : Date.parse(snapshot.started_at),
    lines: snapshot.lines.map((line) => ({
      speaker: line.speaker,
      startMs: line.start_ms,
      text: line.text,
    })),
    partials: {},
  }),
  started: (_state, { at }) => ({ ...initialFeed, phase: "recording", startedAt: at }),
  partial: (state, { speaker, text }) => ({
    ...state,
    partials: { ...state.partials, [speaker]: text },
  }),
  segment: (state, { speaker, startMs, text }) => ({
    ...state,
    lines: [...state.lines, { speaker, startMs, text }],
    partials: withoutSpeaker(state.partials, speaker),
  }),
  stopped: () => initialFeed,
};

export function feedReducer(state: FeedState, event: FeedEvent): FeedState {
  const handle = HANDLERS[event.type] as (state: FeedState, event: FeedEvent) => FeedState;
  return handle(state, event);
}

function withoutSpeaker(partials: Partials, speaker: Speaker): Partials {
  return Object.fromEntries(Object.entries(partials).filter(([key]) => key !== speaker));
}
