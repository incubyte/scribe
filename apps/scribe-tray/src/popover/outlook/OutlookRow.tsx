import { type ReactNode, useId } from "react";
import type { OutlookConnection, OutlookState } from "../types";

interface OutlookRowProps {
  readonly connection: OutlookConnection | null;
  readonly onConnect: () => void;
  readonly onDisconnect: () => void;
}

interface StateContentProps {
  readonly connection: OutlookConnection;
  readonly onConnect: () => void;
  readonly onDisconnect: () => void;
  readonly accountId: string;
}

interface ConnectButtonProps {
  readonly label: string;
  readonly onConnect: () => void;
}

function ConnectButton({ label, onConnect }: ConnectButtonProps) {
  return (
    <button className="actions__primary" type="button" onClick={onConnect}>
      {label}
    </button>
  );
}

function failureText({ error }: OutlookConnection): string | null {
  return error === null ? null : `Outlook didn't connect: ${error}`;
}

function statusText(connection: OutlookConnection): string | null {
  return connection.state === "connecting" ? "Connecting to Outlook…" : failureText(connection);
}

function accountText({ display_name, email }: OutlookConnection): string {
  return `Connected as ${display_name ?? ""} · ${email ?? ""}`;
}

const CONTENT_BY_STATE: Readonly<Record<OutlookState, (props: StateContentProps) => ReactNode>> = {
  disconnected: ({ onConnect }) => <ConnectButton label="Connect Outlook" onConnect={onConnect} />,
  connecting: () => null,
  connected: ({ connection, onDisconnect, accountId }) => (
    <>
      <p id={accountId} className="outlook__account">
        {accountText(connection)}
      </p>
      <button
        className="actions__link"
        type="button"
        aria-describedby={accountId}
        onClick={onDisconnect}
      >
        Disconnect
      </button>
    </>
  ),
  reconnect_needed: ({ onConnect }) => (
    <ConnectButton label="Reconnect Outlook" onConnect={onConnect} />
  ),
};

function StateContent(props: StateContentProps) {
  const Content = CONTENT_BY_STATE[props.connection.state];
  return <Content {...props} />;
}

export function OutlookRow({ connection, onConnect, onDisconnect }: OutlookRowProps) {
  const headingId = useId();
  const accountId = useId();

  return (
    <section className="outlook" aria-labelledby={headingId}>
      <h3 id={headingId} className="outlook__heading">
        Outlook calendar
      </h3>
      {connection === null ? null : (
        <StateContent
          connection={connection}
          onConnect={onConnect}
          onDisconnect={onDisconnect}
          accountId={accountId}
        />
      )}
      <p role="status" aria-label="Outlook status" className="settings__status">
        {connection === null ? null : statusText(connection)}
      </p>
    </section>
  );
}
