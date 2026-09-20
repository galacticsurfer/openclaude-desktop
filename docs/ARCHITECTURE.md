# Architecture

## Technology choice

The brief proposed Tauri 2 + React + TypeScript + Rust and asked for that to be
evaluated rather than assumed. It is the right choice here, and the decision
turns on one thing more than any other: **where the API key lives**.

| Option | Verdict |
| --- | --- |
| **Tauri 2 + React + TS** | **Chosen.** ~10 MB binary, ~90 MB idle RSS, sub-second cold start. The system WebView means no bundled browser. Critically, the Rust side can own all networking so the API key never enters the renderer — which in turn lets the CSP forbid the UI from making any outbound request at all. |
| Electron + React | Fastest to write, and the ecosystem is unmatched. But it ships a ~150 MB Chromium per app, idles around 250–400 MB, and the renderer is where people naturally put `fetch` — making key isolation a discipline rather than a property enforced by the platform. Rejected on footprint and on that security posture. |
| Flutter | Excellent rendering and startup. But it draws its own widgets, so it never inherits GTK theming, system fonts, or the desktop's accessibility stack, and Linux desktop support is the least mature target. Markdown and syntax highlighting would be rebuilt from scratch. Rejected. |
| GTK4 + Rust | The most genuinely native result and the best memory profile. The cost is the UI layer: rich Markdown, streaming text, syntax highlighting and diffing are all hand-built against a much smaller ecosystem. That is a multi-month detour before feature parity. Rejected for Phase 1, and worth revisiting only if the WebView proves limiting. |
| Qt | Mature and portable, but pulls in a large dependency for a Linux-first app, and the licensing conversation (LGPL dynamic linking, or commercial) is a burden a small open-source project does not need. Rejected. |

The deciding trade-off is that Tauri gives Electron's UI ecosystem with a
native-process security boundary in between. The main risk it carries is
WebKitGTK version skew across distributions — see
[LINUX-RISKS.md](LINUX-RISKS.md).

## Layers

```text
┌──────────────────────────────────────────────────────────────┐
│  Renderer  (WebKitGTK · React 19 · TypeScript · Tailwind)     │
│                                                              │
│   components/         stores/ (Zustand)     services/        │
│   ── UI only          ── view state         ── typed invoke  │
│                                                              │
│   No network access. No credentials. CSP: connect-src 'self' │
└───────────────────────────┬──────────────────────────────────┘
                            │  Tauri IPC (commands + events)
┌───────────────────────────┴──────────────────────────────────┐
│  Core  (Rust)                                                │
│                                                              │
│   commands/    thin: validate → delegate → return            │
│       │                                                      │
│       ├── chat.rs        orchestration, streaming, recovery  │
│       ├── db/repo/       SQL, one module per aggregate       │
│       ├── attachments.rs validation + content-addressed blobs│
│       ├── export.rs      Markdown / JSON / text              │
│       └── secrets.rs     Secret Service, never logged        │
│                │                          │                 │
│         provider/ (AIProvider)       db/ (SQLite)            │
│                │                          │                 │
└────────────────┼──────────────────────────┼─────────────────┘
                 │                          │
       api.anthropic.com            ~/.local/share/openclaude
```

Dependencies point inwards. `commands/` may call `chat`, `db::repo` and
`attachments`; none of those may call back into `commands`, and none of them
know Tauri exists beyond emitting events. That is what lets the whole core be
tested with no window on screen — the integration suites in `src-tauri/tests/`
never start a Tauri app.

### Authentication

Two mechanisms behind one `Credential` type: an API key from the keyring, or a
short-lived OAuth token from the Anthropic CLI (`ant auth login`). `chat.rs`
resolves whichever the `claude.authMode` setting names; everything downstream
just sees a `Credential`, and the provider derives the correct headers from
its variant.

Claude Code's `/login` is a first-party flow and deliberately not reused — see
[SECURITY-MODEL.md](SECURITY-MODEL.md).

### Why the HTTP client is in Rust

It would have been less code to call the API from TypeScript. Putting it in
Rust buys four things:

1. **The key never reaches the renderer.** There is deliberately no command
   that returns it. The UI can set, test, delete, and ask *whether* a key
   exists — never read one.
2. **The CSP can be closed.** Because nothing in the UI needs to reach the
   network, `connect-src` is `'self' ipc:`. Prompt-injected markup cannot
   exfiltrate a conversation, because there is no egress.
3. **Streams survive the UI.** A webview reload or a crashed renderer does not
   abort an in-flight response; the Rust task keeps writing to SQLite.
4. **Durability.** The stream is flushed to the database every 400 ms as it
   arrives, so a hard kill costs a sentence, not an answer.

### The provider seam

```rust
pub trait AIProvider: Send + Sync {
    fn id(&self) -> &'static str;
    async fn stream_message(&self, req: ChatRequest) -> Result<EventStream>;
    async fn send_message(&self, req: ChatRequest) -> Result<String>;
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    async fn verify_credentials(&self) -> Result<()>;
}
```

Only `AnthropicProvider` implements it, and nothing else is planned for
Phase 1. The point of the trait is not multi-provider support today — it is
that `chat.rs` speaks in `StreamEvent`, `ChatRequest` and `ContentBlock`
rather than in Anthropic's wire format, so adding a provider later is a new
module instead of a refactor.

## State management

Three Zustand stores, split by lifetime rather than by feature:

- `useSettingsStore` — settings, model list, credential status. Loaded once.
- `useConversationStore` — the list, the open conversation, its messages, and
  live stream buffers.
- `useUIStore` — overlays, toasts, sidebar. Never persisted except the
  sidebar's collapsed state.

The database is the source of truth. Stores hold a view of it and are
refreshed from events, rather than trying to stay in sync by prediction. The
one deliberate exception is settings, which apply optimistically and roll back
if the write fails — a settings toggle that lags is worse than one that
occasionally reverts.

## Rendering a stream without melting the CPU

Tokens arrive far faster than 60 fps. Writing each one to the store would
re-render and re-parse Markdown for the whole message per token.

Instead deltas accumulate in a module-level `Map` and are flushed to the store
once per animation frame (`useConversationStore.applyDelta`). Markdown parses
at most 60 times a second regardless of token rate, and `MessageBubble` is
memoised so only the streaming message re-renders.

Code blocks are *not* highlighted while streaming (`live` prop). Tokenising a
growing, syntactically incomplete block every frame is expensive and flickers;
plain monospace during the stream, Shiki once it settles, reads better and
costs far less.

## Further reading

- [SCHEMA.md](SCHEMA.md) — tables, indexes, FTS and migrations
- [STREAMING.md](STREAMING.md) — the SSE pipeline and interruption handling
- [SECURITY-MODEL.md](SECURITY-MODEL.md) — trust boundaries and key handling
- [UI-WIREFRAMES.md](UI-WIREFRAMES.md) — screen layouts
- [PACKAGING.md](PACKAGING.md) — .deb, AppImage, and what ships
- [LINUX-RISKS.md](LINUX-RISKS.md) — distribution-specific hazards
- [ROADMAP.md](ROADMAP.md) — what is built and what is next
