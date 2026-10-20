import { useCallback, useState } from "react";

interface Announcement {
  readonly message: string | null;
  readonly sequence: number;
}

export function useAnnouncement() {
  const [announcement, setAnnouncement] = useState<Announcement>({ message: null, sequence: 0 });

  const announce = useCallback(
    (message: string) =>
      setAnnouncement((previous) => ({ message, sequence: previous.sequence + 1 })),
    [],
  );

  return { announcement, announce };
}
