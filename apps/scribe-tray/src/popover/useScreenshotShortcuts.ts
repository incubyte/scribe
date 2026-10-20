import { useCallback, useEffect, useState } from "react";
import {
  fetchScreenshotShortcuts,
  pauseScreenshotShortcuts,
  rebindScreenshotShortcut,
  resetScreenshotShortcut,
  resumeScreenshotShortcuts,
} from "./commands";
import {
  SHORTCUTS_UNAVAILABLE_MESSAGE,
  SHORTCUT_FAILURE_MESSAGE,
  describeRebind,
  pressedKeysFrom,
} from "./shortcutCapture";
import type { RebindOutcome, ShortcutKind, ShortcutView } from "./types";

export function useScreenshotShortcuts(announce: (message: string) => void) {
  const [shortcuts, setShortcuts] = useState<readonly ShortcutView[]>([]);
  const [listening, setListening] = useState<ShortcutKind | null>(null);

  const refresh = useCallback(
    () =>
      fetchScreenshotShortcuts().then(setShortcuts, () =>
        announce(SHORTCUTS_UNAVAILABLE_MESSAGE),
      ),
    [announce],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const report = useCallback(
    (kind: ShortcutKind, attempt: Promise<RebindOutcome>) =>
      attempt
        .then((outcome) => announce(describeRebind(outcome, kind)))
        .catch(() => announce(SHORTCUT_FAILURE_MESSAGE))
        .then(refresh),
    [announce, refresh],
  );

  useEffect(() => {
    if (listening === null) {
      return;
    }
    const stopListening = () => setListening(null);
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.code === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        stopListening();
        return;
      }
      const pressed = pressedKeysFrom(event);
      if (pressed === undefined) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      stopListening();
      void report(listening, rebindScreenshotShortcut(listening, pressed));
    };
    document.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("blur", stopListening);
    return () => {
      document.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("blur", stopListening);
      void resumeScreenshotShortcuts();
    };
  }, [listening, report]);

  const startListening = useCallback(
    (kind: ShortcutKind) =>
      pauseScreenshotShortcuts()
        .then(() => setListening(kind))
        .catch(() => {
          announce(SHORTCUT_FAILURE_MESSAGE);
          void resumeScreenshotShortcuts();
        }),
    [announce],
  );

  const reset = useCallback(
    (kind: ShortcutKind) => report(kind, resetScreenshotShortcut(kind)),
    [report],
  );

  return { shortcuts, listening, startListening, reset };
}
