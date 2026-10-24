import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { axe } from "vitest-axe";
import { CHANGE_LABEL, SettingsPanel } from "./SettingsPanel";

const noop = () => undefined;
const FOLDER = "/Users/me/Documents/Scribe";

function renderPanel(
  overrides: Partial<Parameters<typeof SettingsPanel>[0]> = {},
) {
  return render(
    <SettingsPanel
      folder={FOLDER}
      status={null}
      isRecording={false}
      version="0.5.1"
      onChangeFolder={noop}
      onBack={noop}
      {...overrides}
    />,
  );
}

describe("the settings panel", () => {
  it("shows the folder transcripts are saved in", () => {
    renderPanel();

    expect(screen.getByText(FOLDER, { exact: false })).toBeInTheDocument();
  });

  it("shows a loading placeholder instead of an empty folder while the folder loads", () => {
    renderPanel({ folder: null });

    expect(screen.getByText("Loading…")).toBeInTheDocument();
    expect(screen.queryByText(/Transcripts folder:/)).not.toBeInTheDocument();
  });

  it("shows the Scribe version", () => {
    renderPanel();

    expect(screen.getByText("Version 0.5.1")).toBeInTheDocument();
  });

  it("shows no version line until the version is known", () => {
    renderPanel({ version: null });

    expect(screen.queryByText(/Version/)).not.toBeInTheDocument();
  });

  it("announces the status message politely", () => {
    renderPanel({ status: "Moved 2 transcripts." });

    expect(screen.getByRole("status")).toHaveTextContent(
      "Moved 2 transcripts.",
    );
  });

  it("moves focus to the Settings heading when it opens", () => {
    renderPanel();

    expect(screen.getByRole("heading", { name: "Settings" })).toHaveFocus();
  });

  it("asks Scribe to change the folder", async () => {
    const onChangeFolder = vi.fn();
    renderPanel({ onChangeFolder });

    await userEvent.click(screen.getByRole("button", { name: CHANGE_LABEL }));

    expect(onChangeFolder).toHaveBeenCalledOnce();
  });

  it("does not let the folder change while recording, and says why", () => {
    renderPanel({ isRecording: true });

    const change = screen.getByRole("button", { name: CHANGE_LABEL });
    expect(change).toBeDisabled();
    expect(change).toHaveAccessibleDescription(
      "Stop recording to change the folder.",
    );
  });

  it("leaves settings when Back is pressed", async () => {
    const onBack = vi.fn();
    renderPanel({ onBack });

    await userEvent.click(screen.getByRole("button", { name: "Back" }));

    expect(onBack).toHaveBeenCalledOnce();
  });

  it("has no accessibility violations while idle", async () => {
    const { container } = renderPanel();

    expect((await axe(container)).violations).toEqual([]);
  });

  it("has no accessibility violations while recording", async () => {
    const { container } = renderPanel({ isRecording: true });

    expect((await axe(container)).violations).toEqual([]);
  });
});
