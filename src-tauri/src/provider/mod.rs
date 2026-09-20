//! Provider abstraction.
//!
//! Only the Claude Code CLI is implemented. The trait exists so that adding a
//! provider later is a new module rather than a refactor of the chat
//! orchestrator — the orchestrator in `chat.rs` speaks only in these types.
//!
//! This app holds no API credential and makes no HTTP request of its own:
//! everything goes through the `claude` binary, under the user's own login.

pub mod claude_code;
pub mod ndjson;
pub mod wire;

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

/// A single turn in the conversation as the provider sees it.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderMessage {
    pub role: &'static str,
    pub content: Vec<ContentBlock>,
}

/// The content shapes we can send. Mirrors the Anthropic block union, but is
/// provider-neutral enough that another backend could map onto it.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Image { source: MediaSource },
    Document { source: MediaSource },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MediaSource {
    Base64 { media_type: String, data: String },
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub system: Option<String>,
    pub messages: Vec<ProviderMessage>,
    pub max_tokens: u32,
    pub temperature: Option<f64>,
    pub stop_sequences: Vec<String>,
}

/// Normalised streaming events. The chat orchestrator translates these into
/// database writes and Tauri events; it never sees provider wire format.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    /// The backend session is up. Carries what it resolved a model alias to,
    /// and the slash commands this installation offers — both only knowable
    /// once a session starts, and both worth showing the user.
    SessionReady {
        session_id: String,
        model: String,
        slash_commands: Vec<String>,
        /// Tools the session reports as available. Expected to be empty for
        /// a chat conversation; anything here means the lockdown has a hole.
        tools: Vec<String>,
    },
    /// Stream accepted; carries the provider's message id and prompt usage.
    Started {
        message_id: String,
        model: String,
        input_tokens: Option<i64>,
        cache_read: Option<i64>,
        cache_write: Option<i64>,
    },
    /// A chunk of visible answer text.
    TextDelta(String),
    /// A chunk of extended-thinking output, shown collapsed and never replayed.
    ThinkingDelta(String),
    /// Terminal success.
    Completed {
        stop_reason: Option<String>,
        output_tokens: Option<i64>,
    },
    /// Terminal failure reported inside the stream body.
    Failed(crate::error::ErrorDetail),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    /// Stable API identifier — what gets sent on the wire and stored in rows.
    pub id: String,
    /// Human label from the provider; never used as a key.
    pub display_name: String,
    pub created_at: Option<String>,
    /// True when the entry came from a built-in fallback list rather than the
    /// live API, so the UI can say so instead of pretending it is authoritative.
    #[serde(default)]
    pub from_fallback: bool,
}

pub type EventStream = Pin<Box<dyn futures_util::Stream<Item = Result<StreamEvent>> + Send>>;

/// The interface the rest of the app programs against.
#[allow(async_fn_in_trait)]
pub trait AIProvider: Send + Sync {
    fn id(&self) -> &'static str;

    /// Open a streaming completion.
    async fn stream_message(&self, req: ChatRequest) -> Result<EventStream>;

    /// Non-streaming completion — used for short utility calls such as
    /// generating a conversation title.
    async fn send_message(&self, req: ChatRequest) -> Result<String>;

    /// Models this credential can actually use.
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;

    /// Cheap credential check for the onboarding "Test connection" button.
    async fn verify_credentials(&self) -> Result<()>;
}
