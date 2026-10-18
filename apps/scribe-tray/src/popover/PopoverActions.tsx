import type { ClaudeDesktopStatus } from "./types";

interface PopoverActionsProps {
  readonly claudeStatus: ClaudeDesktopStatus | null;
  readonly onConnectClaudeDesktop: () => void;
  readonly onCopyMcpConfig: () => void;
  readonly onOpenFolder: () => void;
}

const CONNECT_LABEL = "Connect to Claude Desktop";
const UPDATE_LABEL = "Update Claude Desktop connection";

export function PopoverActions({
  claudeStatus,
  onConnectClaudeDesktop,
  onCopyMcpConfig,
  onOpenFolder,
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
