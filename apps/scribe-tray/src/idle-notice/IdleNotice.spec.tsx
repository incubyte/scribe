import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "vitest-axe";
import { afterEach, describe, expect, it, vi } from "vitest";
import { IdleNotice } from "./IdleNotice";

afterEach(() => {
  clearMocks();
});

describe("the idle capture notice", () => {
  it("explains that screenshots are saved only during a recording", () => {
    render(<IdleNotice />);

    expect(screen.getByRole("heading", { name: "Copied to clipboard" })).toBeInTheDocument();
    expect(
      screen.getByText(/saves screenshots into a meeting's folder only while it's recording/),
    ).toBeInTheDocument();
  });

  it("starts with keyboard focus on OK, not the checkbox", () => {
    render(<IdleNotice />);

    expect(screen.getByRole("button", { name: "OK" })).toHaveFocus();
    expect(screen.getByRole("checkbox", { name: "Don't show this again" })).not.toBeChecked();
  });

  it("closes without opting out when OK is clicked unticked", async () => {
    const handler = vi.fn();
    mockIPC(handler);
    render(<IdleNotice />);

    await userEvent.click(screen.getByRole("button", { name: "OK" }));

    await waitFor(() => {
      expect(handler).toHaveBeenCalledWith("dismiss_idle_notice", { dontShowAgain: false });
    });
  });

  it("closes when Enter is pressed", async () => {
    const handler = vi.fn();
    mockIPC(handler);
    render(<IdleNotice />);

    await userEvent.keyboard("{Enter}");

    await waitFor(() => {
      expect(handler).toHaveBeenCalledWith("dismiss_idle_notice", { dontShowAgain: false });
    });
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it("closes when Escape is pressed", async () => {
    const handler = vi.fn();
    mockIPC(handler);
    render(<IdleNotice />);

    await userEvent.keyboard("{Escape}");

    await waitFor(() => {
      expect(handler).toHaveBeenCalledWith("dismiss_idle_notice", { dontShowAgain: false });
    });
  });

  it("reaches the checkbox with Tab and toggles it with Space", async () => {
    render(<IdleNotice />);
    const checkbox = screen.getByRole("checkbox", { name: "Don't show this again" });

    await userEvent.tab({ shift: true });
    await userEvent.keyboard(" ");

    expect(checkbox).toHaveFocus();
    expect(checkbox).toBeChecked();
  });

  it("opts out of future notices when closed with the checkbox ticked", async () => {
    const handler = vi.fn();
    mockIPC(handler);
    render(<IdleNotice />);

    await userEvent.click(screen.getByRole("checkbox", { name: "Don't show this again" }));
    await userEvent.click(screen.getByRole("button", { name: "OK" }));

    await waitFor(() => {
      expect(handler).toHaveBeenCalledWith("dismiss_idle_notice", { dontShowAgain: true });
    });
  });

  it("has no accessibility violations", async () => {
    const { container } = render(<IdleNotice />);

    const { violations } = await axe(container);

    expect(violations).toEqual([]);
  });
});
