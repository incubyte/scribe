import { invoke } from "@tauri-apps/api/core";
import type { RecorderSnapshot, SessionSummary } from "./types";

const RECENT_SESSION_COUNT = 5;

export const fetchSnapshot = () => invoke<RecorderSnapshot>("recorder_snapshot");

export const fetchRecentSessions = () =>
  invoke<readonly SessionSummary[]>("recent_sessions", { limit: RECENT_SESSION_COUNT });

export const startRecording = () => invoke("start_recording");

export const stopRecording = () => invoke("stop_recording");

export const openSession = (id: string) => invoke("open_session", { id });

export const hidePopover = () => invoke("hide_popover");
