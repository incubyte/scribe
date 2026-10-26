import { useCallback } from "react";
import { connectOutlook, disconnectOutlook, fetchOutlookConnection } from "../commands";
import { isOutlookConnection } from "../guards";
import { useBackendValue } from "../useBackendValue";

const CONNECTION_EVENT = "outlook://connection";

export function useOutlookConnection() {
  const { value: connection, setValue, refresh } = useBackendValue(
    fetchOutlookConnection,
    CONNECTION_EVENT,
    isOutlookConnection,
  );
  const connect = useCallback(() => void connectOutlook().then(setValue, refresh), [setValue, refresh]);
  const disconnect = useCallback(() => void disconnectOutlook().then(setValue, refresh), [setValue, refresh]);
  return { connection, connect, disconnect };
}
