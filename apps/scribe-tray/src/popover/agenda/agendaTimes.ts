import type { AgendaBlock, HiddenMeeting } from "../types";

const MS_PER_MINUTE = 60_000;
const NOON = 12;
const CLOCK_START = 11;
const HOURS_END = 13;
const MINUTES_START = 14;
const MINUTES_END = 16;

function twelveHour(hours: number): number {
  return hours % NOON === 0 ? NOON : hours % NOON;
}

function meridiem(hours: number): string {
  return hours < NOON ? "AM" : "PM";
}

export function hourLabel(hours: number): string {
  return `${twelveHour(hours)} ${meridiem(hours)}`;
}

export function clockLabel(iso: string): string {
  const hours = Number(iso.slice(CLOCK_START, HOURS_END));
  const minutes = iso.slice(MINUTES_START, MINUTES_END);
  return `${twelveHour(hours)}:${minutes} ${meridiem(hours)}`;
}

export function rangeLabel(meeting: Pick<HiddenMeeting, "starts_at" | "ends_at">): string {
  return `${clockLabel(meeting.starts_at)}–${clockLabel(meeting.ends_at)}`;
}

export function hasEnded(meeting: Pick<HiddenMeeting, "ends_at">, now: number): boolean {
  return Date.parse(meeting.ends_at) <= now;
}

export function minutesSince(dayStart: string, now: number): number {
  return Math.max(0, Math.floor((now - Date.parse(dayStart)) / MS_PER_MINUTE));
}

export function todayHeading(now: number): string {
  const day = new Date(now);
  const weekday = day.toLocaleDateString("en-US", { weekday: "short" });
  return `Today · ${weekday} ${day.getDate()}`;
}

export function blockName(block: AgendaBlock, ended: boolean): string {
  const cues: ReadonlyArray<readonly [boolean, string]> = [
    [block.is_teams, "Teams meeting"],
    [block.is_recurring, "recurring"],
    [ended, "ended"],
    [!ended && block.armed_state === "missed", "No call detected"],
  ];
  const shown = cues.filter(([applies]) => applies).map(([, cue]) => cue);
  return [`${block.subject}, ${rangeLabel(block)}`, ...shown].join(", ");
}
