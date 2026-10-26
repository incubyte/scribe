import { useCallback } from "react";
import { fetchSwitchPrompt, keepRecording, switchMeeting } from "../commands";
import { isNullable } from "../../shared/guards";
import { isSwitchPromptView } from "../guards";
import { useBackendValue } from "../useBackendValue";

const SWITCH_PROMPT_EVENT = "meeting://switch-prompt";
const isPromptOrNone = isNullable(isSwitchPromptView);

export function useSwitchPrompt() {
  const { value: prompt, setValue } = useBackendValue(fetchSwitchPrompt, SWITCH_PROMPT_EVENT, isPromptOrNone);
  const switchToNext = useCallback(() => {
    setValue(null);
    void switchMeeting();
  }, [setValue]);
  const keep = useCallback(() => {
    setValue(null);
    void keepRecording();
  }, [setValue]);
  return { prompt, switchToNext, keep };
}
