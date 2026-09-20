# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed — no API billing anywhere

The app no longer has an API credential of any kind. It now talks to Claude
exclusively through the **Claude Code CLI**, running as you, under the login
you already have. Requests go via your existing Claude Code plan; there is no
separate bill, and nothing to paste or store.

- **Removed**: API key entry, system-keyring storage, the Anthropic HTTP
  client, `ant auth login` browser sign-in, base-URL override, max-tokens and
  temperature settings, and the cost-estimate/pricing feature — all of which
  existed only to configure or account for API billing.
- **Added**: `ClaudeCodeProvider`, driving `claude --print --output-format
  stream-json`. Each conversation maps to one Claude Code session
  (`--session-id`, then `--resume`), so multi-turn context is the CLI's.
- **Security**: the app now makes no network request at all — it opens a pipe
  to a local process and nothing else. Every built-in tool is disabled by
  name, so a reply cannot read files or run commands, and the CLI runs in an
  empty scratch directory unless a project names a working folder. Nothing
  reads Claude Code's credential files.

Requires Claude Code to be installed and signed in.

## [0.1.0] — 2026-09-20

First public release. Phase 1 is complete: this is usable as an everyday
Claude client on Linux.

### Added

**Conversation**
- Streaming replies over the Anthropic Messages API, rendered as they arrive
- GitHub-flavoured Markdown: headings, lists, tables, task lists, blockquotes
- Syntax highlighting for 48 languages, each grammar lazily loaded
- Per-block copy, word-wrap toggle and save-to-file
- Collapsible extended-thinking output
- Stop generation, retry, and continue an interrupted reply
- Branch a new conversation from any message

**Persistence**
- SQLite with WAL, foreign keys, and versioned transactional migrations
- Conversations, messages, attachments, projects and settings
- Session restore: the last conversation reopens with the window state
- Crash recovery: replies cut short by the process ending are marked
  *Interrupted* with their partial text preserved
- Soft delete with undo, plus configurable trash retention
- Online database backup and integrity check

**Search**
- Full-text search across titles, message bodies and attachment filenames
  via FTS5, with safe query escaping and prefix matching as you type

**Projects**
- Standing instructions applied to every conversation in the project
- Default model, working folder, colour
- Conversations survive deletion of their project

**Attachments**
- Drag and drop, clipboard paste, and the native file picker
- Text, source code, JSON, CSV, Markdown, images and PDFs
- Validated against the API's documented limits before sending
- Content-addressed and deduplicated on disk

**Interface**
- Light, dark and system themes, following the desktop
- Command palette, global search, and full keyboard control
- Adjustable font size, compact mode, reduced motion
- Eight settings sections with an explicit, editable pricing table for
  optional cost estimates

**Platform**
- API key stored in the Secret Service (GNOME Keyring / KWallet), with an
  honest memory-only fallback when none is available
- Desktop notifications for long replies while the window is unfocused
- XDG-compliant paths; database and attachments are owner-only
- `.deb` and AppImage bundles, `.desktop` entry and AppStream metadata

### Security

- No telemetry, analytics, crash reporting, or update pings
- The renderer has no network, filesystem or shell access; `connect-src` is
  `'self'`, so injected content cannot exfiltrate a conversation
- No IPC command can read the API key back
- Model output is rendered without raw HTML
- Automatic updates are wired but disabled pending signing keys

### Known limitations

- MCP is not yet active; the schema and settings section are in place
  (Phase 3)
- The system tray preference exists but is disabled (Phase 2)
- Requires WebKitGTK 4.1, so Ubuntu 22.04 and Debian 11 are not supported
- Window *position* is not restored under Wayland — a protocol limitation

[0.1.0]: https://github.com/openclaude/openclaude-desktop/releases/tag/v0.1.0
