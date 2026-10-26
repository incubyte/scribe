import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { axe } from "vitest-axe";
import type { SwitchPromptView } from "../types";
import { SwitchPromptCard } from "./SwitchPromptCard";

const PROMPT: SwitchPromptView = {
  from_subject: "DXC Review",
  to_subject: "Sprint Retro",
};

afterEach(cleanup);

describe("the switch prompt", () => {
  it("names the meeting still being recorded and the one to switch to", () => {
    render(<SwitchPromptCard prompt={PROMPT} onSwitch={vi.fn()} onKeep={vi.fn()} />);

    expect(
      screen.getByText("Still recording DXC Review. Switch to Sprint Retro?"),
    ).toBeInTheDocument();
  });

  it("switches when Switch is pressed", async () => {
    const onSwitch = vi.fn();
    render(<SwitchPromptCard prompt={PROMPT} onSwitch={onSwitch} onKeep={vi.fn()} />);

    await userEvent.click(screen.getByRole("button", { name: "Switch" }));

    expect(onSwitch).toHaveBeenCalledTimes(1);
  });

  it("keeps recording when Keep recording is pressed", async () => {
    const onKeep = vi.fn();
    render(<SwitchPromptCard prompt={PROMPT} onSwitch={vi.fn()} onKeep={onKeep} />);

    await userEvent.click(screen.getByRole("button", { name: "Keep recording" }));

    expect(onKeep).toHaveBeenCalledTimes(1);
  });

  it("moves focus to the prompt when it opens", () => {
    render(<SwitchPromptCard prompt={PROMPT} onSwitch={vi.fn()} onKeep={vi.fn()} />);

    expect(document.activeElement).toBe(
      screen.getByRole("heading", { name: "Switch meeting?" }),
    );
  });

  it("reaches both buttons by Tab and presses them with the keyboard", async () => {
    const onSwitch = vi.fn();
    const onKeep = vi.fn();
    render(<SwitchPromptCard prompt={PROMPT} onSwitch={onSwitch} onKeep={onKeep} />);

    await userEvent.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Switch" }));
    await userEvent.keyboard("{Enter}");
    await userEvent.tab();
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Keep recording" }),
    );
    await userEvent.keyboard(" ");

    expect(onSwitch).toHaveBeenCalledTimes(1);
    expect(onKeep).toHaveBeenCalledTimes(1);
  });

  it("has no accessibility violations", async () => {
    const { container } = render(
      <SwitchPromptCard prompt={PROMPT} onSwitch={vi.fn()} onKeep={vi.fn()} />,
    );

    expect((await axe(container)).violations).toEqual([]);
  });
});
