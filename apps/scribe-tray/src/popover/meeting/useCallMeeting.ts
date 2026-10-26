import { useCallback } from "react";
import { chooseMeeting, fetchCallMeeting } from "../commands";
import { isCallMeetingView } from "../guards";
import { useBackendValue } from "../useBackendValue";

const MATCH_EVENT = "meeting://match";

export function useCallMeeting() {
  const { value: view, setValue, refresh } = useBackendValue(fetchCallMeeting, MATCH_EVENT, isCallMeetingView);
  const choose = useCallback(
    (eventId: string | null) => void chooseMeeting(eventId).then(setValue, refresh),
    [setValue, refresh],
  );
  return { view, choose };
}
