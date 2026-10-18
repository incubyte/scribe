import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { axe } from "vitest-axe";
import { PopoverActions } from "./PopoverActions";

const noop = () => undefined;

describe("the popover's quick actions", () => {
  it("connects Scribe to Claude Desktop from the popover", async () => {
    const onConnectClaudeDesktop = vi.fn();
    render(
      <PopoverActions
        claudeStatus="not_connected"
        onConnectClaudeDesktop={onConnectClaudeDesktop}
        onCopyMcpConfig={noop}
        onOpenFolder={noop}
      />,
    );

    await userEvent.click(
      screen.getByRole("button", { name: "Connect to Claude Desktop" }),
    );

    expect(onConnectClaudeDesktop).toHaveBeenCalledOnce();
  });

  it("copies the MCP config for other AI apps", async () => {
    const onCopyMcpConfig = vi.fn();
    render(
      <PopoverActions
        claudeStatus="not_connected"
        onConnectClaudeDesktop={noop}
        onCopyMcpConfig={onCopyMcpConfig}
        onOpenFolder={noop}
      />,
    );

    await userEvent.click(
      screen.getByRole("button", { name: "Copy MCP config" }),
    );

    expect(onCopyMcpConfig).toHaveBeenCalledOnce();
  });

  it("opens the transcripts folder", async () => {
    const onOpenFolder = vi.fn();
    render(
      <PopoverActions
        claudeStatus="not_connected"
        onConnectClaudeDesktop={noop}
        onCopyMcpConfig={noop}
        onOpenFolder={onOpenFolder}
      />,
    );

    await userEvent.click(
      screen.getByRole("button", { name: "Open transcripts folder" }),
    );

    expect(onOpenFolder).toHaveBeenCalledOnce();
  });

  it("is reachable as a labelled group with no accessibility violations", async () => {
    const { container } = render(
      <PopoverActions
        claudeStatus="not_connected"
        onConnectClaudeDesktop={noop}
        onCopyMcpConfig={noop}
        onOpenFolder={noop}
      />,
    );

    expect(
      screen.getByRole("navigation", { name: "Scribe actions" }),
    ).toBeInTheDocument();
    expect((await axe(container)).violations).toEqual([]);
  });

  it("shows that Scribe is connected instead of offering to connect again", () => {
    render(
      <PopoverActions
        claudeStatus="connected"
        onConnectClaudeDesktop={noop}
        onCopyMcpConfig={noop}
        onOpenFolder={noop}
      />,
    );

    expect(screen.getByText("Connected to Claude Desktop")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Connect to Claude Desktop" }),
    ).not.toBeInTheDocument();
  });

  it("offers to update the connection when Claude Desktop points at an old Scribe", async () => {
    const onConnectClaudeDesktop = vi.fn();
    render(
      <PopoverActions
        claudeStatus="outdated"
        onConnectClaudeDesktop={onConnectClaudeDesktop}
        onCopyMcpConfig={noop}
        onOpenFolder={noop}
      />,
    );

    await userEvent.click(
      screen.getByRole("button", { name: "Update Claude Desktop connection" }),
    );

    expect(onConnectClaudeDesktop).toHaveBeenCalledOnce();
  });
});
