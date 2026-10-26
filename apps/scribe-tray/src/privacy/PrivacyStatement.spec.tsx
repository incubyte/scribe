import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "vitest-axe";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PrivacyStatement } from "./PrivacyStatement";

afterEach(() => {
  clearMocks();
});

describe("the privacy statement window", () => {
  it("states what stays on this computer and what Outlook fetches, and offers a single button", () => {
    render(<PrivacyStatement />);

    expect(
      screen.getByRole("heading", { name: "Scribe runs entirely on this computer" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Speech is transcribed on-device.")).toBeInTheDocument();
    expect(screen.getByText("Audio is never saved.")).toBeInTheDocument();
    expect(
      screen.getByText("Audio and transcripts are never sent to any server."),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "If you connect Outlook, Scribe signs you in at login.microsoftonline.com and reads today's meetings (times, subjects and invitees) from graph.microsoft.com.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Transcripts are saved as text files in your transcripts folder, Documents/Scribe unless you change it in Settings.",
      ),
    ).toBeInTheDocument();
    expect(screen.getAllByRole("button")).toHaveLength(1);
  });

  it("starts with keyboard focus on the acknowledge button", () => {
    render(<PrivacyStatement />);

    expect(screen.getByRole("button", { name: "I understand" })).toHaveFocus();
  });

  it("reads the statements to screen-reader users as the description of the button", () => {
    render(<PrivacyStatement />);

    const button = screen.getByRole("button", { name: "I understand" });

    expect(button).toHaveAccessibleDescription(/Speech is transcribed on-device\./);
    expect(button).toHaveAccessibleDescription(
      /Transcripts are saved as text files in your transcripts folder, Documents\/Scribe unless you change it in Settings\./,
    );
  });

  it("acknowledges the statement when Enter is pressed", async () => {
    const handler = vi.fn();
    mockIPC(handler);
    render(<PrivacyStatement />);

    await userEvent.keyboard("{Enter}");

    await waitFor(() => {
      expect(handler).toHaveBeenCalledWith("acknowledge_privacy", {});
    });
  });

  it("has no accessibility violations", async () => {
    const { container } = render(<PrivacyStatement />);

    const { violations } = await axe(container);

    expect(violations).toEqual([]);
  });
});
