# Instrumentation — tagged probes, compliance-safe logging, and guaranteed cleanup

Read this at Step 7 of `/anthara:debug-fix`, before adding a single log line.

## Contents

- The tag protocol
- Cleanup and proof
- What a probe may log
- What a probe may never log
- Logging shape instead of value
- Replacing waits with conditions
- Use the tools before adding a probe
- Never weaken a control to see inside

## The tag protocol

Every temporary line you add carries a **run tag**: `DEBUG-` plus four hex characters, chosen once per debugging session and reused for every probe in it.

```
[DEBUG-a4f2] dispatch.lookup accountId=<id> matchedRows=0
[DEBUG-a4f2] guard.resolve tenantPresent=true scopeCount=3
```

Rules:

- **One tag per session, not per probe.** The tag identifies the investigation, so a single search finds every trace of it.
- **The tag goes in the message text**, not in a variable name or a comment. Comments get stripped by review discipline; message text survives until you remove it.
- **Every added line is tagged.** An untagged probe is a permanent log line you have not decided to add on purpose.
- **The tag is announced.** State it when you begin instrumenting, so a human reading the working tree knows what to look for.

This exists because "remember to remove your logging" is not a mechanism. A tag is.

## Cleanup and proof

Instrumentation is removed **before** the fix is written, not after it is verified — otherwise you are verifying instrumented code, and the probe itself can be what makes the loop green.

```bash
grep -rn "DEBUG-a4f2" .
```

Remove every hit, then run the same search again and show that it returns nothing. The empty result is the proof, and it goes into the diagnosis record's verification section.

In a regulated codebase a leftover debug log is not untidiness — it is a potential disclosure incident and an audit finding against logging controls. Treat the zero-hit grep as a hard gate on the fix, equal in weight to the tests passing.

If a probe turns out to be genuinely valuable long-term, it does not "stay" — it graduates. Rewrite it as a structured, compliance-reviewed, permanently intended log or metric with no `DEBUG-` tag, and call that out as a deliberate product change in the hand-back.

## What a probe may log

Identifiers and metadata that carry no regulated content:

- Record identifiers — primary keys, UUIDs, correlation and request ids, tenant and org ids
- Operation and event names, state-machine states, enum values
- HTTP methods, status codes, route templates (`/patients/:id`, never the resolved path)
- Counts, lengths, durations, retry attempts, queue depths
- Presence and shape — `hasToken=true`, `bodyKeys=[id,status]`, `rows=0`, `typeof=undefined`
- Feature-flag and configuration *names*, and whether they are set — never their values when they may be secrets
- Code location — module, class, function

## What a probe may never log

Not even temporarily, not even locally, not even redacted-by-eye:

- **ePHI** — names, dates of birth, MRNs, addresses, phone numbers, diagnoses, clinical notes, appointment details, anything from a patient record
- **Payment data** — card numbers, CVV, expiry, cardholder name, payment tokens, bank details
- **Credentials and secrets** — passwords, API keys, bearer tokens, JWTs (including the payload), session cookies, signing secrets, connection strings
- **Personal data** — email addresses, government identifiers, IP addresses tied to an individual
- **Whole request or response bodies** — the one that looks harmless today carries a new field tomorrow
- **Whole query strings and full SQL with bound parameters** — the parameters are the regulated part
- **Raw third-party exception objects** — vendor SDKs routinely embed the failing payload in the message

"Local only" is not an exemption: local logs land in terminal scrollback, editor output panes, session transcripts, and screenshots pasted into tickets. "It is just for a minute" is not an exemption either — the minute is when it gets committed.

This is a hard constraint from the active compliance packs, not a style preference.

## Logging shape instead of value

Almost every question a debugger asks can be answered by shape. When the value is regulated, log its shape and move on:

| Instead of | Log |
|---|---|
| the patient object | its keys, and `typeof` each suspicious field |
| the token | `hasToken`, `tokenLength`, `expClaimPresent` |
| the request body | `bodyKeys`, `contentLength` |
| the row | the primary key and the count of rows returned |
| the email | the domain only, or a stable hash of it |
| the diagnosis code | whether it is present, and whether it parsed |

If a probe genuinely cannot be answered by shape, the answer is not a bigger log line — it is a `SELECT` in a controlled session, or a fixture built from synthetic data that reproduces the same shape.

## Replacing waits with conditions

When the bug is timing-dependent, the instinct is to add a sleep or widen a timeout. Both hide the bug and neither fixes it.

Poll for the condition instead:

- Wait until the row exists, the queue drains, the element is present, the status flips — with a hard ceiling on total wait and a clear failure message when the ceiling is hit.
- The failure message names what never became true. "Timed out after 5s waiting for dispatch status to leave `pending`" is a diagnosis; "timeout" is not.
- Keep the ceiling generous and the poll interval tight. The ceiling exists to fail the test, not to pace it.

A fixed sleep asserts that an operation takes less than N milliseconds — an assertion that is false on a loaded CI runner and unrelated to the behaviour under test. A condition poll asserts what you actually mean.

**Never widen a timeout as a fix.** If a wait had to grow, the ordering is wrong; find the missing await, the missing transaction boundary, or the unsynchronised write.

## Use the tools before adding a probe

Reach for a connected tool before reaching for a log line. Tools answer questions without touching the working tree, which means nothing to clean up and nothing to leak.

- **Browser MCP** — console messages, network requests with status and timing, the live DOM and accessibility tree. Load `browser-verification` for the protocol. A console exception is evidence in its own right.
- **Database MCP** — query the actual data rather than inferring it from application behaviour. **`SELECT` only.** Never `UPDATE`, `DELETE`, `DROP`, `ALTER`, or `TRUNCATE` while investigating, on any environment. If a write seems necessary to reproduce, build a fixture instead.
- **Language-server tools** — find callers, trace references, check the resolved type. Faster and more reliable than grepping for a symbol, and it answers "what actually calls this?" definitively.
- **Tracker and repository tools** — the introducing PR's description, its review comments, and the linked ticket often state the intent the code fails to express.
- **The shell** — running processes, listening ports, resolved environment variables, file permissions, container health, image tags. A surprising share of "impossible" bugs are a service that is not running or a port bound by something else.

## Never weaken a control to see inside

Debugging visibility is never a reason to disable a protection. Specifically forbidden as an investigation shortcut:

- Bypassing or commenting out an auth guard, middleware, or authorization check
- Disabling row-level security, or querying as a superuser to "see the real data"
- Turning off input validation, signature verification, or webhook secret checks
- Loosening CORS, TLS verification, or certificate pinning
- Widening a permission, role, or scope on any environment
- Pointing a local run at production data

If the control is genuinely what is blocking observation, that is itself the finding — and the correct move is a fixture, a synthetic tenant, or a test that runs inside the control. Say so and take that route. A control removed "temporarily" is the most commonly committed accident in the whole practice.
