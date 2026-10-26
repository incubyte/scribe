# Scribe roadmap

What is planned, in rough order. Each item becomes a spec in `docs/specs/` when work starts.

## Now

### Outlook calendar linking

Sign in with a work Outlook account, show today's meetings in the popover, save the meeting name and invitees in each transcript, and record armed meetings automatically.

- Spec: [009 Outlook calendar linking](specs/009-outlook-calendar-linking.md)
- Before building: test that Incubyte and client tenants allow users to consent to `Calendars.ReadBasic` (spec 009, §8.1).

## Later

### Sign in with Google and Google Calendar

The same experience as Outlook for people whose meetings live in Google Workspace: connect a Google account in Settings, see today's events, and link transcripts to meetings.

- **Reuse:** the calendar, credential-store and matching ports from spec 009 are provider-neutral, so Google is a second adapter, not a new design.
- **Sign-in:** Google's desktop OAuth flow (PKCE with a loopback redirect), read-only calendar scope.
- **Gotcha, consent:** calendar scopes are "sensitive" to Google. Until Google verifies the app, the consent screen warns users and the app is limited to 100 users.
- **Gotcha, call detection:** Google Meet runs in the browser, and Scribe detects only the Teams app holding the microphone. Without browser call detection, Google meetings can be shown and linked but not auto-started.
- **Open:** whether one Outlook and one Google account can be connected at the same time (spec 009 allows one account).
