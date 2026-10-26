import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { expecting, isString } from "../shared/guards";
import {
  isClaudeDesktopStatus,
  isFolderChange,
  isModelDownload,
  isOutlookConnection,
  isRebindOutcome,
  isRecorderSnapshot,
  isSessionSummaries,
  isShortcutViewList,
} from "./guards";
import type { PressedKeys, ShortcutKind } from "./types";

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

export const fetchAppVersion = () => getVersion();

export const fetchTranscriptsFolder = () =>
  invoke<unknown>("transcripts_folder").then(expecting(isString, "transcripts folder"));

export const changeTranscriptsFolder = () =>
  invoke<unknown>("change_transcripts_folder").then(expecting(isFolderChange, "folder change"));

export const fetchScreenshotShortcuts = () =>
  invoke<unknown>("screenshot_shortcuts").then(expecting(isShortcutViewList, "screenshot shortcuts"));

export const pauseScreenshotShortcuts = () => invoke("pause_screenshot_shortcuts");

export const resumeScreenshotShortcuts = () => invoke("resume_screenshot_shortcuts");

export const rebindScreenshotShortcut = (kind: ShortcutKind, pressed: PressedKeys) =>
  invoke<unknown>("rebind_screenshot_shortcut", { kind, pressed }).then(
    expecting(isRebindOutcome, "shortcut change"),
  );

export const resetScreenshotShortcut = (kind: ShortcutKind) =>
  invoke<unknown>("reset_screenshot_shortcut", { kind }).then(
    expecting(isRebindOutcome, "shortcut reset"),
  );

export const fetchOutlookConnection = () =>
  invoke<unknown>("outlook_connection").then(expecting(isOutlookConnection, "Outlook connection"));

export const connectOutlook = () =>
  invoke<unknown>("connect_outlook").then(expecting(isOutlookConnection, "Outlook connection"));

export const disconnectOutlook = () =>
  invoke<unknown>("disconnect_outlook").then(expecting(isOutlookConnection, "Outlook connection"));
