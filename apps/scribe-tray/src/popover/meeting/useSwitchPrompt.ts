import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { fetchSwitchPrompt, keepRecording, switchMeeting } from "../commands";
import { isNullable } from "../../shared/guards";
import { isSwitchPromptView } from "../guards";
import type { SwitchPromptView } from "../types";

const SWITCH_PROMPT_EVENT = "meeting://switch-prompt";
const isPromptOrNone = isNullable(isSwitchPromptView);

export function useSwitchPrompt() {
  const [prompt, setPrompt] = useState<SwitchPromptView | null>(null);
  const refresh = useCallback(
    () => void fetchSwitchPrompt().then(setPrompt, () => setPrompt(null)),
    [],
  );

  useEffect(() => {
    refresh();
    window.addEventListener("focus", refresh);
    const unlisten = listen<unknown>(SWITCH_PROMPT_EVENT, ({ payload }) => {
      if (isPromptOrNone(payload)) {
        setPrompt(payload);
      }
    });
    return () => {
      window.removeEventListener("focus", refresh);
      void unlisten.then((stop) => stop());
    };
  }, [refresh]);

  const switchToNext = useCallback(() => {
    setPrompt(null);
    void switchMeeting();
  }, []);
  const keep = useCallback(() => {
    setPrompt(null);
    void keepRecording();
  }, []);

  return { prompt, switchToNext, keep };
}
