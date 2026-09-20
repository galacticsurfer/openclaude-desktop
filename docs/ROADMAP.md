# Roadmap

## Phase 1 — a usable everyday client  ✅ shipped in 0.1.0

| Item | Status |
| --- | --- |
| Tauri 2 + React + TypeScript on Ubuntu | ✅ |
| Claude Code as the backend — no API key, no separate billing | ✅ |
| No credentials held at all; every tool disabled | ✅ |
| New conversations, streaming replies | ✅ |
| Markdown + syntax highlighting (48 languages, lazy) | ✅ |
| Conversation persistence in SQLite | ✅ |
| Automatic conversation titles | ✅ |
| Conversation history, rename, archive, trash | ✅ |
| Full-text search (FTS5, Ctrl+K) | ✅ |
| Model selector (Claude Code aliases, or any full model name) | ✅ |
| Slash commands and skills, discovered from the CLI | ✅ |
| Reasoning effort (`--effort`) | ✅ |
| Drag-and-drop, paste and picker attachments | ✅ |
| Light / dark / system themes | ✅ |
| Settings across 8 sections | ✅ |
| Restart and session restore, interrupted-reply recovery | ✅ |
| `.deb` and AppImage | ✅ |

Beyond the Phase 1 brief, these arrived early because the architecture made
them cheap: projects with instructions, conversation branching, export to
Markdown/JSON/text, desktop notifications, command palette, token usage and
optional cost estimates, soft-delete with undo, and database backup.

## Phase 2 — next

- **System tray** — opt-in, never required. The preference already exists and
  is disabled in the UI.
- **Global shortcut and quick chat** — a Spotlight-style popup
  (`Ctrl+Alt+C`), with "Continue in OpenClaude".
- **Deep links** — `openclaude://conversation/<id>`, needing
  `tauri-plugin-deep-link` and single-instance handling.
- **Conversation tabs** — several conversations open at once.
- **Split view** — two conversations side by side.
- **In-conversation find** — the `search_conversation` command exists already.
- **Message editing and regeneration from any point.**
- **Prompt library and templates.**
- **Remappable keyboard shortcuts.**

## Phase 3 — MCP  ✅ shipped

- stdio / SSE / HTTP server configuration, with tool discovery ✅
- per-tool permissions, enforced before a tool can run ✅
- tool calls rendered in the transcript, with their arguments, success or
  failure ✅
- per-project server sets ✅ — a server with no project is global; one bound
  to a project is invisible elsewhere, and approving a tool inside a project
  does not grant it outside
- a guided "add a folder" flow over the filesystem server — still to do

**How permission works, and what it is not.** Approval happens *before* a
tool can run, not while it runs. A server the user has not enabled is never
written into the config the CLI reads, so its tools do not exist for that
session; a tool with no explicit Allow is not passed in `--allowedTools`;
and `--permission-prompts none` means anything that would have asked is
denied rather than hanging on a prompt no GUI can answer.

That is not the just-in-time "allow once" prompt originally sketched below.
A prompt at call time needs the CLI to call back into a permission tool the
app would have to host, and the pre-approval model is the safer half of
that trade: nothing can run that the user has not already looked at.

### Original sketch, kept for the record

The database tables (`mcp_servers`, `mcp_permissions`) already ship in schema
v1, and Settings → MCP states plainly that the feature is not yet active.

- stdio / SSE / HTTP server configuration, with status and logs
- tool discovery and inline rendering of calls and results, collapsible
- **a permission system before any tool runs** — allow once / always for this
  project / deny, by category (filesystem read, filesystem write, shell,
  network, database)
- per-project server sets
- a guided "add a folder" flow over the filesystem MCP server, scoped to a
  chosen directory rather than `$HOME`

Prompt injection matters far more once tools exist, which is why approval is
a precondition rather than a setting.

## Phase 4 — exploration

- Artifacts in a split view; editable artifacts later
- Mermaid rendering (source / preview toggle)
- Multiple windows
- Screenshot-to-Claude via XDG portals
- Clipboard actions from the tray
- Nautilus / Dolphin "Ask Claude" context-menu integration
- Session profiles (coding, research, writing)
- Local model providers behind the existing `AIProvider` trait

### Coding sessions

Claude Code is already the backend, with every tool switched off because a
chat window should not touch your files. The next step is a **Coding
Session**: the same project, the same sidebar, but tools enabled and scoped to
its working folder, with an explicit per-tool approval flow.

That is the thing this client can do that a macOS-parity client would not —
and the reason the schema already carries `working_dir` on projects.

## Deliberately not planned

- **Scraping claude.ai, or reusing browser cookies.** The brief ruled it out
  and it is the right call: unsupported, fragile, and a good way to get an
  account flagged. Only documented APIs.
- **Telemetry**, including opt-in. Not worth the trust cost for a tool people
  point at private code.
- **Auto-update by default.** Wired but off until there are signing keys and
  a published policy.
