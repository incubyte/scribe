# Write a meeting into the Scribe notebook

Your artifact tools may go by other names. You need to list your artifacts, read one, publish with capabilities, publish to an existing URL, and use an artifact's database (get, query, list, batch).

## When you cannot publish or write

If you have no artifact tools, or a publish or database call is refused for lack of support, give the notes as markdown, say the notebook needs a Claude that can publish artifacts, and stop. Check this first.

## Find the notebook

1. List your own artifacts and keep those titled exactly `Scribe notebook`, newest first.
2. For each, get the database document `notebook/meta`. A refusal saying the artifact does not declare the database capability means a legacy notebook: skip it. The first one that answers is the notebook.
3. If none answers, create the notebook.

Never pick a notebook by title alone, and never read the page itself to decide.

## Create the notebook

Publish `notebook.html` from this folder, unmodified, titled `Scribe notebook`, with exactly these capabilities:

```json
{"db": {"rules": [
  {"path": "notebook", "write": "admin"},
  {"path": "meetings", "write": "admin"},
  {"path": "meeting_index", "write": "admin"},
  {"path": "todos", "write": "interact"}]},
 "sample": {}, "user": {}}
```

Then set `notebook/meta` to `{"schema": 1, "created_at": <now, ISO 8601>}`. Keep the link to give in your reply.

## Write the meeting

First read `meetings/<id>`, `meeting_index/<id>`, and the todos whose `meeting_id` is `<id>`, noting the version of each document that exists.

Then send one batch:

- `set` `meetings/<id>` with the meeting body, pinned to its version when it exists.
- `set` `meeting_index/<id>` with `title`, `started_at`, `duration_seconds` and `type`, pinned when it exists.
- For each todo whose document `todos/<id>--<todo_id>` exists: `update` it with `meeting_id`, `meeting_title`, `meeting_started_at`, `todo_id`, `text`, `owner`, `due` and `actions`, pinned to its version. Never send `done` or `done_at`.
- For each new todo: `set` `todos/<id>--<todo_id>` with those fields plus `done` false and `done_at` null.
- For each existing todo of this meeting that is not in the new list: `delete` it, pinned.

A batch holds at most 50 entries. Never touch another meeting's records, and never delete a todo whose `meeting_id` is not `<id>`.

## When a write is refused

- A version conflict means the notebook changed since you read it. Re-read the same documents and send the batch once more. If it is refused again, reply in one line that the notebook changed while writing and to try again.
- A document-count or size cap means the notebook is full. If the result shows some writes applied, delete the documents this run created, then reply in one line that the notebook is full.

## Reply

List `meeting_index`, paging with the cursor until it ends, and count it. Reply `Added <title>. The notebook holds N meetings.` When you just created the notebook, add its link.
