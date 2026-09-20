# Security policy

## Reporting a vulnerability

**Please do not open a public issue for a security problem.**

Use GitHub's private vulnerability reporting
(*Security → Report a vulnerability* on the repository), which reaches the
maintainers directly.

Please include:

- what the issue is, and the class of problem
- how to reproduce it, minimally
- the version (`Settings → Advanced → About`) and your distribution
- what an attacker gains

You can expect an acknowledgement within **3 working days** and an initial
assessment within **10 days**. We will tell you when a fix is released and
credit you unless you prefer otherwise.

Please do not include a working exploit or an extraction path in the initial
report — a description of the class of problem is enough to act on.

## Supported versions

This project is pre-1.0. Only the latest release receives fixes.

| Version | Supported |
| --- | --- |
| 0.1.x | ✅ |
| < 0.1 | ❌ |

## What we consider a vulnerability

- Anything that exposes the API key outside the system keyring — a log line,
  an error payload, an export, an IPC response, a crash dump.
- Any way for rendered content (a model reply, an attached file, a search
  snippet) to execute script, escape the CSP, or reach the network.
- Any path traversal or arbitrary write through attachment handling, export,
  or backup.
- Privilege escalation through the packaging (`.deb` maintainer scripts,
  AppImage `AppRun`).
- Anything that causes conversation data to be transmitted anywhere other
  than `api.anthropic.com`.

## What is out of scope

These are documented properties, not bugs. They are covered in
[docs/SECURITY-MODEL.md](docs/SECURITY-MODEL.md):

- **Another process running as your user can read your conversations.** The
  database is protected by file permissions (0600), not encryption. If you
  need encryption at rest, encrypt your disk.
- **The keyring unlocks with your login session.** That is how desktop
  keyrings work.
- **A model can be induced to say something misleading.** Prompt injection
  against the *user* is not something this client can prevent; it does
  prevent injected content from reaching the network. This becomes far more
  significant with MCP tools in Phase 3, which is why every tool call will
  require explicit approval before it runs.
- **WebKitGTK vulnerabilities.** The WebView is a system library; fixes come
  from your distribution. Report those to your distribution or to WebKit.
- Findings from automated scanners without a demonstrated impact.

## The embedded terminal

The app ships a terminal (Ctrl+`). It runs your login shell, with your
environment and your permissions — the same authority you already have in
any terminal emulator.

**Claude cannot reach it.** This is structural, not a policy:

- No IPC command returns shell output to anything but the terminal widget.
- Nothing in the provider or chat path can reach a shell session.
- Model replies still run with `--tools ""`, so a reply cannot execute
  anything regardless of whether a terminal is open.

Getting something from the terminal into a conversation means selecting it
and copying it, deliberately. That boundary is the entire reason a built-in
shell is acceptable in an app that otherwise refuses to let a model touch
the machine: the authority here is yours, exercised by typing, and a model
deciding to run a command is a categorically different thing.

Two honest consequences:

- The shell inherits your environment, including any secrets in it. That is
  what a terminal is for, and scrubbing it would break normal use — but it
  means the app's "holds no credentials" claim describes the app, not what
  you choose to run inside it.
- Closing the panel kills the child process, and so does quitting. A hidden
  terminal is never left running.

## Security practices in this project

- The app holds no credentials at all. There is no API key to leak: every
  request runs through the Claude Code CLI under the login you already gave
  it, and no IPC command can read that login.
- The renderer has no network access (`connect-src 'self'`), no filesystem
  access, and no shell access.
- Model output is rendered without raw HTML.
- No telemetry, analytics, or crash reporting.
- Dependencies are kept deliberately few; `cargo audit` and `npm audit` run
  in CI.
- **A known `npm audit` finding, assessed and accepted.** Mermaid (diagram
  rendering) depends on `chevrotain` and `dagre-d3-es`, which depend on
  `lodash-es`. `npm audit` reports five high-severity advisories there,
  against `_.template` (code injection) and `_.unset` / `_.omit` (prototype
  pollution). No patched `lodash-es` exists as of mermaid 12.0.0.

  Those functions are not reachable here. The dependency chain imports
  lodash-es per function, and none of it imports `template`, `unset` or
  `omit`; the only overlap with the advisories is `merge`, which they do not
  cover. Tree-shaking then keeps the vulnerable code out of the build
  entirely — the shipped bundle contains neither `_.template`'s error
  strings nor its `sourceURL` handling. Re-check this if mermaid's
  dependencies change.

  Diagrams are additionally rendered with mermaid's `securityLevel: 'strict'`,
  and never while a reply is still streaming.
- Automatic updates are disabled and will remain so until signing keys and a
  published policy exist.
