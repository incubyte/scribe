import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { axe } from "vitest-axe";
import { ModelDownloadPanel } from "./ModelDownloadPanel";
import type { ModelDownload } from "./types";

const downloading = (percent: number): ModelDownload => ({ state: "downloading", percent });
const failed: ModelDownload = { state: "failed", reason_kind: "network" };

describe("the speech model download panel", () => {
  it("shows nothing once the speech model is ready", () => {
    const { container } = render(<ModelDownloadPanel download={{ state: "ready" }} onRetry={vi.fn()} />);

    expect(container).toBeEmptyDOMElement();
  });

  it("shows how far the download has come", () => {
    render(<ModelDownloadPanel download={downloading(42)} onRetry={vi.fn()} />);

    const bar = screen.getByRole("progressbar", { name: "Speech model download" });

    expect(bar).toHaveAttribute("value", "42");
    expect(screen.getByText("42%")).toBeInTheDocument();
  });

  it("announces progress politely in steps of ten percent", () => {
    render(<ModelDownloadPanel download={downloading(47)} onRetry={vi.fn()} />);

    const announcement = screen.getByRole("status");

    expect(announcement).toHaveAttribute("aria-live", "polite");
    expect(announcement).toHaveTextContent("Downloading speech model, 40%");
  });

  it("announces that the download failed and offers Retry", () => {
    render(<ModelDownloadPanel download={failed} onRetry={vi.fn()} />);

    expect(screen.getByRole("status")).toHaveTextContent("Download failed");
    expect(screen.getByRole("button", { name: "Retry" })).toBeEnabled();
  });

  it("shows an install whose size is unknown without a percentage", () => {
    render(<ModelDownloadPanel download={{ state: "downloading", percent: null }} onRetry={vi.fn()} />);

    expect(screen.getByRole("progressbar", { name: "Speech model download" })).not.toHaveAttribute(
      "value",
    );
    expect(screen.getByRole("status")).toHaveTextContent("Preparing speech model");
  });

  it("keeps one status region while the download turns into a failure", () => {
    const { rerender } = render(<ModelDownloadPanel download={downloading(42)} onRetry={vi.fn()} />);
    const status = screen.getByRole("status");

    rerender(<ModelDownloadPanel download={failed} onRetry={vi.fn()} />);

    expect(screen.getByRole("status")).toBe(status);
    expect(status).toHaveTextContent("Download failed");
  });

  it("retries the download from the keyboard", async () => {
    const onRetry = vi.fn();
    render(<ModelDownloadPanel download={failed} onRetry={onRetry} />);

    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Retry" })).toHaveFocus();
    await userEvent.keyboard("{Enter}");

    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it("has no accessibility violations while downloading", async () => {
    const { container } = render(<ModelDownloadPanel download={downloading(42)} onRetry={vi.fn()} />);

    const { violations } = await axe(container);

    expect(violations).toEqual([]);
  });

  it("has no accessibility violations after a failure", async () => {
    const { container } = render(<ModelDownloadPanel download={failed} onRetry={vi.fn()} />);

    const { violations } = await axe(container);

    expect(violations).toEqual([]);
  });
});
