import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { connectOutlook, disconnectOutlook, fetchOutlookConnection } from "../commands";
import { isOutlookConnection } from "../guards";
import type { OutlookConnection } from "../types";

const CONNECTION_EVENT = "outlook://connection";

export function useOutlookConnection() {
  const [connection, setConnection] = useState<OutlookConnection | null>(null);
  const refresh = useCallback(
    () => void fetchOutlookConnection().then(setConnection, () => setConnection(null)),
    [],
  );

  useEffect(() => {
    refresh();
    window.addEventListener("focus", refresh);
    const unlisten = listen<unknown>(CONNECTION_EVENT, ({ payload }) => {
      if (isOutlookConnection(payload)) {
        setConnection(payload);
      }
    });
    return () => {
      window.removeEventListener("focus", refresh);
      void unlisten.then((stop) => stop());
    };
  }, [refresh]);

  const connect = useCallback(() => void connectOutlook().then(setConnection, refresh), [refresh]);
  const disconnect = useCallback(() => void disconnectOutlook().then(setConnection, refresh), [refresh]);

  return { connection, connect, disconnect };
}
