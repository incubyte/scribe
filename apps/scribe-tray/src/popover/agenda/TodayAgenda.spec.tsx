import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { axe } from "vitest-axe";
import type { AgendaBlock, AgendaChip, AgendaView } from "../types";
import { TodayAgenda } from "./TodayAgenda";

const NOON = Date.parse("2026-10-26T12:00:00+00:00");
const VIEWPORT_HEIGHT = 300;

const block = (overrides: Partial<AgendaBlock>): AgendaBlock => ({
  event_id: "standup",
  subject: "Standup",
  starts_at: "2026-10-26T09:00:00+00:00",
  ends_at: "2026-10-26T09:15:00+00:00",
  is_teams: true,
  is_recurring: true,
  top_minutes: 540,
  height_minutes: 15,
  column: 0,
  columns: 1,
  ...overrides,
});

const STANDUP = block({});
const REVIEW = block({
  event_id: "review",
  subject: "Design review",
  starts_at: "2026-10-26T14:00:00+00:00",
  ends_at: "2026-10-26T15:30:00+00:00",
  is_teams: false,
  is_recurring: false,
  top_minutes: 840,
  height_minutes: 90,
});

const readyWith = (
  blocks: readonly AgendaBlock[],
  overflow: readonly AgendaChip[] = [],
): AgendaView => ({
  status: "ready",
  stale: false,
  day_start: "2026-10-26T00:00:00+00:00",
  fetched_at: "2026-10-26T11:55:00+00:00",
  blocks,
  overflow,
});

const LOADING: AgendaView = { ...readyWith([]), status: "loading", day_start: null, fetched_at: null };
const FAILED: AgendaView = { ...LOADING, status: "failed" };

function backend(view: AgendaView) {
  const handler = vi.fn((command: string) => (command === "todays_agenda" ? view : undefined));
  mockIPC(handler, { shouldMockEvents: true });
  return handler;
}

async function renderAgenda(view: AgendaView) {
  const handler = backend(view);
  render(<TodayAgenda />);
  await waitFor(() => expect(handler).toHaveBeenCalledWith("todays_agenda", {}));
  return handler;
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(NOON);
});

afterEach(async () => {
  cleanup();
  await new Promise((resolve) => setTimeout(resolve));
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
});

