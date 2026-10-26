import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { axe } from "vitest-axe";
import type { CallMeetingView, MeetingCandidate } from "../types";
import { MeetingLine } from "./MeetingLine";

const REVIEW: MeetingCandidate = {
  event_id: "review",
  subject: "DXC Review",
  starts_at: "2026-10-26T11:00:00+05:30",
  ends_at: "2026-10-26T11:45:00+05:30",
};
const STANDUP: MeetingCandidate = {
  event_id: "standup",
  subject: "Anthara Standup",
  starts_at: "2026-10-26T11:15:00+05:30",
  ends_at: "2026-10-26T11:30:00+05:30",
};

const MATCHED: CallMeetingView = {
  state: "matched",
  selected_event_id: "review",
  not_a_meeting: false,
  candidates: [REVIEW],
};
const AMBIGUOUS: CallMeetingView = {
  state: "ambiguous",
  selected_event_id: "review",
  not_a_meeting: false,
  candidates: [REVIEW, STANDUP],
};
const UNMATCHED: CallMeetingView = {
  state: "unmatched",
  selected_event_id: null,
  not_a_meeting: false,
  candidates: [],
};

afterEach(cleanup);

describe("the meeting line", () => {
  it("shows the matched meeting's subject and time range", () => {
    render(<MeetingLine view={MATCHED} onChoose={vi.fn()} />);

    expect(screen.getByRole("button", { name: /DXC Review · 11:00 AM–11:45 AM/ })).toBeTruthy();
  });

  it("asks which meeting it is when several fit and preselects the top candidate", () => {
    render(<MeetingLine view={AMBIGUOUS} onChoose={vi.fn()} />);

    expect(screen.getByRole("button", { name: /Which meeting is this\?/ })).toBeTruthy();
    expect(screen.getByRole("radio", { name: /DXC Review/ })).toBeChecked();
    expect(screen.getByRole("radio", { name: /Anthara Standup/ })).not.toBeChecked();
  });

  it("lists the candidates and a not-a-calendar-meeting option once a matched line is opened", async () => {
    render(<MeetingLine view={AMBIGUOUS} onChoose={vi.fn()} />);

    expect(screen.getAllByRole("radio").map((radio) => radio.getAttribute("value"))).toEqual([
      "review",
      "standup",
      "",
    ]);
    expect(screen.getByRole("radio", { name: "Not a calendar meeting" })).toBeTruthy();
  });

  it("keeps a matched line closed until it is opened", async () => {
    render(<MeetingLine view={MATCHED} onChoose={vi.fn()} />);

    expect(screen.queryAllByRole("radio")).toHaveLength(0);

    await userEvent.click(screen.getByRole("button", { name: /DXC Review/ }));

    expect(screen.getAllByRole("radio")).toHaveLength(2);
  });

  it("reports the chosen candidate's id", async () => {
    const onChoose = vi.fn();
    render(<MeetingLine view={AMBIGUOUS} onChoose={onChoose} />);

    await userEvent.click(screen.getByRole("radio", { name: /Anthara Standup/ }));

    expect(onChoose).toHaveBeenCalledWith("standup");
  });

  it("reports null when it is not a calendar meeting", async () => {
    const onChoose = vi.fn();
    render(<MeetingLine view={AMBIGUOUS} onChoose={onChoose} />);

    await userEvent.click(screen.getByRole("radio", { name: "Not a calendar meeting" }));

    expect(onChoose).toHaveBeenCalledWith(null);
  });

  it("says so when the meeting was set aside", () => {
    const setAside: CallMeetingView = { ...MATCHED, selected_event_id: null, not_a_meeting: true };
    render(<MeetingLine view={setAside} onChoose={vi.fn()} />);

    expect(screen.getByRole("button", { name: /Not a calendar meeting/ })).toBeTruthy();
  });

  it("renders nothing when no meeting matched", () => {
    const { container } = render(<MeetingLine view={UNMATCHED} onChoose={vi.fn()} />);

    expect(container.innerHTML).toBe("");
  });

  it("renders nothing before the match is known", () => {
    const { container } = render(<MeetingLine view={null} onChoose={vi.fn()} />);

    expect(container.innerHTML).toBe("");
  });

  it("is a labelled radio group that the arrow keys move through", async () => {
    const onChoose = vi.fn();
    render(<MeetingLine view={AMBIGUOUS} onChoose={onChoose} />);

    expect(screen.getByRole("group", { name: "Which meeting is this?" })).toBeTruthy();
    screen.getByRole("radio", { name: /DXC Review/ }).focus();
    await userEvent.keyboard("{ArrowDown}");

    expect(onChoose).toHaveBeenCalledWith("standup");
  });

  it("has no accessibility violations", async () => {
    const { container } = render(<MeetingLine view={AMBIGUOUS} onChoose={vi.fn()} />);

    expect((await axe(container)).violations).toEqual([]);
  });
});
