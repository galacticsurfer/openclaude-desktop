# Security model

This application is expected to be used with private company code. The design
assumes the conversation content is sensitive and the API key is the crown
jewel.

## Trust boundaries

```text
  ┌─ Renderer (WebKitGTK) ────────── UNTRUSTED CONTENT ──┐
  │  Renders model output and file contents.             │
  │  No credentials. No network. No filesystem.          │
  └────────────────────────┬─────────────────────────────┘
                           │  Tauri IPC — the only channel
  ┌────────────────────────┴─────────────────────────────┐
  │  Rust core — TRUSTED                                 │
  │  Holds the key, makes every request, owns the DB.    │
  └──────────────────────────────────────────────────────┘
```

Everything rendered in the webview — model output, attached file contents,
search snippets — is treated as untrusted data.

## Authentication — there is none to hold

This app has no API key, no OAuth client, and no credential store. It talks to
Claude by running the `claude` binary in its documented headless mode
(`--print --output-format stream-json`), as the user, under the login they
already established with Claude Code.

That removes the largest class of risk outright: there is no secret to leak
into a log, an export, an error payload or a crash dump, because there is no
secret. It also means this app makes **no outbound network connection of any
kind** — the only thing it opens is a pipe to a local process.

**It does not read Claude Code's credentials.** Nothing in this codebase
touches `~/.claude`, and there is no embedded OAuth client id. How the CLI
authenticates stays entirely the CLI's concern.

`--bare` is deliberately never passed: that flag forces API-key authentication
and explicitly never reads the user's OAuth login, which is the opposite of
what this app wants.

### Tool access is off

A chat window is not a coding agent. Every built-in tool is disabled by name
(`Bash`, `Read`, `Write`, `Edit`, `Glob`, `Grep`, `WebFetch`, `WebSearch`, …)
rather than relying on `--restricted`, which only removes the command-running
tools and would leave file access intact. `--permission-mode dontAsk` is set
too: there is no TTY, so a permission prompt would hang forever.

The CLI runs in an empty directory under `~/.local/share/openclaude/sessions`
unless the conversation's project names a working folder — otherwise a stray
`CLAUDE.md` somewhere on disk could silently join the conversation.

The prompt is written to the process's stdin, never passed in `argv`: an
inlined attachment would risk the argument-length limit, and arguments are
visible to every other process on the machine.

## Content Security Policy

```
default-src 'self';  script-src 'self';  object-src 'none';
style-src 'self' 'unsafe-inline';  img-src 'self' data: blob:;
connect-src 'self' ipc: http://ipc.localhost;
frame-ancestors 'none';  form-action 'none';  base-uri 'self'
```

`connect-src` is the important line. Because the renderer never needs to reach
the network, it *cannot* — so even a successful prompt-injection has nowhere
to send a conversation. The asset protocol is disabled entirely; attachments
are served through a command as data URLs, so the renderer can only see blobs
it names by id.

`'unsafe-inline'` remains for styles only. It is required by the styling
approach and cannot execute script.

## Rendering untrusted content

- **No raw HTML.** `rehype-raw` is deliberately not a dependency, so model
  output cannot inject markup. There is a test asserting that a `<script>` and
  an `<img onerror=…>` in a reply render as text.
- **Search snippets are React nodes**, never `innerHTML`, so a match inside
  user content cannot become an element.
- **Links are intercepted.** Only `http`, `https` and `mailto` are handed to
  the system browser; anything else is refused with a message. A `file://`
  link in a transcript must never be launched on the user's behalf.
- **Shiki output** is the one place `dangerouslySetInnerHTML` is used. The
  input is already tokenised and escaped by the highlighter, which emits only
  `<pre>`, `<code>` and `<span>` with style attributes.

## Tauri capabilities

`src-tauri/capabilities/default.json` grants the minimum: dialogs, clipboard,
notifications, OS name/version, window controls. Notably absent:

- `fs:*` — the renderer has no ambient filesystem access. Every file read
  goes through `add_attachments`, which validates type, size and content.
- `shell:*` — no command execution of any kind.
- `http:*` — no outbound requests from the renderer.

`opener:allow-open-url` is scoped to http/https/mailto and
`reveal-item-in-dir` to `$HOME`.

## Input validation

- **Attachments** are validated for type, UTF-8 validity and size against
  Anthropic's documented per-request limits before anything is sent, so an
  oversized file fails instantly with a clear message rather than after a slow
  upload and an opaque 413. Directories are rejected with advice.
- **Filenames** are reduced to their final path segment, so `../../etc/passwd`
  becomes `passwd`. Control characters are stripped; leading dots removed.
- **Settings keys** are checked against a known list, so a compromised
  renderer cannot stuff arbitrary rows into the table.
- **Search input** is escaped into FTS5 literals (see [SCHEMA.md](SCHEMA.md)).
- **Paths for save/backup** must be absolute and come from a native dialog.

## Privacy

- **No telemetry, no analytics, no crash reporting, no update ping.** There is
  no code to disable, because there is none.
- Conversations never leave the machine except as request bodies to
  `api.anthropic.com`.
- Logs carry diagnostics only. No prompt or response text is passed to a
  logging macro anywhere in the crate — including at `OPENCLAUDE_DEV=1`, which
  adds timing and category detail, never content.
- The database and attachment blobs are 0600; directories 0700.
- Exports contain conversations and settings, never credentials.

## Known limitations

Stated plainly rather than implied:

- **An attacker with your user account can read your conversations.** The
  database is encrypted at rest only if your disk is. File permissions stop
  other local users, not you-as-you.
- **Anyone who can run `claude` as you can use your Claude account**, with or
  without this app. That is a property of having the CLI signed in, not
  something this app adds.
- **WebKitGTK is a system library.** Security fixes come from your
  distribution, not from this app. See [LINUX-RISKS.md](LINUX-RISKS.md).
- **Prompt injection is not solved.** Content in an attached file can attempt
  to influence the model. The CSP means it cannot exfiltrate anything from the
  renderer, but it can still mislead *you*. When MCP tools arrive in Phase 3
  this matters much more, which is why every tool call will require explicit
  approval.

## Reporting a vulnerability

See [SECURITY.md](../SECURITY.md) in the repository root.
