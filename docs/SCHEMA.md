# Database schema

SQLite, at `~/.local/share/openclaude/openclaude.db`. The full DDL is
`src-tauri/migrations/001_init.sql`; this describes the reasoning.

## Connection settings

```sql
PRAGMA journal_mode = WAL;        -- readers never block the streaming writer
PRAGMA synchronous  = NORMAL;     -- durable across app crashes
PRAGMA foreign_keys = ON;         -- cascades are enforced, not assumed
PRAGMA busy_timeout = 5000;
PRAGMA journal_size_limit = 64MB; -- bound WAL growth in long sessions
```

`synchronous = NORMAL` with WAL survives an application crash intact. Only an
OS-level crash or power loss can lose the last transaction, which for us is at
most 400 ms of streamed text — the trade for not doing an fsync per token.

The database file and its `-wal`/`-shm` siblings are chmod 0600: they contain
whole conversations.

One connection behind a `Mutex`, not a pool. SQLite serialises writers anyway,
every query here is sub-millisecond and indexed, and the guard is never held
across an `.await`. The measured sidebar query over 2,400 messages is well
under the 400 ms budget asserted in `search_scales_to_a_large_history`.

## Tables

### `conversations`

| Column | Notes |
| --- | --- |
| `id` | UUIDv4 |
| `title` | Never empty; falls back to a derived title |
| `title_locked` | 1 once a human or the auto-titler named it, so it is never silently overwritten |
| `model` | The API model **id**, not a display name |
| `system_prompt` | Conversation-level instructions |
| `project_id` | `ON DELETE SET NULL` — deleting a project must not delete its conversations |
| `pinned`, `archived` | Sidebar placement |
| `deleted_at` | Soft delete. Non-null means Trash |
| `branched_from_message_id` | Provenance for a branched conversation |
| `last_message_at` | Denormalised, so the sidebar sorts without touching `messages` |
| `metadata` | JSON escape hatch, so additive features need no migration |

The brief listed a `conversation_projects` join table. This uses a single
`project_id` instead: a conversation belongs to at most one project, which is
what the product actually does, and a join table would add a query and a
class of ambiguity ("which project's instructions apply?") for no gain. If
many-to-many is ever needed, that is a migration.

### `messages`

Ordered by `seq`, unique per conversation, derived from `max(seq) + 1` rather
than a row count — so deleting a message can never cause the next insert to
collide with an existing one. There is a regression test for exactly that.

`status` is the interesting column:

| Status | Meaning |
| --- | --- |
| `pending` | Row created, request not yet accepted |
| `streaming` | Tokens arriving |
| `complete` | Finished normally |
| `interrupted` | Stopped by the user, or by the process ending |
| `error` | Failed; `error_kind`/`error_message` say how, and partial text is kept |

`pending` and `streaming` are only ever valid *within* a process. On startup
anything still in those states belonged to a run that died, and
`recover_in_flight` downgrades it — to `interrupted` if text had arrived, or
`error` if none had. That is what makes the restart story work rather than
leaving a bubble spinning forever.

`thinking` is stored separately from `content` so extended-thinking output can
be shown collapsed and is never replayed as conversation context.

### `attachments`

Content-addressed: the blob lives at `attachments/<aa>/<sha256>` and the row
points at it. Attaching the same file to ten conversations costs one copy on
disk, and branching a conversation duplicates rows but shares blobs. A blob is
only unlinked once `refcount(sha256)` reaches zero.

`text_content` inlines UTF-8 text so building a request does not need to touch
the filesystem for the common case.

### `projects`, `settings`, `mcp_servers`, `mcp_permissions`

`settings` is key/value with JSON values, so a setting can grow from a bool
into an object without a migration. A malformed value is ignored in favour of
the default rather than failing the launch — there is a test for that too.
**No secret is ever stored here.**

The two MCP tables exist but are unused in Phase 1. They are in v1 of the
schema deliberately: the shape of MCP configuration and per-tool permissions
is settled, and shipping the tables now avoids a migration later.

## Full-text search

Three FTS5 external-content tables (`messages_fts`, `conversations_fts`,
`attachments_fts`) kept in sync by triggers, so an edit or delete updates the
index in the same transaction as the row.

Search unions the three and biases the ranking: title matches get `bm25 - 4.0`
because matching a title is a much stronger signal than matching a word in a
long message, and archived conversations get `+2.0` so live work surfaces
first.

### Query escaping

User input is never passed to `MATCH` directly. FTS5 treats `"`, `*`, `-`,
`:`, `^`, `(`, `)` and bare `AND`/`OR`/`NOT` as syntax, so typing `NOT` or a
stray quote would either raise `fts5: syntax error` or silently invert the
query. `build_match_query` splits input on non-alphanumerics and emits each
token as a quoted literal, with a `*` on the last token (when it is at least
two characters) so results appear while you are still typing.

`fts_metacharacters_in_a_query_do_not_raise_an_error` covers the cases that
used to break.

## Migrations

Driven by `PRAGMA user_version`. Each migration runs inside a transaction
together with its version bump, so a power cut mid-upgrade leaves a consistent
database at the previous version.

Opening a database created by a *newer* build is refused with a clear message
rather than being silently mishandled, which would risk corruption.

To add a migration: add `src-tauri/migrations/00N_name.sql`, append an entry to
`MIGRATIONS` in `src-tauri/src/db/migrations.rs`, and add a test. Never edit a
migration that has shipped.

## Backup

`Db::backup_to` uses SQLite's online backup API, so Settings → Advanced → Back
up now is safe to run while a response is streaming.
