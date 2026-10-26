import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { axe } from "vitest-axe";
import type { OutlookConnection } from "../types";
import { OutlookRow } from "./OutlookRow";

const noop = () => undefined;

const SIGNED_OUT: OutlookConnection = {
  state: "disconnected",
  display_name: null,
  email: null,
  error: null,
};

const CONNECTED: OutlookConnection = {
  state: "connected",
  display_name: "Sapan Parikh",
  email: "sapan@incubyte.co",
  error: null,
};

const DECLINED: OutlookConnection = {
  ...SIGNED_OUT,
  error: "the sign-in was cancelled or consent was declined",
};

const CONNECTING: OutlookConnection = { ...SIGNED_OUT, state: "connecting" };

const RECONNECT_NEEDED: OutlookConnection = {
  ...SIGNED_OUT,
  state: "reconnect_needed",
};

function renderRow(
  connection: OutlookConnection | null,
  handlers: { onConnect?: () => void; onDisconnect?: () => void } = {},
) {
  return render(
    <OutlookRow
      connection={connection}
      onConnect={handlers.onConnect ?? noop}
      onDisconnect={handlers.onDisconnect ?? noop}
    />,
  );
}

describe("the Outlook calendar row in Settings", () => {
  it("offers to connect Outlook while signed out", () => {
    renderRow(SIGNED_OUT);

    expect(
      screen.getByRole("button", { name: "Connect Outlook" }),
    ).toBeInTheDocument();
    expect(screen.queryByText(/didn't connect/)).not.toBeInTheDocument();
  });

  it("starts the sign-in when Connect Outlook is pressed", async () => {
    const onConnect = vi.fn();
    renderRow(SIGNED_OUT, { onConnect });

    await userEvent.click(
      screen.getByRole("button", { name: "Connect Outlook" }),
    );

    expect(onConnect).toHaveBeenCalledOnce();
  });

  it("shows the connected account's name and email with a Disconnect button", () => {
    renderRow(CONNECTED);

    expect(
      screen.getByText("Connected as Sapan Parikh · sapan@incubyte.co"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Disconnect" }),
    ).toHaveAccessibleDescription("Connected as Sapan Parikh · sapan@incubyte.co");
    expect(
      screen.queryByRole("button", { name: "Connect Outlook" }),
    ).not.toBeInTheDocument();
  });

  it("disconnects when Disconnect is pressed", async () => {
    const onDisconnect = vi.fn();
    renderRow(CONNECTED, { onDisconnect });

    await userEvent.click(screen.getByRole("button", { name: "Disconnect" }));

    expect(onDisconnect).toHaveBeenCalledOnce();
  });

  it("explains in one line why Outlook didn't connect and offers to try again", () => {
    renderRow(DECLINED);

    expect(screen.getByRole("status")).toHaveTextContent(
      "Outlook didn't connect: the sign-in was cancelled or consent was declined",
    );
    expect(
      screen.getByRole("button", { name: "Connect Outlook" }),
    ).toBeInTheDocument();
  });

  it("keeps one status region mounted so a failed sign-in is read out", () => {
    const { rerender } = renderRow(CONNECTING);
    const region = screen.getByRole("status", { name: "Outlook status" });

    rerender(<OutlookRow connection={DECLINED} onConnect={noop} onDisconnect={noop} />);

    expect(screen.getByRole("status", { name: "Outlook status" })).toBe(region);
    expect(region).toHaveTextContent(
      "Outlook didn't connect: the sign-in was cancelled or consent was declined",
    );
  });

  it.each([
    ["signed out", SIGNED_OUT],
    ["connected", CONNECTED],
    ["while connecting", CONNECTING],
    ["when a reconnect is needed", RECONNECT_NEEDED],
  ])("has a status region %s", (_, connection) => {
    renderRow(connection);

    expect(screen.getByRole("status", { name: "Outlook status" })).toBeInTheDocument();
  });

  it("says it is connecting while the sign-in is under way", () => {
    renderRow(CONNECTING);

    expect(screen.getByText("Connecting to Outlook…")).toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("asks to reconnect Outlook when Microsoft refused a refresh", async () => {
    const onConnect = vi.fn();
    renderRow(RECONNECT_NEEDED, { onConnect });

    await userEvent.click(
      screen.getByRole("button", { name: "Reconnect Outlook" }),
    );

    expect(onConnect).toHaveBeenCalledOnce();
  });

  it("reaches Connect Outlook from the keyboard", async () => {
    const onConnect = vi.fn();
    renderRow(SIGNED_OUT, { onConnect });

    await userEvent.tab();
    await userEvent.keyboard("{Enter}");

    expect(onConnect).toHaveBeenCalledOnce();
  });

  it("shows no controls until the connection is known", () => {
    renderRow(null);

    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it.each([
    ["signed out", SIGNED_OUT],
    ["connected", CONNECTED],
    ["after a failed sign-in", DECLINED],
    ["while connecting", CONNECTING],
    ["when a reconnect is needed", RECONNECT_NEEDED],
  ])("has no accessibility violations %s", async (_, connection) => {
    const { container } = renderRow(connection);

    expect((await axe(container)).violations).toEqual([]);
  });
});
