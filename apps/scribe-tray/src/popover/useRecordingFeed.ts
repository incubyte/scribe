import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { type ActionDispatch, useCallback, useEffect, useReducer, useState } from "react";
import { fetchRecentSessions, fetchSnapshot } from "./commands";
import { type FeedEvent, feedReducer, initialFeed, type FeedState } from "./feedReducer";
import type { SessionSummary, Speaker } from "./types";

interface PartialPayload {
  readonly speaker: Speaker;
  readonly text: string;
}

interface SegmentPayload extends PartialPayload {
  readonly start_ms: number;
}

export interface RecordingFeed {
  readonly feed: FeedState;
  readonly recent: readonly SessionSummary[] | null;
}

type Dispatch = ActionDispatch<[FeedEvent]>;

function subscribe(dispatch: Dispatch, onStopped: () => void): () => void {
  const subscriptions: Promise<UnlistenFn>[] = [
    listen("recording://started", () => dispatch({ type: "started", at: Date.now() })),
    listen<PartialPayload>("recording://partial", ({ payload }) =>
      dispatch({ type: "partial", speaker: payload.speaker, text: payload.text }),
    ),
    listen<SegmentPayload>("recording://segment", ({ payload }) =>
      dispatch({
        type: "segment",
        speaker: payload.speaker,
        startMs: payload.start_ms,
        text: payload.text,
      }),
    ),
    listen("recording://stopped", () => {
      dispatch({ type: "stopped" });
      onStopped();
    }),
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
    void fetchSnapshot().then((snapshot) => dispatch({ type: "snapshot", snapshot }));
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