describe("today's agenda in the popover", () => {
  it("is headed with today's weekday and day", async () => {
    await renderAgenda(readyWith([STANDUP]));

    expect(await screen.findByRole("heading", { name: "Today · Mon 26" })).toBeInTheDocument();
  });

  it("says there are no meetings when nothing qualifies", async () => {
    await renderAgenda(readyWith([]));

    expect(await screen.findByText("No meetings today")).toBeInTheDocument();
  });

  it("asks the app to refresh when it opens", async () => {
    const handler = await renderAgenda(readyWith([STANDUP]));

    expect(handler).toHaveBeenCalledWith("refresh_agenda", {});
  });

  it("shows the last fetched meetings with when they were fetched after a failed refresh", async () => {
    await renderAgenda({ ...readyWith([STANDUP]), stale: true });

    expect(await screen.findByText("Updated 11:55 AM")).toBeInTheDocument();
    expect(screen.getByRole("listitem", { name: /^Standup/ })).toBeInTheDocument();
  });

  it("does not claim the agenda is stale when the refresh worked", async () => {
    await renderAgenda(readyWith([STANDUP]));

    await screen.findByRole("listitem", { name: /^Standup/ });
    expect(screen.queryByText(/^Updated/)).not.toBeInTheDocument();
  });

  it("offers a retry when nothing has been fetched yet and the first fetch failed", async () => {
    const handler = await renderAgenda(FAILED);
    const retry = await screen.findByRole("button", { name: "Retry" });

    await userEvent.click(retry);

    expect(screen.getByText(/Couldn't load your calendar/)).toBeInTheDocument();
    expect(handler.mock.calls.filter(([command]) => command === "refresh_agenda")).toHaveLength(2);
  });

  it("waits quietly for the first fetch", async () => {
    await renderAgenda(LOADING);

    expect(await screen.findByText("Loading your calendar")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Retry" })).not.toBeInTheDocument();
  });

  it("names each block with its subject, times, Teams and recurring in time order", async () => {
    await renderAgenda(readyWith([REVIEW, STANDUP]));

    await screen.findAllByRole("listitem");
    expect(screen.getAllByRole("listitem").map((item) => item.getAttribute("aria-label"))).toEqual([
      "Standup, 9:00 AM–9:15 AM, Teams meeting, recurring, ended",
      "Design review, 2:00 PM–3:30 PM",
    ]);
  });

  it("marks a meeting that has ended in its name and not only by dimming", async () => {
    await renderAgenda(readyWith([STANDUP, REVIEW]));

    const ended = await screen.findByRole("listitem", { name: /^Standup.*ended$/ });
    expect(ended).toHaveClass("agenda__block--ended");
    expect(screen.getByRole("listitem", { name: /^Design review/ })).not.toHaveClass(
      "agenda__block--ended",
    );
  });

  it("lets keyboard focus reach every block", async () => {
    await renderAgenda(readyWith([STANDUP, REVIEW]));

    await userEvent.tab();
    expect(await screen.findByRole("listitem", { name: /^Standup/ })).toHaveFocus();
    await userEvent.tab();
    expect(screen.getByRole("listitem", { name: /^Design review/ })).toHaveFocus();
  });

  it("places a block on the hour scale and keeps the minimum height", async () => {
    await renderAgenda(readyWith([STANDUP]));

    const placed = await screen.findByRole("listitem", { name: /^Standup/ });
    expect(placed).toHaveStyle({ top: "540px", height: "15px" });
  });

  it("splits the width between overlapping meetings", async () => {
    const left = block({ event_id: "left", subject: "Left", columns: 2, column: 0 });
    const right = block({ event_id: "right", subject: "Right", columns: 2, column: 1 });
    await renderAgenda(readyWith([left, right]));

    expect(await screen.findByRole("listitem", { name: /^Left/ })).toHaveStyle({
      left: "0%",
      width: "50%",
    });
    expect(screen.getByRole("listitem", { name: /^Right/ })).toHaveStyle({
      left: "50%",
      width: "50%",
    });
  });

  it("lists the meetings a chip hides when it is clicked", async () => {
    const chip: AgendaChip = {
      top_minutes: 555,
      height_minutes: 30,
      hidden: [
        {
          event_id: "third",
          subject: "Third wheel",
          starts_at: "2026-10-26T09:15:00+00:00",
          ends_at: "2026-10-26T09:45:00+00:00",
        },
      ],
    };
    await renderAgenda(readyWith([STANDUP], [chip]));
    const button = await screen.findByRole("button", { name: "1 more meeting" });
    expect(screen.queryByText("Third wheel, 9:15 AM–9:45 AM")).not.toBeVisible();

    await userEvent.click(button);

    expect(screen.getByText("Third wheel, 9:15 AM–9:45 AM")).toBeVisible();
    expect(button).toHaveAttribute("aria-expanded", "true");
  });

  it("draws the now line at the current minute and scrolls it into the top third", async () => {
    vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(VIEWPORT_HEIGHT);
    backend(readyWith([STANDUP]));
    const { container } = render(<TodayAgenda />);

    await screen.findByRole("listitem", { name: /^Standup/ });

    expect(container.querySelector(".agenda__now")).toHaveStyle({ top: "720px" });
    expect(container.querySelector(".agenda__scroller")?.scrollTop).toBe(620);
  });

  it("replaces the agenda when the app announces a new one", async () => {
    await renderAgenda(readyWith([STANDUP]));
    await screen.findByRole("listitem", { name: /^Standup/ });

    await act(() => emit("agenda://updated", readyWith([REVIEW])));

    expect(await screen.findByRole("listitem", { name: /^Design review/ })).toBeInTheDocument();
    expect(screen.queryByRole("listitem", { name: /^Standup/ })).not.toBeInTheDocument();
  });

  it("has no accessibility violations", async () => {
    backend(readyWith([STANDUP, REVIEW]));
    const { container } = render(<TodayAgenda />);
    await screen.findByRole("listitem", { name: /^Standup/ });

    expect((await axe(container)).violations).toEqual([]);
  });
});
