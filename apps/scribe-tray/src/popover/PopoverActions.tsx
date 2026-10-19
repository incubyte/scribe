import type { Ref } from "react";
import type { ClaudeDesktopStatus } from "./types";

interface PopoverActionsProps {
  readonly claudeStatus: ClaudeDesktopStatus | null;
  readonly onConnectClaudeDesktop: () => void;
  readonly onCopyMcpConfig: () => void;
  readonly onOpenFolder: () => void;
  readonly onOpenSettings: () => void;
  readonly settingsButtonRef?: Ref<HTMLButtonElement>;
}

const CONNECT_LABEL = "Connect to Claude Desktop";
const UPDATE_LABEL = "Update Claude Desktop connection";

export function PopoverActions({
  claudeStatus,
  onConnectClaudeDesktop,
  onCopyMcpConfig,
  onOpenFolder,
  onOpenSettings,
  settingsButtonRef,
}: PopoverActionsProps) {
  return (
    <nav className="actions" aria-label="Scribe actions">
      <ClaudeDesktopAction
        status={claudeStatus}
        onConnect={onConnectClaudeDesktop}
      />
      <div className="actions__secondary">
        <button
          className="actions__link"
          type="button"
          onClick={onCopyMcpConfig}
        >
          Copy MCP config
        </button>
        <button className="actions__link" type="button" onClick={onOpenFolder}>
          Open transcripts folder
        </button>
        <button
          ref={settingsButtonRef}
          className="actions__icon"
          type="button"
          aria-label="Settings"
          onClick={onOpenSettings}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true" width="16" height="16">
            <circle cx="8" cy="8" r="2.25" fill="none" stroke="currentColor" strokeWidth="1.5" />
            <path
              d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </svg>
        </button>
      </div>
    </nav>
  );
}

interface ClaudeDesktopActionProps {
  readonly status: ClaudeDesktopStatus | null;
  readonly onConnect: () => void;
}

function ClaudeDesktopAction({ status, onConnect }: ClaudeDesktopActionProps) {
  if (status === "connected") {
    return (
      <p className="actions__connected">
        <svg className="actions__check" viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M3.5 8.5l3 3 6-7"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
          />
        </svg>
        Connected to Claude Desktop
      </p>
    );
  }

  return (
    <button className="actions__primary" type="button" onClick={onConnect}>
      {status === "outdated" ? UPDATE_LABEL : CONNECT_LABEL}
    </button>
  );
}
