# Write a meeting into the Scribe notebook

Your artifact tools may go by other names. You need to list your artifacts, read one, publish with capabilities, publish to an existing URL, and use an artifact's database (get, query, list, batch).

## When you cannot publish or write

If you have no artifact tools, or a publish or database call is refused for lack of support, give the notes as markdown, say the notebook needs a Claude that can publish artifacts, and stop. Check this first.

## Find the notebook

1. List your own artifacts and keep those titled exactly `Scribe notebook`, newest first.
2. For each, get the database document `notebook/meta`. A refusal saying the artifact does not declare the database capability means a legacy notebook: skip it. The first one that answers is the notebook.
3. If none answers, create the notebook. A refusal of any other kind is not an answer: stop, report it in one line, and never create a notebook after an unexplained refusal.

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
- A batch whose entries are pinned, which is any batch touching an existing document, is applied all-or-nothing, so a refused rewrite leaves nothing changed.
- A document-count or size cap means the notebook is full. Only the unpinned batch of a brand-new meeting can apply partially. If the result shows some writes applied, delete exactly the documents that batch created, then reply in one line that the notebook is full.

## Reply

List `meeting_index`, paging with the cursor until it ends, and count it. Reply `Added <title>. The notebook holds N meetings.` When you just created the notebook, add its link.

## Import old notebooks

Old notebooks keep their meetings inside the page. Use this when the user asks to import them.

1. Find the notebook as above, and create it if it does not exist.
2. List `meeting_index` ids, paging with the cursor until it ends. These are the ids already in the notebook.
3. Your own artifacts titled `Scribe notebook` whose `notebook/meta` was refused for lack of the database capability are the old notebooks. If there are none, reply in one line that there is nothing to import, and stop.
4. Read each old notebook's page. Its meetings are the JSON array inside `<script type="application/json" id="scribe-meetings">`.
5. Take each meeting in turn. Skip it when its `id` is already in the notebook or came earlier in this import, whichever notebook it came from. Otherwise write it with one batch, as in Write the meeting, with these mappings:
   - `meetings/<id>` gets `title`, `started_at`, `duration_seconds`, `type`, `summary`, `speakers` and `blocks` as they are, and `todo_ids` from its todos in their order.
   - `meeting_index/<id>` gets `title`, `started_at`, `duration_seconds` and `type`.
   - Each todo goes to `todos/<id>--<todo_id>` as the full todo record from Write the meeting: `meeting_id` the meeting's `id`, `meeting_title` its `title`, `meeting_started_at` its `started_at`, `todo_id` the old todo `id`, its `text`, `owner`, `due` and `actions`, and `done` false and `done_at` null. Ticks in old notebooks were never saved, so every imported todo starts open.
6. If a batch is refused for a document-count or size cap, stop importing. Keep every earlier batch, delete only the documents the refused batch applied, and say which meetings were added before the stop.
7. Reply `Added N meetings, skipped M already there.`

Then list each old notebook with its title, link and meeting count, and ask which ones to delete. Delete only the notebooks the user names in their reply. Never delete one on a vague answer, and never delete one when the import stopped early.

## Update the notebook page

Use this when the user asks to update the notebook page, for example after a plugin update. Never do it on a normal run.

1. Find the notebook as above. If none answers, tell the user there is no notebook to update, and stop.
2. Read its page, which the publish tool needs before it will republish.
3. Publish `notebook.html` from this folder, unmodified, to that same artifact URL with exactly the capabilities in Create the notebook. Never create a new artifact.
4. Reply `Updated the notebook page. Every meeting is kept.`
