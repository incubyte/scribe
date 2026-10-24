# Scribe

Scribe records your calls on your own computer. No bot joins the meeting, no audio leaves your machine, and every call becomes a plain-text transcript in a folder you choose.

When you want notes, ask Claude. `/meeting-notes` reads the transcript and writes up the decisions and todos, the way a colleague who was in the room would, into one notebook that collects every call.

Scribe is a small tray app for macOS and Windows that transcribes your calls without joining them. You click record, and Scribe transcribes your microphone and the sound your computer plays, live and entirely on your computer. Each call is saved as a Markdown transcript, and any AI app that supports MCP, such as Claude Desktop, can list, read, and search those transcripts.

## What it does

- **Records any call.** Scribe captures your microphone as "Me" and the computer's audio output as "Them", so it works with Zoom, Teams, Google Meet in a browser, or anything else that plays sound. No bot joins the call.
- **Transcribes on your computer.** macOS uses Apple's SpeechAnalyzer (English). Windows uses NVIDIA Parakeet, which detects the spoken language automatically.
- **Shows the transcript live.** Click the tray icon to open a popover with the running transcript, a timer, and your recent calls.
- **Stops when the conversation goes quiet.** A recording stops automatically after 5 minutes without speech, and a notification tells you the transcript is saved.
- **Saves plain text.** Each call becomes one Markdown file, with a timestamp and speaker on every line. Each call gets its own folder inside a month folder, such as `2026-10/2026-10-20-143005/`, in your transcripts folder, which is `Documents/Scribe` by default.
- **Takes screenshots.** One shortcut captures the whole screen and another lets you select a region or click a window, always on the monitor under your pointer. During a call, the screenshot is saved in that call's folder, named by its time into the call (`00-12-34.png`) so it lines up with the transcript. Every screenshot is also copied to the clipboard.
- **Connects to AI apps.** The bundled `scribe-mcp` server gives AI apps five tools: `list_transcripts`, `get_transcript`, `search_transcripts`, `get_screenshot`, and `get_transcripts_folder`, which tells apps that can read local files where transcripts are saved. A transcript lists the call's screenshots at the end, and `get_screenshot` returns any of them as an image scaled to 1568 px on the long edge. The tray can connect Scribe to Claude Desktop in one click.

## Privacy

