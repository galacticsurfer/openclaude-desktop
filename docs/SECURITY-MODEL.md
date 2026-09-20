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

## Authentication

Two supported mechanisms, both authenticating to the Anthropic **API**:

| | Stored where | Header |
| --- | --- | --- |
| API key | System keyring | `x-api-key` |
| Browser sign-in | Anthropic CLI profile (`~/.config/anthropic`) | `Authorization: Bearer` + `anthropic-beta: oauth-2025-04-20` |

Modelled as a `Credential` enum rather than a bare string, so the provider
cannot send the wrong header pair — sending both an `x-api-key` and an
`Authorization` header is rejected by the API, and `/v1/messages` rejects an
OAuth token without the beta opt-in. `Credential` implements `Debug` by hand
to redact itself, so a struct containing one cannot leak the secret into a log
line or a panic message.

### What is deliberately not implemented

Claude Code's `/login` is a **first-party** OAuth flow: Anthropic registered
Claude Code as its own OAuth client, and subscription-backed usage is tied to
that client. A third-party application could only join it by embedding that
client id — impersonating a first-party application — or by reading
`~/.claude/.credentials.json`. Both are out of bounds, so neither is
implemented: nothing in this codebase reads `~/.claude`, and there is no
embedded OAuth client id.

The supported equivalent is the Anthropic CLI's own OAuth (`ant auth login`),
whose profile the official SDKs already share. OpenClaude asks the CLI for a
token (`ant auth print-credentials --access-token`) rather than parsing its
credential files, so token storage and refresh remain the CLI's concern. The
token is fetched per request: it is short-lived, the CLI owns refresh, and one
subprocess is negligible beside a completion. The CLI is invoked directly —
never through a shell — and a profile name is validated against
`[A-Za-z0-9._-]{1,64}` before it becomes an argument.

## The API key

Rules, enforced structurally rather than by convention:

- **There is no command that returns the key.** The IPC surface offers
  `set_api_key`, `delete_api_key`, `test_api_key` and `credential_status`.
  `credential_status` returns a boolean, the backend in use, and the last four
  characters — enough to tell two keys apart, never enough to use one.
- Stored via the Secret Service API (GNOME Keyring, KWallet's bridge) using
  the `keyring` crate.
- **Never** in SQLite, never in a config file, never in an environment
  variable read by the app, never in an export.
- Never passed to a logging macro. Errors carry a structured `ErrorDetail`
  with category, HTTP status and Anthropic `request-id` — never headers or
  request bodies. `a_server_error_is_retryable_and_never_leaks_the_api_key`
  asserts the serialised error contains no `sk-ant`.

### When there is no keyring

Some sessions have no Secret Service (headless, minimal WM, broken
`gnome-keyring`). The app then falls back to **memory for the current run
only** and says so, in the sidebar badge and in Settings.

The alternative — writing an obfuscated key to disk — was rejected. It looks
like security, provides none against anyone with file access, and would
quietly break the promise on the welcome screen. Asking the user to paste the
key again is the honest failure mode.

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
- **The keyring is only as strong as your login keyring.** On most desktops
  it unlocks automatically at login.
- **WebKitGTK is a system library.** Security fixes come from your
  distribution, not from this app. See [LINUX-RISKS.md](LINUX-RISKS.md).
- **Prompt injection is not solved.** Content in an attached file can attempt
  to influence the model. The CSP means it cannot exfiltrate anything from the
  renderer, but it can still mislead *you*. When MCP tools arrive in Phase 3
  this matters much more, which is why every tool call will require explicit
  approval.

## Reporting a vulnerability

See [SECURITY.md](../SECURITY.md) in the repository root.
