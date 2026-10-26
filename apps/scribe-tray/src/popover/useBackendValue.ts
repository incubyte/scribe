import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

export function useBackendValue<T>(
  fetchValue: () => Promise<T>,
  eventName: string,
  isValue: (payload: unknown) => payload is T,
) {
  const [value, setFetchedValue] = useState<T | null>(null);
  const eventsApplied = useRef(0);
  const setValue = useCallback((latest: T | null) => {
    eventsApplied.current += 1;
    setFetchedValue(latest);
  }, []);
  const refresh = useCallback(() => {
    const appliedBeforeFetch = eventsApplied.current;
    const unlessOvertaken = (fetched: T | null) => {
      if (eventsApplied.current === appliedBeforeFetch) {
        setFetchedValue(fetched);
      }
    };
    void fetchValue().then(unlessOvertaken, () => unlessOvertaken(null));
  }, [fetchValue]);
  useEffect(() => {
    refresh();
    window.addEventListener("focus", refresh);
    const unlisten = listen<unknown>(eventName, ({ payload }) => {
      if (isValue(payload)) {
        setValue(payload);
      }
    });
    return () => {
      window.removeEventListener("focus", refresh);
      void unlisten.then((stop) => stop());
    };
  }, [refresh, setValue, eventName, isValue]);
  return { value, setValue, refresh };
}
