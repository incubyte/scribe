import { useEffect, useState } from "react";
import { fetchAppVersion } from "./commands";

export function useAppVersion() {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    fetchAppVersion().then(setVersion, () => setVersion(null));
  }, []);

  return version;
}