Audio, transcripts, and screenshots never leave your computer through Scribe. Audio is processed in memory and is never written to disk, and only the transcript text is saved. Screenshots are saved only while a call is being recorded; at other times they go to the clipboard only. An AI app connected through MCP receives a scaled copy of a screenshot only when it asks for one. The only network traffic is a one-time download of the speech model on Windows (and Apple's speech model install on macOS), and an hourly check of GitHub for a newer Scribe release, which sends nothing about you or your calls.

You are responsible for telling the people on a call that you are transcribing it. Recording and transcription laws differ by country and state, and some require everyone's consent.

## Install

Download the latest build from the [Releases page](https://github.com/incubyte/scribe/releases).

| Platform | File | Requirements |
|---|---|---|
| macOS | `Scribe_<version>_aarch64.dmg` | Apple Silicon, macOS 26 or later |
| Windows | `Scribe_<version>_x64-setup.exe` | 64-bit Windows 10 (2004) or later |

The macOS build is signed and notarized by Apple. The Windows build is not signed yet, so Windows asks you to confirm the first launch:

- **macOS:** open Scribe. Allow the **Microphone** and **System Audio Recording** permissions when Scribe asks for them, and **Screen Recording** the first time you take a screenshot. macOS asks you to reconfirm Screen Recording about once a month.
- **Windows:** on the SmartScreen prompt, choose **More info → Run anyway**. The installer also installs the Microsoft Visual C++ runtime if your computer does not have it yet, and on first launch Scribe downloads its speech model (about 670 MB).

## Use

1. Click the Scribe icon in the menu bar or notification area to open the popover.
2. Click the round record button to start. The button turns red and shows a stop square while Scribe is recording.
3. Click it again to stop. The transcript is saved in this month's folder inside your transcripts folder.

To keep transcripts somewhere else, open **Settings** from the popover and choose **Change Transcripts Folder**. Scribe moves your existing transcripts into the new folder, and it never overwrites a file that is already there. The folder cannot be changed while a call is being recorded.

Right-click the tray icon for the menu, which has the same actions plus **Privacy** and **Quit Scribe**.

### Take a screenshot

| Shortcut | macOS | Windows |
|---|---|---|
| Whole screen under the pointer | `⌃⌥⌘1` | `Ctrl+Shift+Alt+1` |
| Region or window | `⌃⌥⌘2` | `Ctrl+Shift+Alt+2` |

The region shortcut freezes the screen under your pointer. Drag to select a region, or hover over a window and click it to capture just that window. Press Esc or right-click to cancel. The capture is of the screen as it was when you pressed the shortcut, so a window snap includes anything that was covering part of that window.

While a call is recording, the screenshot is saved in that call's folder and copied to the clipboard. When nothing is recording, it is only copied to the clipboard.

To change either shortcut, open **Settings** from the popover, choose **Change** next to the shortcut, and press the new keys. A shortcut needs ⌘, ⌃ or ⌥ (Ctrl or Alt on Windows) plus a letter, number or function key. **Reset to default** restores the shortcuts above.

### Connect an AI app

- **Claude Desktop:** click **Connect to Claude Desktop** in the popover, then restart Claude Desktop. Scribe adds itself to Claude Desktop's config and keeps all your other MCP servers.
- **Other MCP apps:** click **Copy MCP config** and paste the snippet into the app's MCP settings.
- **Claude Code:** run `claude mcp add scribe -s user -- <path to scribe-mcp>`. The path is inside the app bundle, next to the Scribe executable.

Then ask things like "List my recent Scribe calls" or "Search my Scribe transcripts for budget".

### Install the Claude plugin

The Scribe plugin adds two commands to Claude: `/recap` lists your recent calls with a summary of each, and `/meeting-notes` writes one call up as notes with todos, added to your one Scribe notebook from any chat (it needs a Claude that can publish artifacts). Connect Scribe to Claude Desktop first (see above).

- **Claude Code:** run `/plugin marketplace add incubyte/scribe`, then `/plugin install scribe@scribe`.
- **Claude Desktop:** open **Customize**, then **Plugins**, click **Add marketplace**, enter `incubyte/scribe`, and install **Scribe**.
- **If the marketplace install fails:** download `scribe-plugin.zip` from the latest [GitHub Release](https://github.com/incubyte/scribe/releases) and upload it under **Customize**, then **Plugins**.

See the [plugin README](plugins/scribe/README.md) for its requirements and commands.

## Development

Scribe is a Rust workspace with a Tauri 2 tray app and a React popover. The design follows a hexagonal (ports and adapters) style, which is described in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

| Path | Contents |
|---|---|
| `apps/scribe-tray` | Tauri tray app (`src-tauri`) and the React popover (`src`) |
| `apps/scribe-mcp` | Read-only MCP server over stdio |
| `crates/scribe-core` | Recording domain, transcript library, and the ports |
| `crates/audio-*` | Microphone, macOS Core Audio tap, and Windows WASAPI loopback capture |
| `crates/stt-*` | Apple SpeechAnalyzer and Parakeet speech-to-text adapters |
| `crates/capture-*` | Screen capture: ScreenCaptureKit on macOS, `xcap` on Windows |
| `crates/store-markdown` | Markdown transcript files and screenshots |
| `plugins/scribe` | Claude plugin with the `recap` and `meeting-notes` commands |
| `docs/` | Spec, plans, and architecture |

You need Rust 1.93, Node 22 or later, and on macOS the macOS 26 SDK (Xcode 26). The bundled MCP server must be built before the tray app, so run the staging script once before your first build:

```sh
apps/scribe-tray/scripts/stage-sidecar.sh
cargo test --workspace
cd apps/scribe-tray && npm ci && npm test
npm run tauri dev
```

[`CLAUDE.md`](CLAUDE.md) lists every lint, test, mutation, and release command.

## Releases

Releases are built on a developer's Mac with `scripts/release.sh`, which builds the macOS `.dmg`, cross-compiles the Windows installer, and uploads both to a draft GitHub Release. The reasons for this setup, and for the CI layout, are recorded as decision records in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md#9-decision-records).

## Status

Scribe is an early build. Open questions and known limitations are tracked in [`docs/specs/001-scribe-tray-transcriber.md`](docs/specs/001-scribe-tray-transcriber.md#9-open-questions). The main ones are signed builds, separating multiple remote speakers, and a first full test of the Windows installer on real hardware.
