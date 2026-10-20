import { useCallback, useEffect, useState } from "react";
import {
  fetchScreenshotShortcuts,
  pauseScreenshotShortcuts,
  rebindScreenshotShortcut,
  resetScreenshotShortcut,
  resumeScreenshotShortcuts,
} from "./commands";
import {
  CANCELLED_MESSAGE,
  LISTENING_MESSAGE,
  SHORTCUTS_UNAVAILABLE_MESSAGE,
  SHORTCUT_FAILURE_MESSAGE,
  describeRebind,
  pressedKeysFrom,
} from "./shortcutCapture";
import type { PressedKeys, RebindOutcome, ShortcutKind, ShortcutView } from "./types";

interface ShortcutCaptureHandlers {
  readonly onListening: () => void;
  readonly onCaptured: (kind: ShortcutKind, pressed: PressedKeys) => void;
  readonly onCancelled: () => void;
  readonly onPauseFailed: () => void;
}

function useShortcutCapture({
  onListening,
  onCaptured,
  onCancelled,
  onPauseFailed,
}: ShortcutCaptureHandlers) {
  const [listening, setListening] = useState<ShortcutKind | null>(null);

  useEffect(() => {
    if (listening === null) {
      return;
    }
    const cancel = () => {
      setListening(null);
      onCancelled();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      const pressed = pressedKeysFrom(event);
      if (pressed === undefined) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      if (event.code === "Escape") {
        cancel();
        return;
      }
      setListening(null);
      onCaptured(listening, pressed);
    };
    document.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("blur", cancel);
    return () => {
      document.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("blur", cancel);
      void resumeScreenshotShortcuts();
    };
  }, [listening, onCaptured, onCancelled]);

  const startListening = useCallback(
    (kind: ShortcutKind) =>
      pauseScreenshotShortcuts()
        .then(() => {
          setListening(kind);
          onListening();
        })
        .catch(() => {
          onPauseFailed();
          void resumeScreenshotShortcuts();
        }),
    [onListening, onPauseFailed],
  );

  return { listening, startListening };
}

export function useScreenshotShortcuts(announce: (message: string) => void) {
  const [shortcuts, setShortcuts] = useState<readonly ShortcutView[]>([]);

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

  const rebind = useCallback(
    (kind: ShortcutKind, pressed: PressedKeys) =>
      void report(kind, rebindScreenshotShortcut(kind, pressed)),
    [report],
  );

  const announceListening = useCallback(() => announce(LISTENING_MESSAGE), [announce]);
  const announceCancelled = useCallback(() => announce(CANCELLED_MESSAGE), [announce]);
  const announcePauseFailed = useCallback(() => announce(SHORTCUT_FAILURE_MESSAGE), [announce]);

  const { listening, startListening } = useShortcutCapture({
    onListening: announceListening,
    onCaptured: rebind,
    onCancelled: announceCancelled,
    onPauseFailed: announcePauseFailed,
  });

  const reset = useCallback(
    (kind: ShortcutKind) => report(kind, resetScreenshotShortcut(kind)),
    [report],
  );

  return { shortcuts, listening, startListening, reset };
}
