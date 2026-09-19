# Streaming architecture

## The pipeline

```text
  Renderer                Tauri IPC              Rust core            Anthropic
  ────────                ─────────              ─────────            ─────────
  send(text) ──invoke──▶ send_message
                          │
                          ├─ TRANSACTION: user row + attachments
                          │               + assistant placeholder (streaming)
                          │
                          ├─ emit  chat:start ────────────────────▶ (UI shows a bubble)
                          │
                          └─ spawn task ──▶ plan_request
                                              │ conversation + project
                                              │ instructions → system
                                              │ history → alternating turns
                                              │ attachments → blocks
                                              ▼
                                            POST /v1/messages  ───────▶
                                                                stream: true
                                              ◀─────────────────────────
                                            SseDecoder → map_sse
                                              │
                    ◀── emit chat:delta ──────┤  per token
                                              │
                                              ├─ flush to SQLite every 400 ms
                                              │
                    ◀── emit chat:end ────────┴─ finalize row (status, usage)
                          │
  reloadMessages() ◀──────┘
```

The renderer only ever reads the database through commands and receives
events. It never holds the authoritative copy of a message.

## Decoding SSE

`provider/sse.rs` is a standalone decoder with no network or provider
knowledge, so the parts that actually break are unit-testable without a
server. It handles the cases that occur in practice:

- a frame split across TCP chunks (`event: content_bl` … `ock_delta`)
- a multi-byte UTF-8 character split across chunks — the buffer is only ever
  cut on an ASCII newline, so `from_utf8_lossy` cannot corrupt a character
- multi-line `data:` joined with newlines
- comment/keep-alive lines (`: ping`)
- CRLF as well as LF terminators
- exactly one optional leading space stripped from a value, so `data:  x`
  yields `" x"`

`map_sse` then converts frames into the provider-neutral `StreamEvent`. Any
event type it does not recognise returns `None` — a new upstream event is a
no-op, not a crash.

## Durability

Three mechanisms, because losing a long answer is the worst failure this app
can have:

1. **Periodic flush.** The accumulated buffer is written to SQLite every
   400 ms. Frequent enough that a `kill -9` costs a sentence; rare enough to
   avoid a disk write per token.
2. **Errors keep partial text.** `finish_with_error` writes whatever arrived
   before the failure, then marks the row `error`. A dropped connection
   halfway through never blanks the answer.
3. **Startup recovery.** `pending`/`streaming` are only meaningful inside a
   running process. On launch, `recover_in_flight` downgrades any survivor to
   `interrupted` (text arrived) or `error` (none did). The UI then offers
   Continue and Retry instead of spinning forever.

## Cancellation

Stop sets an `AtomicBool` rather than aborting the task. The loop notices at
the next event, then finalises normally: flush the text, mark the row
`interrupted`, emit `chat:end`. Aborting the future would leave the row in
`streaming` and the buffer unwritten.

Window destruction calls `cancel_all()` for the same reason.

## Timeouts

A per-chunk idle timeout of 120 s, not a whole-response timeout. A long answer
may legitimately stream for minutes, but Anthropic sends `ping` frames during
thinking pauses, so two minutes of true silence means a dead socket.

## Recovering a conversation for the next request

`build_messages` turns stored rows into an API-valid `messages` array and
enforces the two rules that matter:

- the first turn must be `user` — leading assistant rows are dropped
- roles must alternate — consecutive same-role rows are **merged**, which
  happens routinely after a failed generation

Which rows are replayed:

| Status | Replayed? |
| --- | --- |
| `complete` | Yes |
| `interrupted` with text | Yes — a partial answer is still real context |
| `error` (empty) | No — noise |
| `pending` / `streaming` | No — including the placeholder being filled |

`system` rows are never turns; instructions belong in the `system` field,
composed as project instructions first, then conversation instructions.

## Rendering without melting the CPU

Deltas land in a module-level `Map` and flush to the Zustand store once per
animation frame. Markdown therefore re-parses at most 60 times a second no
matter how fast tokens arrive, and `MessageBubble` is memoised so only the
streaming message re-renders.

Code blocks stay unhighlighted while `live` — re-tokenising a growing,
syntactically incomplete block every frame is expensive and flickers. Shiki
runs once the message settles.

The view follows the stream via a `ResizeObserver`, because growing content
fires no `scroll` event. Un-pinning happens only on a real gesture (wheel,
touch, scrollbar drag, keys); our own programmatic scrolls may only ever
re-pin. Deciding "the user scrolled away" from raw scroll events is wrong,
because those are delivered asynchronously — by the time one arrives more
tokens have grown the container, it reads as a large distance from the
bottom, and the follow is abandoned mid-reply. That bug was real and is what
the current design exists to avoid.
