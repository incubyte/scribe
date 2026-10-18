import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { fetchModelDownload, retryModelDownload } from "./commands";
import { isModelDownload } from "./guards";
import type { ModelDownload } from "./types";

const MODEL_DOWNLOAD_EVENTS = [
  "model-download://progressed",
  "model-download://completed",
  "model-download://failed",
];

export interface ModelDownloadFeed {
  readonly download: ModelDownload;
  readonly retry: () => void;
}

export function useModelDownload(): ModelDownloadFeed {
  const [download, setDownload] = useState<ModelDownload>({ state: "ready" });

  useEffect(() => {
    void fetchModelDownload().then(setDownload);
    const subscriptions = MODEL_DOWNLOAD_EVENTS.map((event) =>
      listen<unknown>(event, ({ payload }) => {
        if (isModelDownload(payload)) {
          setDownload(payload);
        }
      }),
    );
    return () => subscriptions.forEach((subscription) => void subscription.then((stop) => stop()));
  }, []);

  const retry = useCallback(() => {
    void retryModelDownload();
  }, []);

  return { download, retry };
}
