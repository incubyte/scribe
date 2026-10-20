# Scribe

Scribe is a small tray app for macOS and Windows that transcribes your calls without joining them. You click record, and Scribe transcribes your microphone and the sound your computer plays, live and entirely on your computer. Each call is saved as a Markdown transcript, and any AI app that supports MCP, such as Claude Desktop, can list, read, and search those transcripts.

## What it does

- **Records any call.** Scribe captures your microphone as "Me" and the computer's audio output as "Them", so it works with Zoom, Teams, Google Meet in a browser, or anything else that plays sound. No bot joins the call.
- **Transcribes on your computer.** macOS uses Apple's SpeechAnalyzer (English). Windows uses NVIDIA Parakeet, which detects the spoken language automatically.
- **Shows the transcript live.** Click the tray icon to open a popover with the running transcript, a timer, and your recent calls.
- **Stops when the conversation goes quiet.** A recording stops automatically after 5 minutes without speech, and a notification tells you the transcript is saved.
- **Saves plain text.** Each call becomes one Markdown file, with a timestamp and speaker on every line. Files are grouped into month folders such as `2026-10/` inside your transcripts folder, which is `Documents/Scribe` by default.
- **Connects to AI apps.** The bundled `scribe-mcp` server gives AI apps five tools: `list_transcripts`, `get_transcript`, `search_transcripts`, `get_screenshot`, and `get_transcripts_folder`, which tells apps that can read local files where transcripts are saved. The tray can connect Scribe to Claude Desktop in one click.

## Privacy

Audio and transcripts never leave your computer. Audio is processed in memory and is never written to disk, and only the transcript text is saved. The only network traffic is a one-time download of the speech model on Windows (and Apple's speech model install on macOS).

You are responsible for telling the people on a call that you are transcribing it. Recording and transcription laws differ by country and state, and some require everyone's consent.

## Install

Download the latest build from the [Releases page](https://github.com/incubyte/scribe/releases). The repository is private, so you need access to `incubyte/scribe` and a GitHub login.

| Platform | File | Requirements |
|---|---|---|
| macOS | `Scribe_<version>_aarch64.dmg` | Apple Silicon, macOS 26 or later |
| Windows | `Scribe_<version>_x64-setup.exe` | 64-bit Windows 10 (2004) or later |

Builds are not signed with a developer certificate yet, so each platform asks you to confirm the first launch:

- **macOS:** open Scribe once, then go to System Settings → Privacy & Security and choose **Open Anyway**. Allow the **Microphone** and **System Audio Recording** permissions when Scribe asks for them.
- **Windows:** on the SmartScreen prompt, choose **More info → Run anyway**. The installer also installs the Microsoft Visual C++ runtime if your computer does not have it yet, and on first launch Scribe downloads its speech model (about 670 MB).

## Use

1. Click the Scribe icon in the menu bar or notification area to open the popover.
2. Click the round record button to start. The button turns red and shows a stop square while Scribe is recording.
3. Click it again to stop. The transcript is saved in this month's folder inside your transcripts folder.

To keep transcripts somewhere else, open **Settings** from the popover and choose **Change Transcripts Folder**. Scribe moves your existing transcripts into the new folder, and it never overwrites a file that is already there. The folder cannot be changed while a call is being recorded.

Right-click the tray icon for the menu, which has the same actions plus **Privacy** and **Quit Scribe**.

### Connect an AI app

- **Claude Desktop:** click **Connect to Claude Desktop** in the popover, then restart Claude Desktop. Scribe adds itself to Claude Desktop's config and keeps all your other MCP servers.
- **Other MCP apps:** click **Copy MCP config** and paste the snippet into the app's MCP settings.
- **Claude Code:** run `claude mcp add scribe -s user -- <path to scribe-mcp>`. The path is inside the app bundle, next to the Scribe executable.

Then ask things like "List my recent Scribe calls" or "Search my Scribe transcripts for budget".

## Development

Scribe is a Rust workspace with a Tauri 2 tray app and a React popover. The design follows a hexagonal (ports and adapters) style, which is described in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

| Path | Contents |
|---|---|
| `apps/scribe-tray` | Tauri tray app (`src-tauri`) and the React popover (`src`) |
| `apps/scribe-mcp` | Read-only MCP server over stdio |
| `crates/scribe-core` | Recording domain, transcript library, and the ports |
| `crates/audio-*` | Microphone, macOS Core Audio tap, and Windows WASAPI loopback capture |
| `crates/stt-*` | Apple SpeechAnalyzer and Parakeet speech-to-text adapters |
| `crates/store-markdown` | Markdown transcript files |
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
