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

interface KeyCaptureHandlers {
  readonly onCaptured: (kind: ShortcutKind, pressed: PressedKeys) => void;
  readonly onCancelled: () => void;
}

function useCaptureOfPressedKeys(
  listening: ShortcutKind | null,
  { onCaptured, onCancelled }: KeyCaptureHandlers,
) {
  useEffect(() => {
    if (listening === null) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      const pressed = pressedKeysFrom(event);
      if (pressed === undefined) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      if (event.code === "Escape") {
        onCancelled();
      } else {
        onCaptured(listening, pressed);
      }
    };
    document.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("blur", onCancelled);
    return () => {
      document.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("blur", onCancelled);
      void resumeScreenshotShortcuts();
    };
  }, [listening, onCaptured, onCancelled]);
}

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

  const rebind = useCallback(
    (kind: ShortcutKind, pressed: PressedKeys) => {
      setListening(null);
      void report(kind, rebindScreenshotShortcut(kind, pressed));
    },
    [report],
  );

  const cancelListening = useCallback(() => {
    setListening(null);
    announce(CANCELLED_MESSAGE);
  }, [announce]);

  useCaptureOfPressedKeys(listening, { onCaptured: rebind, onCancelled: cancelListening });

  const startListening = useCallback(
    (kind: ShortcutKind) =>
      pauseScreenshotShortcuts()
        .then(() => {
          setListening(kind);
          announce(LISTENING_MESSAGE);
        })
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
