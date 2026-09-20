# Contributing

Thanks for considering it. This is a small project with a clear shape, so
the fastest route to a merged change is to match the conventions already in
the codebase.

## Getting set up

```bash
git clone https://github.com/openclaude/openclaude-desktop
cd openclaude-desktop
./scripts/setup-deps.sh      # WebKitGTK toolchain (needs sudo)
npm install
npm run app:dev
```

Rust 1.77+ and Node 18.18+ (20+ recommended). See
[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for the layout and for the
gotchas — in particular, **do not use a bare `cargo build` for UI work**:
`dist/` is embedded at compile time, so the binary will keep serving a stale
UI.

## Before you open a pull request

```bash
npm run typecheck
npm run lint
npm test
cargo test  --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt   --manifest-path src-tauri/Cargo.toml
```

CI runs all of these. Clippy is `-D warnings`, so a warning is a failure.

## Things that will get a change sent back

Not to be difficult — these are the properties the project exists to have:

- **Anything that gives the app credentials of its own.** It has none, and
  that is the point: requests run through the Claude Code CLI under the
  login the user already gave it. There must be no IPC command that can
  read that login.
- **A test that spends money or makes a real network call.** Everything in
  `src-tauri/tests/` runs against an in-memory database and parses recorded
  CLI output. A suite that costs money is a suite people stop running.
- **Widening the tool lockdown.** Sessions run with `--tools ""` and
  `--permission-prompts none`; MCP tools are opt-in per tool. Enabling a
  tool by default is not a change that will be accepted.
- **Telemetry, analytics, or crash reporting.** Including opt-in.
- **Adding `rehype-raw`** or otherwise rendering raw HTML from model output.
- **Widening the Tauri capabilities** (`fs`, `shell`, `http`) without a
  discussion first.
- **Logging prompt or response content.** At any log level.
- **Editing a migration that has already shipped.** Add a new one.

## Conventions

**Rust.** `cargo fmt`. Repository functions take `&Connection` so they
compose inside a transaction. Commands stay thin: validate, delegate,
return. Every command returns `Result<T, AppError>`.

**TypeScript.** No `any` — the IPC boundary uses `unknown` plus narrowing.
All IPC goes through `src/services/api.ts`; components never call `invoke`
directly.

**Accessibility.** `IconButton` requires a `label`, so no icon-only control
can ship without an accessible name. New interactive elements need keyboard
operation and a visible focus ring.

**Errors.** Users see a sentence they can act on. Diagnostics (status,
category, `request-id`) go behind "Details". Never a stack trace, never a
raw upstream string as the whole message.

**Comments explain why, not what.** If something looks odd but is
load-bearing — the FTS escaping, the autoscroll gesture rule, the 400 ms
flush — say why, so the next person does not "simplify" it back into a bug.

## Tests

New behaviour needs a test. The suites are organised by concern:

| Area | Where |
| --- | --- |
| SSE decoding | `src-tauri/src/provider/sse.rs` |
| Provider behaviour, errors | `src-tauri/tests/streaming.rs` |
| Persistence, recovery | `src-tauri/tests/persistence.rs` |
| Request assembly, branching | `src-tauri/tests/requests.rs` |
| Search, projects, settings | `src-tauri/tests/search_and_projects.rs` |
| Frontend units | `src/**/*.test.ts(x)` |

Name tests after the behaviour they protect
(`a_generation_killed_with_the_process_is_recovered_as_interrupted`), not
after the function they call.

## Commits and pull requests

Conventional-ish prefixes are appreciated (`feat:`, `fix:`, `docs:`,
`refactor:`, `test:`, `chore:`) but not enforced. What matters:

- One logical change per pull request.
- Say *why* in the description, not just what.
- Include a screenshot for any visible change, in both themes if it touches
  colour.
- Note anything you could not test on your machine.

## Reporting bugs

Include your distribution and desktop, the version from
Settings → Advanced → About, and what you expected. If the window is blank,
try `WEBKIT_DISABLE_COMPOSITING_MODE=1` first and say whether it helped.

For security issues, **do not** open a public issue — see
[SECURITY.md](SECURITY.md).

## Scope

The roadmap is in [docs/ROADMAP.md](docs/ROADMAP.md), including things
deliberately *not* planned. If you want to build something large, open an
issue first so we can agree on the shape before you spend the time.
