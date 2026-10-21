import type { FakeDatabase } from "./fakeClaudeRuntime";

type RecordBody = Record<string, unknown>;

export type MeetingBody = {
  title: string;
  started_at: string;
  duration_seconds: number;
  type: string;
  summary: string;
  speakers: RecordBody[];
  blocks: RecordBody[];
  todo_ids: string[];
};

export type MeetingSummaryBody = Pick<MeetingBody, "title" | "started_at" | "duration_seconds" | "type">;

export type TodoBody = {
  meeting_id: string;
  meeting_title: string;
  meeting_started_at: string;
  todo_id: string;
  text: string;
  owner: string;
  due: string;
  actions: { label: string; prompt: string }[];
  done: boolean;
  done_at: string | null;
};

export type StoredMeeting = { id: string; meeting: MeetingBody; todos?: TodoBody[] };

type PlaceRecord = (collection: string, id: string, body: RecordBody) => void;

export function aMeeting(overrides: Partial<MeetingBody> = {}): MeetingBody {
  return {
    title: "Weekly sync",
    started_at: "2026-10-21T10:00:00Z",
    duration_seconds: 1800,
    type: "Team sync",
    summary: "The team agreed the plan for the week.",
    speakers: [],
    blocks: [],
    todo_ids: [],
    ...overrides,
  };
}

export function aSummaryOf(meeting: MeetingBody): MeetingSummaryBody {
  return {
    title: meeting.title,
    started_at: meeting.started_at,
    duration_seconds: meeting.duration_seconds,
    type: meeting.type,
  };
}

export function aTodo(meetingId: string, overrides: Partial<TodoBody> = {}): TodoBody {
  return {
    meeting_id: meetingId,
    meeting_title: "Weekly sync",
    meeting_started_at: "2026-10-21T10:00:00Z",
    todo_id: "t1",
    text: "Send the recap",
    owner: "Me",
    due: "",
    actions: [],
    done: false,
    done_at: null,
    ...overrides,
  };
}

export function seedMeeting(database: FakeDatabase, stored: StoredMeeting): void {
  placeMeeting((collection, id, body) => database.seed(collection, id, body), stored);
}

export function writeMeeting(database: FakeDatabase, stored: StoredMeeting): void {
  placeMeeting((collection, id, body) => database.write(collection, id, body), stored);
}

function placeMeeting(place: PlaceRecord, { id, meeting, todos = [] }: StoredMeeting): void {
  place("meetings", id, meeting);
  place("meeting_index", id, aSummaryOf(meeting));
  todos.forEach((todo) => place("todos", id + "--" + todo.todo_id, todo));
}
