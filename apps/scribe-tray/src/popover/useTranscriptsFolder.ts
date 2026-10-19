import { useCallback, useEffect, useState } from "react";
import { changeTranscriptsFolder, fetchTranscriptsFolder } from "./commands";

export function useTranscriptsFolder() {
  const [folder, setFolder] = useState<string | null>(null);
  const refresh = useCallback(() => fetchTranscriptsFolder().then(setFolder, () => undefined), []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const change = useCallback(
    () => changeTranscriptsFolder().then(refresh, () => undefined),
    [refresh],
  );

  return { folder, change };
}
