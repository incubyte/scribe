import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  changeTranscriptsFolder,
  connectClaudeDesktop,
  copyMcpConfig,
  fetchClaudeDesktopStatus,
  fetchModelDownload,
  fetchRecentSessions,
  fetchScreenshotShortcuts,
  fetchSnapshot,
  fetchTranscriptsFolder,
  hidePopover,
  openSession,
  openTranscriptsFolder,
  pauseScreenshotShortcuts,
  rebindScreenshotShortcut,
  resetScreenshotShortcut,
  resumeScreenshotShortcuts,
  retryModelDownload,
  startRecording,
  stopRecording,
} from "./commands";

afterEach(() => clearMocks());

const PRESSED = { code: "KeyR", control: true, alt: false, shift: false, meta: false };

function replyingTo(command: string, reply: unknown) {
  const handler = vi.fn((received: string) => (received === command ? reply : undefined));
  mockIPC(handler);
  return handler;
}

describe("the popover refuses replies from the app that it cannot understand", () => {
  it.each([
    ["recorder snapshot", fetchSnapshot, "recorder_snapshot"],
    ["recent sessions", fetchRecentSessions, "recent_sessions"],
    ["model download status", fetchModelDownload, "model_download_status"],
    ["Claude Desktop status", connectClaudeDesktop, "connect_claude_desktop"],
    ["Claude Desktop status", fetchClaudeDesktopStatus, "claude_desktop_status"],
    ["transcripts folder", fetchTranscriptsFolder, "transcripts_folder"],
    ["folder change", changeTranscriptsFolder, "change_transcripts_folder"],
    ["screenshot shortcuts", fetchScreenshotShortcuts, "screenshot_shortcuts"],
    ["shortcut change", () => rebindScreenshotShortcut("region", PRESSED), "rebind_screenshot_shortcut"],
    ["shortcut reset", () => resetScreenshotShortcut("region"), "reset_screenshot_shortcut"],
  ])("a bad %s", async (what, ask, command) => {
    replyingTo(command, 42);

    await expect(ask()).rejects.toThrow(`Unexpected ${what} from Scribe`);
  });
});

describe("the popover sends each request to the app with what it needs", () => {
  it("asks for the five most recent sessions", async () => {
    const handler = replyingTo("recent_sessions", []);

    await fetchRecentSessions();

    expect(handler).toHaveBeenCalledWith("recent_sessions", { limit: 5 });
  });

  it("names the shortcut and the pressed keys when changing a shortcut", async () => {
    const handler = replyingTo("rebind_screenshot_shortcut", { outcome: "in_use_by_another_app" });

    await rebindScreenshotShortcut("region", PRESSED);

    expect(handler).toHaveBeenCalledWith(
      "rebind_screenshot_shortcut",
      { kind: "region", pressed: PRESSED },
    );
  });

  it("names the shortcut when resetting it", async () => {
    const handler = replyingTo("reset_screenshot_shortcut", { outcome: "in_use_by_another_app" });

    await resetScreenshotShortcut("full_screen");

    expect(handler).toHaveBeenCalledWith("reset_screenshot_shortcut", { kind: "full_screen" });
  });

  it("names the session to open", async () => {
    const handler = replyingTo("open_session", null);

    await openSession("2026-10-17-120000");

    expect(handler).toHaveBeenCalledWith("open_session", { id: "2026-10-17-120000" });
  });

  it.each([
    [startRecording, "start_recording"],
    [stopRecording, "stop_recording"],
    [hidePopover, "hide_popover"],
    [retryModelDownload, "retry_model_download"],
    [copyMcpConfig, "copy_mcp_config"],
    [openTranscriptsFolder, "open_transcripts_folder"],
    [pauseScreenshotShortcuts, "pause_screenshot_shortcuts"],
    [resumeScreenshotShortcuts, "resume_screenshot_shortcuts"],
  ])("sends a request without details to the app", async (send, command) => {
    const handler = replyingTo(command, null);

    await send();

    expect(handler).toHaveBeenCalledWith(command, {});
  });
});
