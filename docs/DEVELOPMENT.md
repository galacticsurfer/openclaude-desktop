# Development

## Prerequisites

- Rust 1.77+ (`rustup`)
- Node 18.18+ (20+ recommended)
- [Claude Code](https://code.claude.com/docs), installed and signed in — the
  app's only backend
- The WebKitGTK toolchain: `./scripts/setup-deps.sh`

## Running

```bash
npm install
npm run app:dev        # or ./scripts/dev.sh, which also sets OPENCLAUDE_DEV=1
```

`tauri dev` serves the UI from Vite, so frontend edits hot-reload and Rust
edits trigger a rebuild.

> **Do not use a bare `cargo build` for UI work.** With the `custom-protocol`
> feature, `dist/` is embedded at compile time, so the binary keeps serving
> whatever was built last. If you build that way, run `npm run build` *and*
> `cargo build`.

## Layout

```text
src/                     React renderer
  components/            UI, grouped by area
  stores/                Zustand: settings, conversations, UI
  services/              api.ts (every IPC call) and ipc.ts (errors)
  hooks/                 theme, hotkeys, stream events, autoscroll
  lib/                   formatting, highlighter, markdown helpers
src-tauri/               Rust core
  src/commands/          IPC surface — thin
  src/db/repo/           SQL, one module per aggregate
  src/provider/          AIProvider trait, Anthropic client, SSE decoder
  src/chat.rs            orchestration, streaming, branching, titles
  migrations/            versioned SQL
  tests/                 integration tests (no Tauri app needed)
docs/                    architecture and design notes
scripts/                 setup, dev, build, icons
```

## Tests

```bash
npm test                                          # 37 frontend tests
cargo test --manifest-path src-tauri/Cargo.toml   # 75 Rust tests
npm run typecheck && npm run lint && npm run rust:lint
```

**No test spawns the CLI, needs credentials, or makes a network call.** The
provider is tested by inspecting the command it would run and by parsing
recorded CLI output. Keep it that way: a suite that consumes quota is a suite
people stop running.

Coverage worth knowing about:

| Area | Where |
| --- | --- |
| NDJSON decoding, incl. split lines and multi-byte chars | `src/provider/ndjson.rs` |
| Stream-event mapping and error wording | `src/provider/wire.rs` |
| CLI argument construction, record parsing | `src/provider/claude_code.rs` |
| Persistence, ordering, crash recovery | `tests/persistence.rs` |
| Request assembly, role alternation, titles, branching | `tests/requests.rs` |
| Search escaping, FTS sync, scale | `tests/search_and_projects.rs` |
| Migrations | `src/db/migrations.rs` |
| Attachment validation | `src/attachments.rs` |
| Streaming buffer, formatting, Markdown safety | `src/**/*.test.ts(x)` |

## Developer mode

```bash
OPENCLAUDE_DEV=1 openclaude
OPENCLAUDE_LOG=openclaude=debug openclaude
OPENCLAUDE_DB=/tmp/scratch.db openclaude   # a throwaway database
```

Developer mode adds timing and diagnostics. It never logs prompt or response
content, and never a credential.

## Inspecting what the CLI is asked to do

The provider builds a `tokio::process::Command`; its unit tests assert on the
arguments rather than running anything. To see a real exchange by hand:

```bash
echo 'say hi' | claude --print --output-format stream-json \
  --include-partial-messages --verbose \
  --disallowed-tools "Bash Read Write Edit WebFetch" \
  --permission-mode dontAsk
```

Each line is one JSON record: `system/init` (carries the session id),
`stream_event` (wrapping the usual Anthropic event shapes), then `result`.

## Conventions

- Rust: `cargo fmt`, and `clippy` clean at `-D warnings`.
- TypeScript: no `any`; the IPC boundary uses `unknown` plus narrowing.
- Every IPC command returns `Result<T, AppError>`; the UI never sees a raw
  error string.
- Icon-only buttons take a required `label` prop, so nothing ships without an
  accessible name.
- Comments explain *why*. Anything that looks odd but is load-bearing —
  the FTS escaping, the autoscroll gesture rule, the flush interval — should
  say why it is that way.
