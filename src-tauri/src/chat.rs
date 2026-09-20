//! Chat orchestration: turning stored rows into a provider request, running
//! the stream, and writing the result back durably.

use crate::db::models::*;
use crate::db::repo;
use crate::db::Db;
use crate::error::{AppError, ErrorDetail, Result};
use crate::provider::claude_code::{ClaudeCodeProvider, SessionRef};
use crate::provider::{
    AIProvider, ChatRequest, ContentBlock, MediaSource, ProviderMessage, StreamEvent,
};
use crate::settings_defaults as sk;
use crate::state::{AppState, StreamHandle};
use base64::Engine;
use futures_util::StreamExt;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

/// How often streamed text is pushed to the UI.
///
/// The backend produces deltas far faster than a screen refreshes, and each
/// one crossing the IPC bridge costs a serialise plus a webview message. At
/// ~30ms the text still reads as continuous while the bridge carries a
/// fraction of the traffic.
const EMIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(30);

/// How often the accumulated stream buffer is written to SQLite.
///
/// The tension: too frequent and we do a disk write per token; too rare and a
/// hard kill loses more text. 400 ms bounds the loss to a sentence or two
/// while keeping writes to a couple per second.
const FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(400);

// --- events emitted to the frontend ---------------------------------------

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStart {
    pub conversation_id: String,
    pub message_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamDelta {
    pub conversation_id: String,
    pub message_id: String,
    pub text: String,
    /// `text` or `thinking`
    pub channel: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamEnd {
    pub conversation_id: String,
    pub message_id: String,
    pub status: MessageStatus,
    pub stop_reason: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub error: Option<ErrorDetail>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingUpdate {
    pub conversation_id: String,
    pub message_id: String,
    /// The provider's running estimate, when it has offered one.
    pub tokens: Option<i64>,
}

/// A tool call, as it happens. Also stored on the message so the record
/// survives a reload — a tool that ran is history, not a transient.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolUpdate {
    pub conversation_id: String,
    pub message_id: String,
    pub id: String,
    pub name: String,
    /// Arguments, once the completed message carries them.
    pub input: Option<serde_json::Value>,
    /// None while running, then the outcome.
    pub ok: Option<bool>,
}

pub const EV_START: &str = "chat:start";
pub const EV_DELTA: &str = "chat:delta";
pub const EV_THINKING: &str = "chat:thinking";
pub const EV_TOOL: &str = "chat:tool";
pub const EV_END: &str = "chat:end";
pub const EV_CONVERSATION: &str = "conversation:updated";
pub const EV_TITLE: &str = "conversation:title";

// --- request assembly -----------------------------------------------------

/// Compose the system prompt from the project's instructions and the
/// conversation's own. Project instructions come first so a conversation can
/// refine them.
pub fn compose_system(
    project_instructions: Option<&str>,
    conversation: Option<&str>,
) -> Option<String> {
    let parts: Vec<&str> = [project_instructions, conversation]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

/// Is this request rebuilding a conversation the CLI has no session for?
///
/// True after an edit, which truncates the transcript: the old session still
/// holds the turns we just removed, so it must be abandoned rather than
/// resumed. A branch also has no session, but inherits one by forking and so
/// already carries its context — replaying it there would duplicate it.
pub fn rebuilding_context(conversation: &Conversation, turns: usize) -> bool {
    conversation.provider_conversation_id.is_none()
        && conversation
            .metadata
            .get("forkFrom")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .is_none()
        && turns > 1
}

/// Render earlier turns as text, to be carried in the system prompt.
///
/// The obvious approach — replaying them as real turns — is not affordable:
/// the CLI answers every `user` record it is given, so an N-turn replay
/// costs N generations. Injected as system context it costs none, at the
/// price of the model seeing the history as a transcript rather than as its
/// own turns.
pub fn render_prior_turns(prior: &[Message]) -> String {
    let mut out = String::from(
        "## Earlier in this conversation\n\n\
         This is the transcript so far, for context. It has been edited: \
         answer only the message that follows it.\n",
    );
    for m in prior {
        let who = match m.role {
            Role::User => "User",
            Role::Assistant => "Claude",
            Role::System => continue,
        };
        let body = m.content.trim();
        if body.is_empty() && m.attachments.is_empty() {
            continue;
        }
        out.push_str(&format!("\n### {who}\n{body}\n"));
        for a in &m.attachments {
            out.push_str(&format!("[attachment: {}]\n", a.filename));
        }
    }
    out
}

/// Should this stored row be replayed to the provider?
fn is_replayable(m: &Message) -> bool {
    match m.status {
        // A completed turn always counts.
        MessageStatus::Complete => !m.content.trim().is_empty() || !m.attachments.is_empty(),
        // A partial answer is still real context — the user may be asking
        // Claude to continue it.
        MessageStatus::Interrupted => !m.content.trim().is_empty(),
        // An empty failure is noise; never send it.
        MessageStatus::Error => false,
        MessageStatus::Pending | MessageStatus::Streaming => false,
    }
}

/// Build the content blocks for one stored message.
///
/// Text attachments are inlined as fenced text with a filename header rather
/// than sent as opaque documents: it costs fewer tokens, and Claude can refer
/// to the file by name.
fn blocks_for(m: &Message, store_root: &std::path::Path) -> Result<Vec<ContentBlock>> {
    let mut blocks = Vec::new();

    for a in &m.attachments {
        match a.kind {
            AttachmentKind::Text => {
                let body = match &a.text_content {
                    Some(t) => t.clone(),
                    None => std::fs::read_to_string(store_root.join(&a.storage_path))
                        .unwrap_or_else(|_| "[file could not be read]".to_string()),
                };
                blocks.push(ContentBlock::Text {
                    text: format!("<file name=\"{}\">\n{}\n</file>", a.filename, body),
                });
            }
            AttachmentKind::Image | AttachmentKind::Document => {
                let bytes = std::fs::read(store_root.join(&a.storage_path)).map_err(|_| {
                    AppError::invalid(format!(
                        "The attachment \"{}\" is missing from local storage.",
                        a.filename
                    ))
                })?;
                let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                let source = MediaSource::Base64 {
                    media_type: a.mime_type.clone(),
                    data,
                };
                blocks.push(if a.kind == AttachmentKind::Image {
                    ContentBlock::Image { source }
                } else {
                    ContentBlock::Document { source }
                });
            }
        }
    }

    // Text last: with attachments present, the instruction reads better after
    // the material it refers to.
    if !m.content.trim().is_empty() {
        blocks.push(ContentBlock::Text {
            text: m.content.clone(),
        });
    }

    if blocks.is_empty() {
        blocks.push(ContentBlock::Text {
            text: "(empty message)".to_string(),
        });
    }
    Ok(blocks)
}

/// Convert stored history into the provider's message list.
///
/// Enforces the two rules the Messages API cares about: the first turn must be
/// `user`, and roles must alternate. Consecutive same-role rows — which happen
/// after a failed generation, or when the user sends twice in a row — are
/// merged rather than rejected.
pub fn build_messages(
    history: &[Message],
    store_root: &std::path::Path,
) -> Result<Vec<ProviderMessage>> {
    let mut out: Vec<ProviderMessage> = Vec::new();

    for m in history.iter().filter(|m| is_replayable(m)) {
        let role = match m.role {
            Role::Assistant => "assistant",
            // A stored `system` row is not a turn; it belongs in `system`.
            Role::System => continue,
            Role::User => "user",
        };

        // Drop leading assistant turns: the API requires the first to be user.
        if out.is_empty() && role == "assistant" {
            continue;
        }

        let blocks = blocks_for(m, store_root)?;
        match out.last_mut() {
            Some(prev) if prev.role == role => prev.content.extend(blocks),
            _ => out.push(ProviderMessage {
                role,
                content: blocks,
            }),
        }
    }

    Ok(out)
}

pub struct RequestPlan {
    pub request: ChatRequest,
    pub conversation: Conversation,
}

/// Assemble everything needed for one generation from the database.
pub fn plan_request(
    db: &Db,
    conversation_id: &str,
    store_root: &std::path::Path,
) -> Result<RequestPlan> {
    let conn = db.conn();
    let conversation = repo::conversations::get(&conn, conversation_id)?;

    let project_instructions = conversation
        .project_id
        .as_deref()
        .and_then(|pid| repo::projects::get(&conn, pid).ok())
        .map(|p| p.instructions)
        .filter(|s| !s.trim().is_empty());

    let mut system = compose_system(
        project_instructions.as_deref(),
        conversation.system_prompt.as_deref(),
    );

    let history = repo::messages::list(&conn, conversation_id)?;
    let messages = build_messages(&history, store_root)?;

    if messages.is_empty() {
        return Err(AppError::invalid("There is nothing to send yet."));
    }

    // Only the final turn is sent on the wire; everything before it normally
    // lives in the CLI's own session. When there is no session to resume,
    // that history has to travel some other way.
    if rebuilding_context(&conversation, messages.len()) {
        let replayable: Vec<Message> = history.into_iter().filter(is_replayable).collect();
        let prior = &replayable[..replayable.len().saturating_sub(1)];
        if !prior.is_empty() {
            let transcript = render_prior_turns(prior);
            system = Some(match system {
                Some(s) => format!("{s}\n\n{transcript}"),
                None => transcript,
            });
        }
    }

    Ok(RequestPlan {
        request: ChatRequest {
            model: conversation.model.clone(),
            system,
            messages,
            // The CLI owns response limits and sampling; neither is exposed
            // as a flag, so carrying them here would be a lie.
            max_tokens: 0,
            temperature: None,
            stop_sequences: Vec::new(),
        },
        conversation,
    })
}

// --- running a generation -------------------------------------------------

/// Where the CLI process should run for this conversation.
///
/// A project's working folder when it has one — that is the point of the
/// setting, and it lets `CLAUDE.md` in that repo apply. Otherwise a dedicated
/// empty directory, so a stray `CLAUDE.md` in some unrelated path cannot
/// silently join the conversation.
fn working_dir(db: &Db, conversation: &Conversation) -> std::path::PathBuf {
    if let Some(dir) = conversation
        .project_id
        .as_deref()
        .and_then(|pid| repo::projects::get(&db.conn(), pid).ok())
        .and_then(|p| p.working_dir)
        .filter(|d| !d.trim().is_empty())
    {
        let path = std::path::PathBuf::from(dir);
        if path.is_dir() {
            return path;
        }
    }
    crate::paths::session_dir()
}

fn provider_for(db: &Db, conversation: &Conversation, fresh: bool) -> Result<ClaudeCodeProvider> {
    // The conversation id doubles as the Claude Code session id — it is
    // already a UUID, which is what `--session-id` requires. The first turn
    // claims it; later turns resume it.
    let session = match conversation.provider_conversation_id.as_deref() {
        Some(id) if !id.is_empty() => SessionRef::Resume(id.to_string()),
        // A branch has no session yet but inherits one to continue from, so
        // the copied history is actually in context rather than only on
        // screen. Forking keeps the original conversation untouched.
        _ => match conversation
            .metadata
            .get("forkFrom")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            Some(from) => SessionRef::Fork {
                from: from.to_string(),
            },
            // The conversation id seeds the first session. A rebuild cannot
            // reuse it — the CLI rejects a `--session-id` it already knows —
            // so later ones get an id of their own.
            None if !fresh => SessionRef::New(crate::db::models::new_id()),
            None => SessionRef::New(conversation.id.clone()),
        },
    };
    let effort: Option<String> = {
        let conn = db.conn();
        repo::settings::get_or(&conn, sk::EFFORT, None)
    };
    // Tools exist only where the user has enabled a server and approved
    // something on it; otherwise this is empty and the session has none.
    let mcp = crate::mcp::plan(
        db,
        &crate::paths::mcp_dir(),
        conversation.project_id.as_deref(),
    )
    .unwrap_or_else(|e| {
        tracing::warn!(error = %e, "could not prepare MCP config; continuing without tools");
        crate::mcp::McpPlan::default()
    });

    Ok(
        ClaudeCodeProvider::new(working_dir(db, conversation), session)
            .with_effort(effort)
            .with_mcp(mcp),
    )
}

/// Append a user turn and start generating a reply.
pub async fn send(
    app: AppHandle,
    state: Arc<AppState>,
    conversation_id: String,
    text: String,
    attachment_ids: Vec<String>,
) -> Result<String> {
    if state.is_streaming(&conversation_id) {
        return Err(AppError::invalid(
            "This conversation is already generating a reply.",
        ));
    }
    if text.trim().is_empty() && attachment_ids.is_empty() {
        return Err(AppError::invalid("Type a message or attach a file first."));
    }

    let db = state.db.clone();
    let now = now_ms();

    // One transaction: user turn, its attachments, and the assistant
    // placeholder all land together or not at all.
    let assistant_id = db.tx(|tx| {
        let user = repo::messages::insert(
            tx,
            repo::messages::NewMessage {
                conversation_id: &conversation_id,
                role: Role::User,
                content: text.trim(),
                status: MessageStatus::Complete,
                model: None,
            },
        )?;
        if !attachment_ids.is_empty() {
            repo::attachments::attach_to_message(tx, &attachment_ids, &user.id)?;
            tx.execute(
                "UPDATE attachments SET conversation_id = ?2 WHERE id IN
                 (SELECT id FROM attachments WHERE message_id = ?1)",
                rusqlite::params![user.id, conversation_id],
            )?;
        }

        let model: String = repo::conversations::get(tx, &conversation_id)?.model;
        let assistant = repo::messages::insert(
            tx,
            repo::messages::NewMessage {
                conversation_id: &conversation_id,
                role: Role::Assistant,
                content: "",
                status: MessageStatus::Streaming,
                model: Some(&model),
            },
        )?;
        repo::conversations::touch(tx, &conversation_id, now)?;
        Ok(assistant.id)
    })?;

    spawn_stream(app, state, conversation_id, assistant_id.clone());
    Ok(assistant_id)
}

/// Regenerate the last assistant turn. The failed/unwanted reply and anything
/// after it is removed first, so history stays a clean alternating sequence.
pub async fn retry(
    app: AppHandle,
    state: Arc<AppState>,
    conversation_id: String,
) -> Result<String> {
    if state.is_streaming(&conversation_id) {
        return Err(AppError::invalid(
            "This conversation is already generating a reply.",
        ));
    }
    let db = state.db.clone();

    let assistant_id = db.tx(|tx| {
        let last = repo::messages::last(tx, &conversation_id)?
            .ok_or_else(|| AppError::invalid("There is nothing to retry."))?;

        if last.role == Role::Assistant {
            repo::messages::delete_from_seq(tx, &conversation_id, last.seq)?;
        }

        let model = repo::conversations::get(tx, &conversation_id)?.model;
        let assistant = repo::messages::insert(
            tx,
            repo::messages::NewMessage {
                conversation_id: &conversation_id,
                role: Role::Assistant,
                content: "",
                status: MessageStatus::Streaming,
                model: Some(&model),
            },
        )?;
        Ok(assistant.id)
    })?;

    spawn_stream(app, state, conversation_id, assistant_id.clone());
    Ok(assistant_id)
}

/// Rewrite a user turn, discard everything after it, and answer again.
///
/// The CLI session cannot be rewound, so the one holding the removed turns
/// is abandoned: clearing `provider_conversation_id` makes the next request
/// build a fresh session, with the surviving history carried in the system
/// prompt instead. See `rebuilding_context`.
pub async fn edit_and_resend(
    app: AppHandle,
    state: Arc<AppState>,
    conversation_id: String,
    message_id: String,
    text: String,
) -> Result<String> {
    if state.is_streaming(&conversation_id) {
        return Err(AppError::invalid(
            "This conversation is already generating a reply.",
        ));
    }
    if text.trim().is_empty() {
        return Err(AppError::invalid("An edited message cannot be empty."));
    }
    let db = state.db.clone();

    let assistant_id = db.tx(|tx| {
        let target = repo::messages::get(tx, &message_id)?;
        if target.conversation_id != conversation_id {
            return Err(AppError::invalid(
                "That message is in another conversation.",
            ));
        }
        if target.role != Role::User {
            return Err(AppError::invalid("Only your own messages can be edited."));
        }

        // Rewrite in place rather than replacing the row: the turn keeps its
        // `seq` and its attachments stay attached to it.
        tx.execute(
            "UPDATE messages SET content = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![target.id, text.trim(), now_ms()],
        )?;
        // Everything after it is now answering a question that was not asked.
        repo::messages::delete_from_seq(tx, &conversation_id, target.seq + 1)?;

        // Abandon the session that still holds the turns just removed.
        tx.execute(
            "UPDATE conversations SET provider_conversation_id = NULL WHERE id = ?1",
            rusqlite::params![conversation_id],
        )?;

        let model = repo::conversations::get(tx, &conversation_id)?.model;
        let assistant = repo::messages::insert(
            tx,
            repo::messages::NewMessage {
                conversation_id: &conversation_id,
                role: Role::Assistant,
                content: "",
                status: MessageStatus::Streaming,
                model: Some(&model),
            },
        )?;
        Ok(assistant.id)
    })?;

    spawn_stream(app, state, conversation_id, assistant_id.clone());
    Ok(assistant_id)
}

/// Ask Claude to carry on from a reply that was cut short, keeping the partial
/// text as its own turn and adding a fresh one after it.
pub async fn continue_interrupted(
    app: AppHandle,
    state: Arc<AppState>,
    conversation_id: String,
) -> Result<String> {
    if state.is_streaming(&conversation_id) {
        return Err(AppError::invalid(
            "This conversation is already generating a reply.",
        ));
    }
    let db = state.db.clone();

    let assistant_id = db.tx(|tx| {
        let last = repo::messages::last(tx, &conversation_id)?
            .ok_or_else(|| AppError::invalid("There is nothing to continue."))?;
        if last.role != Role::Assistant || last.status != MessageStatus::Interrupted {
            return Err(AppError::invalid("The last reply was not interrupted."));
        }
        // Promote the partial text to a complete turn so it is replayed as
        // context, then add a user nudge and a new placeholder.
        repo::messages::finalize(
            tx,
            &last.id,
            repo::messages::Completion {
                content: &last.content,
                status: Some(MessageStatus::Complete),
                ..Default::default()
            },
        )?;
        repo::messages::insert(
            tx,
            repo::messages::NewMessage {
                conversation_id: &conversation_id,
                role: Role::User,
                content: "Please continue from exactly where you left off.",
                status: MessageStatus::Complete,
                model: None,
            },
        )?;
        let model = repo::conversations::get(tx, &conversation_id)?.model;
        let assistant = repo::messages::insert(
            tx,
            repo::messages::NewMessage {
                conversation_id: &conversation_id,
                role: Role::Assistant,
                content: "",
                status: MessageStatus::Streaming,
                model: Some(&model),
            },
        )?;
        Ok(assistant.id)
    })?;

    spawn_stream(app, state, conversation_id, assistant_id.clone());
    Ok(assistant_id)
}

pub fn stop(state: &AppState, conversation_id: &str) -> Result<()> {
    state
        .cancel(conversation_id)
        .map(|_| ())
        .ok_or_else(|| AppError::invalid("Nothing is generating in this conversation."))
}

fn spawn_stream(app: AppHandle, state: Arc<AppState>, conversation_id: String, message_id: String) {
    let cancel = Arc::new(AtomicBool::new(false));
    state.register(
        &conversation_id,
        StreamHandle {
            message_id: message_id.clone(),
            cancel: cancel.clone(),
        },
    );

    let _ = app.emit(
        EV_START,
        StreamStart {
            conversation_id: conversation_id.clone(),
            message_id: message_id.clone(),
        },
    );

    tauri::async_runtime::spawn(async move {
        let result = run_stream(&app, &state, &conversation_id, &message_id, cancel).await;
        state.unregister(&conversation_id);

        if let Err(e) = &result {
            tracing::warn!(conversation = %conversation_id, error = %e, "generation failed");
        }
        let _ = app.emit(EV_CONVERSATION, conversation_id.clone());
    });
}

async fn run_stream(
    app: &AppHandle,
    state: &Arc<AppState>,
    conversation_id: &str,
    message_id: &str,
    cancel: Arc<AtomicBool>,
) -> Result<()> {
    let db = state.db.clone();
    let store_root = crate::paths::attachments_dir();
    let started = std::time::Instant::now();

    // Any failure before the first token still has to finalise the row and
    // tell the UI, or the bubble spins forever.
    let plan = match plan_request(&db, conversation_id, &store_root) {
        Ok(p) => p,
        Err(e) => return finish_with_error(app, &db, conversation_id, message_id, e, ""),
    };
    // A conversation whose transcript was truncated cannot resume its old
    // session; plan_request has already moved that history into the system
    // prompt, and this gives it a session id the CLI has not seen.
    let fresh = !rebuilding_context(&plan.conversation, plan.request.messages.len());
    let provider = match provider_for(&db, &plan.conversation, fresh) {
        Ok(p) => p,
        Err(e) => return finish_with_error(app, &db, conversation_id, message_id, e, ""),
    };
    let first_turn = plan.conversation.provider_conversation_id.is_none();

    let mut stream = match provider.stream_message(plan.request).await {
        Ok(s) => s,
        Err(e) => return finish_with_error(app, &db, conversation_id, message_id, e, ""),
    };

    let mut text = String::new();
    let mut thinking = String::new();
    let mut provider_message_id: Option<String> = None;
    let mut resolved_model: Option<String> = None;
    let mut reported_session: Option<String> = None;
    let mut stop_reason: Option<String> = None;
    let (mut input_tokens, mut output_tokens) = (None, None);
    let (mut cache_read, mut cache_write) = (None, None);
    let mut last_flush = std::time::Instant::now();
    // Deltas are batched rather than forwarded one by one; see EMIT_INTERVAL.
    let mut pending_text = String::new();
    let mut pending_thinking = String::new();
    let mut last_emit = std::time::Instant::now();
    // Reasoning: Claude Code reports a token estimate rather than text.
    let mut thinking_tokens: Option<i64> = None;
    // Tool calls in this reply, kept so they can be persisted with it.
    let mut tool_calls: Vec<serde_json::Value> = Vec::new();

    while let Some(item) = stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            break;
        }

        let event = match item {
            Ok(e) => e,
            Err(e) => return finish_with_error(app, &db, conversation_id, message_id, e, &text),
        };

        match event {
            StreamEvent::SessionReady {
                session_id,
                model,
                slash_commands,
                tools,
            } => {
                // The lockdown is a deny list, so a CLI upgrade could add a
                // tool it has never heard of. Say so loudly rather than
                // letting a chat window quietly gain file access.
                let approved = {
                    let conn = db.conn();
                    let project = repo::conversations::get(&conn, conversation_id)
                        .ok()
                        .and_then(|c| c.project_id);
                    repo::mcp::allowed_tool_names(&conn, project.as_deref()).unwrap_or_default()
                };
                let unexpected = crate::provider::claude_code::unexpected_tools(&tools, &approved);
                if !unexpected.is_empty() {
                    tracing::warn!(
                        tools = ?unexpected,
                        "Claude Code offered tools this build does not disable"
                    );
                    let conn = db.conn();
                    let _ = repo::settings::set(&conn, sk::UNEXPECTED_TOOLS, &unexpected);
                } else {
                    let conn = db.conn();
                    let _ = repo::settings::set(&conn, sk::UNEXPECTED_TOOLS, &Vec::<String>::new());
                }
                // Record the id the CLI actually adopted: a forked session
                // gets a fresh one rather than the id we asked for.
                if !session_id.is_empty() {
                    reported_session = Some(session_id);
                }
                // The alias the user picked ("sonnet") resolved to a concrete
                // model; record it so the UI can show what actually ran.
                resolved_model = Some(model);
                // Slash commands belong to the CLI installation, not the
                // conversation, so they are cached app-wide for the composer.
                if !slash_commands.is_empty() {
                    let conn = db.conn();
                    let _ = repo::settings::set(&conn, sk::SLASH_COMMANDS, &slash_commands);
                }
            }
            StreamEvent::Started {
                message_id: pid,
                input_tokens: it,
                cache_read: cr,
                cache_write: cw,
                ..
            } => {
                provider_message_id = Some(pid);
                input_tokens = it;
                cache_read = cr;
                cache_write = cw;
            }
            StreamEvent::TextDelta(chunk) => {
                text.push_str(&chunk);
                pending_text.push_str(&chunk);
            }
            StreamEvent::ThinkingDelta(chunk) => {
                thinking.push_str(&chunk);
                pending_thinking.push_str(&chunk);
            }
            StreamEvent::ThinkingStarted => {
                // Announce it immediately: with no reasoning text coming,
                // this is the only thing standing between the user and a
                // blank screen while Claude thinks.
                emit_thinking(app, conversation_id, message_id, None);
            }
            StreamEvent::ThinkingProgress(n) => {
                // Estimates can arrive out of order or restart per block;
                // the largest seen is the honest running total.
                if thinking_tokens.map_or(true, |prev| n > prev) {
                    thinking_tokens = Some(n);
                    emit_thinking(app, conversation_id, message_id, thinking_tokens);
                }
            }
            StreamEvent::ToolCall { id, name, input } => {
                // The call is announced first without arguments, then
                // repeated with them — update in place rather than listing
                // the same call twice.
                match tool_calls
                    .iter_mut()
                    .find(|c| c.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
                {
                    Some(existing) => {
                        if let Some(args) = input.clone() {
                            existing["input"] = args;
                        }
                    }
                    None => tool_calls.push(serde_json::json!({
                        "id": id, "name": name, "input": input
                    })),
                }
                let _ = app.emit(
                    EV_TOOL,
                    ToolUpdate {
                        conversation_id: conversation_id.to_string(),
                        message_id: message_id.to_string(),
                        id,
                        name,
                        input,
                        ok: None,
                    },
                );
            }
            StreamEvent::ToolResult { id, ok } => {
                if let Some(entry) = tool_calls
                    .iter_mut()
                    .find(|c| c.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
                {
                    entry["ok"] = serde_json::Value::Bool(ok);
                }
                let name = tool_calls
                    .iter()
                    .find(|c| c.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
                    .and_then(|c| c.get("name").and_then(|v| v.as_str()))
                    .unwrap_or_default()
                    .to_string();
                let _ = app.emit(
                    EV_TOOL,
                    ToolUpdate {
                        conversation_id: conversation_id.to_string(),
                        message_id: message_id.to_string(),
                        id,
                        name,
                        input: None,
                        ok: Some(ok),
                    },
                );
            }
            StreamEvent::Completed {
                stop_reason: sr,
                output_tokens: ot,
            } => {
                stop_reason = sr;
                output_tokens = ot;
            }
            StreamEvent::Failed(detail) => {
                return finish_with_error(
                    app,
                    &db,
                    conversation_id,
                    message_id,
                    AppError::Provider(Box::new(detail)),
                    &text,
                )
            }
        }

        if last_emit.elapsed() >= EMIT_INTERVAL {
            emit_pending(
                app,
                conversation_id,
                message_id,
                &mut pending_text,
                &mut pending_thinking,
            );
            last_emit = std::time::Instant::now();
        }

        // Periodic durability: a kill -9 now costs at most FLUSH_INTERVAL of text.
        if last_flush.elapsed() >= FLUSH_INTERVAL {
            let t = (!thinking.is_empty()).then_some(thinking.as_str());
            let _ = repo::messages::update_stream_buffer(&db.conn(), message_id, &text, t);
            last_flush = std::time::Instant::now();
        }
    }

    // Whatever is still batched must reach the UI before the message is
    // marked finished, or the tail of the reply would appear only after the
    // row is reloaded.
    emit_pending(
        app,
        conversation_id,
        message_id,
        &mut pending_text,
        &mut pending_thinking,
    );

    let cancelled = cancel.load(Ordering::SeqCst);
    let status = if cancelled {
        MessageStatus::Interrupted
    } else {
        MessageStatus::Complete
    };

    {
        let conn = db.conn();
        repo::messages::finalize(
            &conn,
            message_id,
            repo::messages::Completion {
                content: &text,
                thinking: (!thinking.is_empty()).then_some(thinking.as_str()),
                thinking_tokens,
                status: Some(status),
                // Record the concrete model, not the alias, so an old
                // conversation still says what actually answered it.
                model: resolved_model.as_deref(),
                provider_message_id: provider_message_id.as_deref(),
                stop_reason: stop_reason.as_deref(),
                input_tokens,
                output_tokens,
                cache_read_tokens: cache_read,
                cache_write_tokens: cache_write,
            },
        )?;
        if !tool_calls.is_empty() {
            // Kept on the message so the record of what ran survives a
            // reload, not just the live event.
            let _ = repo::messages::merge_metadata(
                &conn,
                message_id,
                &serde_json::json!({ "toolCalls": tool_calls }),
            );
        }
        repo::conversations::touch(&conn, conversation_id, now_ms())?;
        // Claude Code adopts the id we passed on the first turn; record it so
        // the next turn resumes the same session instead of starting over.
        if first_turn && !cancelled {
            // The id the CLI reports, not the one we asked for: a forked
            // session gets a fresh id of its own.
            if let Some(session) = reported_session.as_deref() {
                let _ = conn.execute(
                    "UPDATE conversations SET provider_conversation_id = ?2 WHERE id = ?1
                       AND provider_conversation_id IS NULL",
                    rusqlite::params![conversation_id, session],
                );
            }
        }
    }

    let _ = app.emit(
        EV_END,
        StreamEnd {
            conversation_id: conversation_id.to_string(),
            message_id: message_id.to_string(),
            status,
            stop_reason: stop_reason.clone(),
            input_tokens,
            output_tokens,
            error: None,
        },
    );

    if !cancelled {
        notify_if_unfocused(app, &db, conversation_id, started.elapsed());
        maybe_generate_title(app.clone(), state.clone(), conversation_id.to_string());
    }
    Ok(())
}

/// Send any batched text to the UI and clear the buffers.
fn emit_pending(
    app: &AppHandle,
    conversation_id: &str,
    message_id: &str,
    text: &mut String,
    thinking: &mut String,
) {
    for (buf, channel) in [(text, "text"), (thinking, "thinking")] {
        if buf.is_empty() {
            continue;
        }
        let _ = app.emit(
            EV_DELTA,
            StreamDelta {
                conversation_id: conversation_id.to_string(),
                message_id: message_id.to_string(),
                text: std::mem::take(buf),
                channel,
            },
        );
    }
}

/// Tell the UI that Claude is reasoning, and how much of it there has been.
fn emit_thinking(app: &AppHandle, conversation_id: &str, message_id: &str, tokens: Option<i64>) {
    let _ = app.emit(
        EV_THINKING,
        ThinkingUpdate {
            conversation_id: conversation_id.to_string(),
            message_id: message_id.to_string(),
            tokens,
        },
    );
}

/// Persist a failure, keeping whatever text arrived, and tell the UI.
fn finish_with_error(
    app: &AppHandle,
    db: &Db,
    conversation_id: &str,
    message_id: &str,
    err: AppError,
    partial: &str,
) -> Result<()> {
    let detail = err.detail().cloned().unwrap_or(ErrorDetail {
        kind: err.kind().to_string(),
        message: err.to_string(),
        status: None,
        request_id: None,
        retryable: err.retryable(),
        retry_after_secs: None,
    });

    {
        let conn = db.conn();
        // Partial text is preserved: losing a half-written answer to a dropped
        // connection is exactly the failure this app should not have.
        let _ = repo::messages::fail(&conn, message_id, &detail.kind, &detail.message, partial);
    }

    let _ = app.emit(
        EV_END,
        StreamEnd {
            conversation_id: conversation_id.to_string(),
            message_id: message_id.to_string(),
            status: MessageStatus::Error,
            stop_reason: None,
            input_tokens: None,
            output_tokens: None,
            error: Some(detail),
        },
    );
    Ok(())
}

fn notify_if_unfocused(
    app: &AppHandle,
    db: &Db,
    conversation_id: &str,
    elapsed: std::time::Duration,
) {
    let (enabled, min_ms): (bool, u64) = {
        let conn = db.conn();
        (
            repo::settings::get_or(&conn, sk::NOTIFY_ENABLED, true),
            repo::settings::get_or(&conn, sk::NOTIFY_MIN_MS, 8000u64),
        )
    };
    if !enabled || elapsed.as_millis() < min_ms as u128 {
        return;
    }

    let focused = app
        .get_webview_window("main")
        .and_then(|w| w.is_focused().ok())
        .unwrap_or(false);
    if focused {
        return;
    }

    let title = db
        .conn()
        .query_row(
            "SELECT title FROM conversations WHERE id = ?1",
            [conversation_id],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "Conversation".to_string());

    use tauri_plugin_notification::NotificationExt;
    let _ = app
        .notification()
        .builder()
        .title("Claude finished responding")
        .body(title)
        .show();
}

// --- automatic titles -----------------------------------------------------

/// Derive an immediate placeholder from the user's own words, so a
/// conversation is never nameless even with auto-titling off or offline.
pub fn fallback_title(first_user_message: &str) -> String {
    // Find the first line of actual prose: skip blank lines, quoted text, and
    // everything *inside* a fenced code block — a title of "print(1)" would
    // be worse than no title at all.
    let mut in_fence = false;
    let cleaned = first_user_message
        .lines()
        .map(str::trim)
        .find(|l| {
            if l.starts_with("```") || l.starts_with("~~~") {
                in_fence = !in_fence;
                return false;
            }
            !in_fence && !l.is_empty() && !l.starts_with('>') && !l.starts_with('#')
        })
        .unwrap_or("New conversation");

    let mut out = String::new();
    for word in cleaned.split_whitespace() {
        if out.len() + word.len() + 1 > 48 {
            out.push('…');
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    if out.is_empty() {
        "New conversation".to_string()
    } else {
        out
    }
}

/// Strip the wrapping a small model tends to add around a one-line answer.
pub fn clean_generated_title(raw: &str) -> Option<String> {
    let mut t = raw.trim().lines().next().unwrap_or("").trim().to_string();
    for q in ['"', '\'', '“', '”', '`'] {
        t = t.trim_matches(q).to_string();
    }
    t = t.trim().trim_end_matches('.').trim().to_string();
    if t.is_empty() || t.len() > 80 {
        return None;
    }
    Some(t)
}

const TITLE_PROMPT: &str =
    "Write a title of at most six words for the conversation that starts with the message below. \
Reply with the title only — no quotes, no punctuation at the end, no preamble.\n\n";

/// Generate a title in the background after the first exchange.
///
/// This costs a small API call, so it is gated on the `claude.autoTitle`
/// setting, is disclosed during onboarding, and always runs on the cheapest
/// model available. Failure is silent: the fallback title already stands.
fn maybe_generate_title(app: AppHandle, state: Arc<AppState>, conversation_id: String) {
    tauri::async_runtime::spawn(async move {
        let db = state.db.clone();

        let (enabled, locked, first_user, model, title_model) = {
            let conn = db.conn();
            let Ok(c) = repo::conversations::get(&conn, &conversation_id) else {
                return;
            };
            let enabled: bool = repo::settings::get_or(&conn, sk::AUTO_TITLE, true);
            let first = repo::messages::list(&conn, &conversation_id)
                .ok()
                .and_then(|ms| ms.into_iter().find(|m| m.role == Role::User))
                .map(|m| m.content);
            let tm: Option<String> = repo::settings::get_or(&conn, sk::TITLE_MODEL, None);
            (enabled, c.title_locked, first, c.model, tm)
        };

        if locked {
            return;
        }
        let Some(first_user) = first_user.filter(|s| !s.trim().is_empty()) else {
            return;
        };

        // Always set the derived title first: instant, free, and good enough
        // that a failed or disabled API call is not a regression.
        let fallback = fallback_title(&first_user);
        {
            let conn = db.conn();
            let _ = conn.execute(
                "UPDATE conversations SET title = ?2, updated_at = ?3
                  WHERE id = ?1 AND title_locked = 0",
                rusqlite::params![conversation_id, fallback, now_ms()],
            );
        }
        let _ = app.emit(EV_TITLE, (conversation_id.clone(), fallback));
        let _ = app.emit(EV_CONVERSATION, conversation_id.clone());

        if !enabled {
            return;
        }

        let Ok(conversation) = ({
            let conn = db.conn();
            repo::conversations::get(&conn, &conversation_id)
        }) else {
            return;
        };
        // A fresh throwaway session: asking for a title inside the
        // conversation's own session would put it in the transcript.
        let provider = ClaudeCodeProvider::new(
            crate::paths::session_dir(),
            SessionRef::New(crate::db::models::new_id()),
        );
        let _ = &conversation;

        let excerpt: String = first_user.chars().take(2000).collect();
        let req = ChatRequest {
            model: title_model.unwrap_or(model),
            system: None,
            messages: vec![ProviderMessage {
                role: "user",
                content: vec![ContentBlock::Text {
                    text: format!("{TITLE_PROMPT}{excerpt}"),
                }],
            }],
            max_tokens: 0,
            temperature: None,
            stop_sequences: Vec::new(),
        };

        match provider.send_message(req).await {
            Ok(raw) => {
                if let Some(title) = clean_generated_title(&raw) {
                    let wrote = {
                        let conn = db.conn();
                        repo::conversations::set_generated_title(&conn, &conversation_id, &title)
                            .unwrap_or(false)
                    };
                    if wrote {
                        let _ = app.emit(EV_TITLE, (conversation_id.clone(), title));
                        let _ = app.emit(EV_CONVERSATION, conversation_id);
                    }
                }
            }
            Err(e) => tracing::debug!(error = %e, "title generation skipped"),
        }
    });
}

// --- branching ------------------------------------------------------------

/// Copy a conversation's history up to and including one message into a new
/// conversation, so an alternative direction can be explored without touching
/// the original.
pub fn branch_from(db: &Db, message_id: &str) -> Result<Conversation> {
    db.tx(|tx| {
        let source_msg = repo::messages::get(tx, message_id)?;
        let source = repo::conversations::get(tx, &source_msg.conversation_id)?;
        let history = repo::messages::list_through_seq(tx, &source.id, source_msg.seq)?;

        let new_id = new_id();
        let now = now_ms();
        tx.execute(
            "INSERT INTO conversations
                (id, title, title_locked, provider, model, system_prompt, project_id,
                 branched_from_message_id, created_at, updated_at, last_message_at,
                 metadata)
             VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?8, ?9)",
            rusqlite::params![
                new_id,
                format!("{} (branch)", source.title),
                source.provider,
                source.model,
                source.system_prompt,
                source.project_id,
                message_id,
                now,
                // Remember which backend session to continue from, so the
                // copied history is in context and not merely on screen.
                serde_json::json!({ "forkFrom": source.provider_conversation_id }).to_string()
            ],
        )?;

        for (i, m) in history.iter().enumerate() {
            let copy_id = crate::db::models::new_id();
            tx.execute(
                "INSERT INTO messages
                    (id, conversation_id, seq, role, content, thinking, status, model,
                     stop_reason, input_tokens, output_tokens, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                rusqlite::params![
                    copy_id,
                    new_id,
                    i as i64,
                    m.role.as_str(),
                    m.content,
                    m.thinking,
                    m.status.as_str(),
                    m.model,
                    m.stop_reason,
                    m.input_tokens,
                    m.output_tokens,
                    m.created_at,
                    now
                ],
            )?;
            // Attachment rows are duplicated but the blobs are shared: the
            // store is content-addressed, so this costs no extra disk.
            for a in &m.attachments {
                tx.execute(
                    "INSERT INTO attachments
                        (id, conversation_id, message_id, filename, mime_type, size_bytes,
                         kind, storage_path, sha256, text_content, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    rusqlite::params![
                        crate::db::models::new_id(),
                        new_id,
                        copy_id,
                        a.filename,
                        a.mime_type,
                        a.size_bytes,
                        a.kind.as_str(),
                        a.storage_path,
                        a.sha256,
                        a.text_content,
                        a.created_at
                    ],
                )?;
            }
        }

        repo::conversations::get(tx, &new_id)
    })
}
