import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { type ActionDispatch, useCallback, useEffect, useReducer, useState } from "react";
import { fetchRecentSessions, fetchSnapshot } from "./commands";
import { type FeedEvent, feedEvents, feedReducer, initialFeed, type FeedState } from "./feedReducer";
import { isPartialPayload, isSegmentPayload, isStartFailedPayload, isStatePayload } from "./guards";
import type { SessionSummary } from "./types";

export interface RecordingFeed {
  readonly feed: FeedState;
  readonly recent: readonly SessionSummary[] | null;
}

type Dispatch = ActionDispatch<[FeedEvent]>;

function listenFor<T>(
  event: string,
  guard: (payload: unknown) => payload is T,
  handle: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen<unknown>(event, ({ payload }) => {
    if (guard(payload)) {
      handle(payload);
    }
  });
}

function subscribe(dispatch: Dispatch, onStopped: () => void): () => void {
  const subscriptions: Promise<UnlistenFn>[] = [
    listen("recording://started", () => dispatch(feedEvents.started(Date.now()))),
    listenFor("recording://partial", isPartialPayload, ({ speaker, text }) =>
      dispatch(feedEvents.partial(speaker, text)),
    ),
    listenFor("recording://segment", isSegmentPayload, ({ speaker, start_ms, text }) =>
      dispatch(feedEvents.segment({ speaker, startMs: start_ms, text })),
    ),
    listen("recording://stopped", () => {
      dispatch(feedEvents.stopped());
      onStopped();
    }),
    listenFor("recorder://state", isStatePayload, ({ state }) =>
      dispatch(feedEvents.phaseChanged(state)),
    ),
    listenFor("recording://start-failed", isStartFailedPayload, ({ reason_kind }) =>
      dispatch(feedEvents.startFailed(reason_kind)),
    ),
  ];
  return () => subscriptions.forEach((subscription) => void subscription.then((stop) => stop()));
}

export function useRecordingFeed(): RecordingFeed {
  const [feed, dispatch] = useReducer(feedReducer, initialFeed);
  const [recent, setRecent] = useState<readonly SessionSummary[] | null>(null);

  const refreshRecent = useCallback(() => {
    void fetchRecentSessions().then(setRecent);
  }, []);

  const catchUp = useCallback(() => {
    void fetchSnapshot().then((snapshot) => dispatch(feedEvents.snapshot(snapshot)));
    refreshRecent();
  }, [refreshRecent]);

  useEffect(() => {
    catchUp();
    window.addEventListener("focus", catchUp);
    const unsubscribe = subscribe(dispatch, refreshRecent);
    return () => {
      window.removeEventListener("focus", catchUp);
      unsubscribe();
    };
  }, [catchUp, refreshRecent]);

  return { feed, recent };
}
