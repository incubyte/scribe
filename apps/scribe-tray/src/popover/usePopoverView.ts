import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";

export const VIEWS = { main: "main", settings: "settings" } as const;
type View = (typeof VIEWS)[keyof typeof VIEWS];

const POPOVER_HIDDEN_EVENT = "popover://hidden";

function useReturnToMainWhenHidden(setView: (view: View) => void) {
  useEffect(() => {
    const subscription = listen(POPOVER_HIDDEN_EVENT, () => setView(VIEWS.main));
    return () => void subscription.then((stop) => stop());
  }, [setView]);
}

export function usePopoverView() {
  const [view, setView] = useState<View>(VIEWS.main);
  const settingsButton = useRef<HTMLButtonElement>(null);
  useReturnToMainWhenHidden(setView);

  const openSettings = () => setView(VIEWS.settings);

  const leaveSettings = () => {
    flushSync(() => setView(VIEWS.main));
    settingsButton.current?.focus();
  };

  return { view, openSettings, leaveSettings, settingsButton };
}
