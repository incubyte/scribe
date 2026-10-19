import { useCallback, useEffect, useState } from "react";
import { changeTranscriptsFolder, fetchTranscriptsFolder } from "./commands";
import {
  CHANGE_FAILURE_MESSAGE,
  describeFolderChange,
} from "./folderChangeMessage";

export function useTranscriptsFolder() {
  const [folder, setFolder] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const refresh = useCallback(
    () => fetchTranscriptsFolder().then(setFolder, () => undefined),
    [],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const change = useCallback(
    () =>
      changeTranscriptsFolder()
        .then((outcome) => setStatus(describeFolderChange(outcome)))
        .catch(() => setStatus(CHANGE_FAILURE_MESSAGE))
        .then(refresh),
    [refresh],
  );

  return { folder, status, change };
}
