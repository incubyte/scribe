import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

export function useBackendValue<T>(
  fetchValue: () => Promise<T>,
  eventName: string,
  isValue: (payload: unknown) => payload is T,
) {
  const [value, setValue] = useState<T | null>(null);
  const refresh = useCallback(() => void fetchValue().then(setValue, () => setValue(null)), [fetchValue]);
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
  }, [refresh, eventName, isValue]);
  return { value, setValue, refresh };
}
