import { invoke } from "@tauri-apps/api/core";
import { expecting, isClaudeDesktopStatus, isModelDownload, isRecorderSnapshot, isSessionSummaries } from "./guards";

const RECENT_SESSION_COUNT = 5;

export const fetchSnapshot = () =>
  invoke<unknown>("recorder_snapshot").then(expecting(isRecorderSnapshot, "recorder snapshot"));

export const fetchRecentSessions = () =>
  invoke<unknown>("recent_sessions", { limit: RECENT_SESSION_COUNT }).then(
    expecting(isSessionSummaries, "recent sessions"),
  );

export const startRecording = () => invoke("start_recording");

export const stopRecording = () => invoke("stop_recording");

export const openSession = (id: string) => invoke("open_session", { id });

export const hidePopover = () => invoke("hide_popover");

export const fetchModelDownload = () =>
  invoke<unknown>("model_download_status").then(expecting(isModelDownload, "model download status"));

export const retryModelDownload = () => invoke("retry_model_download");

export const connectClaudeDesktop = () =>
  invoke<unknown>("connect_claude_desktop").then(expecting(isClaudeDesktopStatus, "Claude Desktop status"));

export const fetchClaudeDesktopStatus = () =>
  invoke<unknown>("claude_desktop_status").then(expecting(isClaudeDesktopStatus, "Claude Desktop status"));

export const copyMcpConfig = () => invoke("copy_mcp_config");

export const openTranscriptsFolder = () => invoke("open_transcripts_folder");
