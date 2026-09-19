# UI wireframes

The visual system is independently designed: a warm neutral scale with one
clay accent, quiet 1px borders, generous spacing, no gradients outside the
app icon, and no saturated fills. The goal is something you can leave open
beside an editor all day. Nothing here reproduces Anthropic's proprietary UI.

## Main window

```text
┌──────────────────────────┬────────────────────────────────────────────────────────┐
│ ┌──────────────────────┐ │ ▣  Debug Mongo pool timeout   [Claude Sonnet 4.5 ▾] (i)│
│ │ ⊞ New chat    Ctrl N │ │────────────────────────────────────────────────────────│
│ └──────────────────────┘ │                                                        │
│ ┌──────────────────────┐ │                                          You           │
│ │ ⌕ Filter titles…   ⌕ │ │                       ┌──────────────────────────────┐ │
│ └──────────────────────┘ │                       │ Why is my driver timing out? │ │
│                          │                       └──────────────────────────────┘ │
│ ▾ PROJECTS          ⊞    │                                                        │
│   ▤ All conversations    │  Claude                                                │
│   ▤ Backend API      12  │  Here's what's going on with that timeout.             │
│   ▤ Research          3  │                                                        │
│                          │  ## Cause                                              │
│ PINNED                   │  The pool is **exhausted**. Every checkout waits for    │
│  📌 Deploy runbook   2d  │  `waitQueueTimeoutMS` and then throws.                  │
│                          │                                                        │
│ TODAY                    │  ┌────────────────────────────────────────────────┐    │
│  ● Debug Mongo pool  now │  │ PYTHON  8 lines          ⤶  ⤓  ⧉               │    │
│  ● Review FastAPI    2h  │  ├────────────────────────────────────────────────┤    │
│                          │  │ client = AsyncIOMotorClient(                   │    │
│ YESTERDAY                │  │     "mongodb://localhost:27017",               │    │
│  ● Redis caching     1d  │  │     maxPoolSize=50,                            │    │
│  ● Interview prep    1d  │  │ )                                              │    │
│                          │  └────────────────────────────────────────────────┘    │
│                          │                                                        │
│                          │  │ Raising the pool alone hides the leak.               │
│                          │                                                        │
│                          │  842 in · 301 out · claude-sonnet-4-5                   │
│                          │                                                        │
│ ┌──────────────────────┐ │────────────────────────────────────────────────────────│
│ │Active│Archived│Trash │ │ ┌────────────────────────────────────────────────────┐ │
│ └──────────────────────┘ │ │ 📎  Message Claude…  (Enter to send)            ↑  │ │
│ ⌁ Connected           ⚙ │ │ └────────────────────────────────────────────────────┘ │
└──────────────────────────┴────────────────────────────────────────────────────────┘
   272px, collapsible                    Claude can make mistakes.
```

Notes that matter to the implementation:

- User turns are tinted cards, Claude's are flush on the canvas. Long answers
  should read like a document, not a chat bubble.
- The per-message action row (copy, branch) and the token line are revealed on
  hover, so a quiet conversation stays quiet.
- The sidebar groups by **local calendar day**, so something from 23:00 last
  night reads "Yesterday" at 00:30, not "Today".

## First launch

```text
                    ┌─────────────────────────────────┐
                    │              🔑                 │
                    │  Welcome to OpenClaude Desktop  │
                    │  An unofficial, open-source     │
                    │  Claude client for Linux.       │
                    ├─────────────────────────────────┤
                    │ Anthropic API key               │
                    │ ┌─────────────────────────────┐ │
                    │ │ sk-ant-…                    │ │
                    │ └─────────────────────────────┘ │
                    │ Get one from console.anthropic… │
                    │                                 │
                    │ ┌─ 🛡 ─────────────────────────┐ │
                    │ │ Stored in your system        │ │
                    │ │ keyring. Sent only to        │ │
                    │ │ api.anthropic.com. No        │ │
                    │ │ telemetry.                   │ │
                    │ └──────────────────────────────┘ │
                    │                                 │
                    │ Name conversations       [ ●─ ] │
                    │ automatically                   │
                    │ Makes one short request to a    │
                    │ small model. Off uses the first │
                    │ line of your message, free.     │
                    │                                 │
                    │ [Test connection] [Save and → ] │
                    │   Skip for now                  │
                    └─────────────────────────────────┘
       Not affiliated with, endorsed by, or sponsored by Anthropic.
```

Three things are stated before anything is stored: where the key goes, what
leaves the machine, and that auto-titling costs a small API call. Surprising
someone with a charge — however small — is not acceptable.

## Search (Ctrl+K)

```text
        ┌────────────────────────────────────────────────────┐
        │ ⌕  mongo pool                                    ⟳ │
        ├────────────────────────────────────────────────────┤
        │ ⌶  Debug Mongo pool timeout      title · 2h        │
        │    Debug ⟨Mongo⟩ ⟨pool⟩ timeout                    │
        │                                                    │
        │ ▤  Debug Mongo pool timeout      Claude · 2h    ↵  │
        │    the connection ⟨pool⟩ keeps exhausting itself   │
        │                                                    │
        │ 🗎  Review FastAPI architecture  attachment · 1d    │
        │    pool_config.py                                  │
        ├────────────────────────────────────────────────────┤
        │ ↑↓ navigate   ↵ open   Esc close        3 results  │
        └────────────────────────────────────────────────────┘
```

Matches are wrapped by FTS5 in `<<`/`>>` and rendered as React `<mark>` nodes,
never as HTML — the surrounding text is user and model content.

## Settings

```text
┌──────────────────────────────────────────────────────────────┐
│ Settings                                                   ✕ │
├───────────────┬──────────────────────────────────────────────┤
│ ⚙ General     │  Claude                                      │
│ 👁 Appearance │                                              │
│ ▣ Claude    ◀ │  API key                                     │
│ ⚡ MCP servers│  ┌ 🛡 Key ending …a1b2 is configured. ──────┐ │
│ ⊙ Notificat…  │  │    Stored in your system keyring. [Remove]│ │
│ ⌨ Shortcuts   │  └───────────────────────────────────────────┘ │
│ 🔒 Privacy    │  [ sk-ant-…            ] [Test] [Save]        │
│ ⛁ Advanced    │                                              │
│               │  Models                                      │
│               │  Default model          [Claude Sonnet 4.5 ▾]│
│               │  Max response length    [ 8192             ] │
│               │  Temperature            [ default          ] │
│               │                                              │
│               │  Conversation titles                         │
│               │  Name automatically              [ ●─ ]      │
└───────────────┴──────────────────────────────────────────────┘
```

## Interrupted and failed replies

Losing a half-written answer to a dropped connection is precisely the failure
this app should not have, so partial text is always kept:

```text
  Claude
  Write-ahead logging batches writes into a single

  ┌ ⏸ This reply was interrupted. ──────────────────────────┐
  │   It stopped before Claude finished — either you        │
  │   pressed Stop, or the app closed.                      │
  │   [▶ Continue]  [⟳ Retry]                               │
  └─────────────────────────────────────────────────────────┘
```

```text
  ┌ ⚠ Rate limit reached. Claude asked us to slow down. ────┐
  │   [⟳ Retry]  [Details]                                  │
  │   kind   rate_limit_error                               │
  │   model  claude-sonnet-4-5                              │
  └─────────────────────────────────────────────────────────┘
```

"Details" shows status, error category and the Anthropic `request-id` — useful
in a bug report, and never a stack trace or a secret.
