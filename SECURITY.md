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

## Security practices in this project

- The API key is never written to the database, a config file, or a log, and
  there is no IPC command that returns it.
- The renderer has no network access (`connect-src 'self'`), no filesystem
  access, and no shell access.
- Model output is rendered without raw HTML.
- No telemetry, analytics, or crash reporting.
- Dependencies are kept deliberately few; `cargo audit` and `npm audit` run
  in CI.
- Automatic updates are disabled and will remain so until signing keys and a
  published policy exist.
