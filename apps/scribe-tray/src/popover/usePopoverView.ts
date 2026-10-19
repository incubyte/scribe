import { useRef, useState } from "react";
import { flushSync } from "react-dom";

export const VIEWS = { main: "main", settings: "settings" } as const;
type View = (typeof VIEWS)[keyof typeof VIEWS];

export function usePopoverView() {
  const [view, setView] = useState<View>(VIEWS.main);
  const settingsButton = useRef<HTMLButtonElement>(null);

  const openSettings = () => setView(VIEWS.settings);

  const leaveSettings = () => {
    flushSync(() => setView(VIEWS.main));
    settingsButton.current?.focus();
  };

  return { view, openSettings, leaveSettings, settingsButton };
}
